"""Build the bundled verse embedding index and the on-device encoder assets.

Run once on a developer machine; the outputs are committed and ship inside the
binary (FR-15, FR-59). The app never runs this script.

    python scripts/build-verse-index.py

Writes into src-tauri/assets/:
    verse-index.bin        int8 embeddings of every verse + its reference
    encoder/encoder.bin    the static token embedding matrix, int8
    encoder/tokenizer.json the matching tokenizer

Why one model for both sides
----------------------------
The index and the runtime query must be embedded by the SAME model. Vectors
from two different models are not comparable, so a cosine score between them is
meaningless. PRD 15.4 originally paired an OpenAI-built index with a different
on-device query encoder; this script implements the corrected design and the
PRD amendment records it.

Static embeddings (model2vec) mean the runtime side is a token lookup plus a
mean — no transformer, no ONNX runtime, no GPU. That is what keeps offline
semantic detection inside the 40 MB installer.
"""

from __future__ import annotations

import gzip
import hashlib
import json
import os
import struct
import sys
from pathlib import Path

import numpy as np
from model2vec import StaticModel

MODEL_ID = "minishlab/potion-base-8M"

REPO_ROOT = Path(__file__).resolve().parent.parent
ASSETS = REPO_ROOT / "src-tauri" / "assets"
TRANSLATIONS = ASSETS / "translations"
ENCODER_DIR = ASSETS / "encoder"

# Built from KJV today, though the measurements point at WEB.
#
# Detection only needs one semantic space: a verse matched here is displayed in
# whichever translation the operator chose. Preachers paraphrase in modern
# English and a static bag-of-tokens model cannot bridge archaic wording, so on
# a set of 8 modern paraphrases a WEB index beat a KJV one (top-5 4 vs 3, top-3
# 4 vs 2).
#
# We ship the KJV index anyway, because correctness beats recall: the WEB pack
# was found to be missing 122 verses including Jeremiah 29:11, and API.Bible's
# monthly quota is exhausted so it cannot be rebuilt yet. KJV is verified
# complete — 31,102 verses, every canary present, no thin chapters.
#
# Switch back to WEB once its pack is rebuilt and passes check_complete.
INDEX_TRANSLATION = os.environ.get("INDEX_TRANSLATION", "KJV")

INDEX_MAGIC = b"SAIVIDX1"
ENCODER_MAGIC = b"SAIENC01"

# Same 66-book order as build-translation-pack.py; the index stores a book as
# one byte of index into this list.
CANON = (
    "GEN EXO LEV NUM DEU JOS JDG RUT 1SA 2SA 1KI 2KI 1CH 2CH EZR NEH EST JOB PSA PRO "
    "ECC SNG ISA JER LAM EZK DAN HOS JOL AMO OBA JON MIC NAM HAB ZEP HAG ZEC MAL "
    "MAT MRK LUK JHN ACT ROM 1CO 2CO GAL EPH PHP COL 1TH 2TH 1TI 2TI TIT PHM HEB "
    "JAS 1PE 2PE 1JN 2JN 3JN JUD REV"
).split()

BOOK_INDEX = {code: i for i, code in enumerate(CANON)}


def load_verses(code: str) -> list[dict]:
    path = TRANSLATIONS / f"{code.lower()}.jsonl.gz"
    if not path.exists():
        sys.exit(f"{path} is missing. Run build-translation-pack.py {code} first.")

    verses = []
    with gzip.open(path, "rt", encoding="utf-8") as handle:
        handle.readline()  # header
        for line in handle:
            verses.append(json.loads(line))
    return verses


def quantize_unit(vectors: np.ndarray) -> np.ndarray:
    """L2-normalise, then scale to int8.

    Because every row is unit length first, the dot product of two quantized
    rows is proportional to their cosine similarity — so the runtime search is
    a plain integer dot product with no per-row scale to carry around.
    """
    norms = np.linalg.norm(vectors, axis=1, keepdims=True)
    norms[norms == 0] = 1.0
    unit = vectors / norms
    return np.clip(np.rint(unit * 127.0), -127, 127).astype(np.int8)


def write_verse_index(verses: list[dict], vectors: np.ndarray, dims: int) -> Path:
    path = ASSETS / "verse-index.bin"
    path.parent.mkdir(parents=True, exist_ok=True)

    with path.open("wb") as out:
        out.write(INDEX_MAGIC)
        out.write(struct.pack("<III", 1, dims, len(verses)))
        out.write(struct.pack("<32s", MODEL_ID.encode("utf-8")[:32]))

        # Reference table: book index, chapter, verse. Chapter tops out at 150
        # and verse at 176, so a byte each is enough; the fourth byte keeps
        # rows 4-aligned for cheap slicing on the Rust side.
        for verse in verses:
            out.write(bytes((BOOK_INDEX[verse["b"]], verse["c"], verse["v"], 0)))

        out.write(vectors.tobytes())

    return path


def write_encoder(model: StaticModel) -> Path:
    """Export the token embedding matrix and tokenizer for the Rust runtime."""
    ENCODER_DIR.mkdir(parents=True, exist_ok=True)

    embedding = np.asarray(model.embedding, dtype=np.float32)
    vocab, dims = embedding.shape

    # Per-row scales: token vectors vary a lot in magnitude, and a single
    # global scale would flatten the rare, information-dense tokens that carry
    # most of the meaning in a short spoken phrase.
    scales = np.abs(embedding).max(axis=1)
    scales[scales == 0] = 1.0
    quantized = np.clip(np.rint(embedding / scales[:, None] * 127.0), -127, 127).astype(np.int8)

    path = ENCODER_DIR / "encoder.bin"
    with path.open("wb") as out:
        out.write(ENCODER_MAGIC)
        out.write(struct.pack("<III", 1, dims, vocab))
        out.write(scales.astype(np.float32).tobytes())
        out.write(quantized.tobytes())

    model.tokenizer.save(str(ENCODER_DIR / "tokenizer.json"))
    return path


def dequantize_encoder(path: Path) -> np.ndarray:
    """Read back what Rust will read, so validation measures the shipped thing."""
    data = path.read_bytes()
    assert data[:8] == ENCODER_MAGIC
    _version, dims, vocab = struct.unpack_from("<III", data, 8)
    offset = 8 + 12
    scales = np.frombuffer(data, dtype=np.float32, count=vocab, offset=offset)
    offset += vocab * 4
    quantized = np.frombuffer(data, dtype=np.int8, count=vocab * dims, offset=offset)
    return quantized.reshape(vocab, dims).astype(np.float32) * scales[:, None] / 127.0


def validate(model: StaticModel, verses: list[dict], index: np.ndarray, encoder_path: Path) -> None:
    """Check quantization did not wreck retrieval.

    Quantizing twice — the model and the index — could quietly degrade recall.
    This compares the shipped int8 pipeline against full float32 on a sample of
    verses, using each verse's own text as the query.
    """
    print("\nValidating quantized retrieval against float32...")

    rng = np.random.default_rng(20260908)
    sample = rng.choice(len(verses), size=300, replace=False)
    queries = [verses[i]["t"] for i in sample]

    reference = model.encode(queries).astype(np.float32)
    reference /= np.linalg.norm(reference, axis=1, keepdims=True)

    exact = index.astype(np.float32)
    exact /= np.linalg.norm(exact, axis=1, keepdims=True)

    hits_top1 = 0
    hits_top5 = 0
    for row, verse_row in zip(reference, sample):
        scores = exact @ row
        top5 = np.argpartition(-scores, 5)[:5]
        top5 = top5[np.argsort(-scores[top5])]
        if top5[0] == verse_row:
            hits_top1 += 1
        if verse_row in top5:
            hits_top5 += 1

    n = len(sample)
    print(f"  self-retrieval top-1: {hits_top1}/{n} ({hits_top1 / n:.1%})")
    print(f"  self-retrieval top-5: {hits_top5}/{n} ({hits_top5 / n:.1%})")

    if hits_top5 / n < 0.95:
        sys.exit("Quantized index fails to retrieve its own verses. Do not ship this.")

    # The encoder round-trip must also survive quantization.
    restored = dequantize_encoder(encoder_path)
    original = np.asarray(model.embedding, dtype=np.float32)
    cos = (restored * original).sum(1) / (
        np.linalg.norm(restored, axis=1) * np.linalg.norm(original, axis=1) + 1e-9
    )
    print(f"  encoder token cosine after int8: mean {cos.mean():.4f}, min {cos.min():.4f}")


def main() -> None:
    print(f"Loading {MODEL_ID}")
    model = StaticModel.from_pretrained(MODEL_ID)
    dims = int(np.asarray(model.embedding).shape[1])

    verses = load_verses(INDEX_TRANSLATION)
    print(f"Embedding {len(verses):,} {INDEX_TRANSLATION} verses ({dims} dims)")

    vectors = model.encode([v["t"] for v in verses], show_progress_bar=True).astype(np.float32)
    quantized = quantize_unit(vectors)

    index_path = write_verse_index(verses, quantized, dims)
    encoder_path = write_encoder(model)

    validate(model, verses, quantized, encoder_path)

    for path in (index_path, encoder_path, ENCODER_DIR / "tokenizer.json"):
        size_mb = path.stat().st_size / 1024 / 1024
        digest = hashlib.sha256(path.read_bytes()).hexdigest()[:16]
        print(f"  {path.relative_to(REPO_ROOT)}  {size_mb:.2f} MB  sha256 {digest}...")


if __name__ == "__main__":
    main()
