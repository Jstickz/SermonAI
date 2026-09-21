"""Build a KJV translation pack from a public-domain source, not YouVersion.

This is a **fallback**, staged but not shipped. It exists because the KJV is
the translation most of our target churches preach from, and it is not
currently licensed to our YouVersion app key: the only King James in their
1,485-version catalog is a Thai one.

    python scripts/build-kjv-fallback.py

Output goes to ``staging/translations/`` — deliberately outside
``src-tauri/assets``, which is what the Tauri bundle globs. Nothing here ships
until someone moves it, which is the point.

Source
------
``https://bible.helloao.org`` (the Free Use Bible API), serving ebible.org's
``eng_kjv``. Chosen over wldeh/bible-api, whose chapter files duplicate every
verse — Genesis 1 comes back with 62 verses instead of 31, and a naive import
would produce a 62,204-verse Bible.

The underlying text is the 1769 Blayney revision, the same edition Project
Gutenberg distributes as eBook #10 and #30. Gutenberg is recorded in the
manifest as a provenance note, per the request that it be credited.

Licensing note
--------------
The KJV is public domain in the United States. In the United Kingdom it rests
under perpetual Crown copyright, administered by Cambridge University Press
under letters patent, which permits reproduction of brief passages but is not
the same freedom. PRD §4.2 lists the UK among target markets, so this is worth
a deliberate decision before the pack is enabled in production rather than an
assumption.
"""

from __future__ import annotations

import gzip
import hashlib
import json
import sys
import time
from pathlib import Path

import requests

API_ROOT = "https://bible.helloao.org/api"
TRANSLATION_ID = "eng_kjv"

REPO_ROOT = Path(__file__).resolve().parent.parent

# Deliberately NOT src-tauri/assets: that path is bundled into the installer by
# the resources glob, and this pack is staged, not enabled.
OUT_DIR = REPO_ROOT / "staging" / "translations"
CACHE_DIR = REPO_ROOT / ".cache" / "helloao"

REQUESTS_PER_SECOND = 5.0
MIN_INTERVAL = 1.0 / REQUESTS_PER_SECOND
MAX_ATTEMPTS = 5

EXPECTED_VERSES = 31_102

# The 66-book Protestant canon in USFM codes; shared with
# build-translation-pack.py so the two packs are interchangeable.
CANON = (
    "GEN EXO LEV NUM DEU JOS JDG RUT 1SA 2SA 1KI 2KI 1CH 2CH EZR NEH EST JOB PSA PRO "
    "ECC SNG ISA JER LAM EZK DAN HOS JOL AMO OBA JON MIC NAM HAB ZEP HAG ZEC MAL "
    "MAT MRK LUK JHN ACT ROM 1CO 2CO GAL EPH PHP COL 1TH 2TH 1TI 2TI TIT PHM HEB "
    "JAS 1PE 2PE 1JN 2JN 3JN JUD REV"
).split()
CANON_SET = frozenset(CANON)

CANARY_VERSES = [
    ("GEN", 1, 1), ("PSA", 23, 1), ("PSA", 119, 105), ("PRO", 3, 5),
    ("ISA", 40, 31), ("JER", 29, 11), ("DAN", 4, 8), ("EZR", 4, 11),
    ("MAT", 6, 33), ("JHN", 3, 16), ("ROM", 8, 28), ("PHP", 4, 13),
    ("REV", 21, 4),
]


class Client:
    def __init__(self) -> None:
        self.session = requests.Session()
        self.session.headers["Accept"] = "application/json"
        self._last = 0.0

    def get(self, path: str) -> dict:
        delay = 1.0
        last = ""

        for attempt in range(1, MAX_ATTEMPTS + 1):
            elapsed = time.monotonic() - self._last
            if elapsed < MIN_INTERVAL:
                time.sleep(MIN_INTERVAL - elapsed)
            self._last = time.monotonic()

            try:
                response = self.session.get(f"{API_ROOT}{path}", timeout=60)
            except requests.RequestException as exc:
                last = f"{type(exc).__name__}: {exc}"
            else:
                if response.status_code == 200:
                    return response.json()
                last = f"HTTP {response.status_code}"
                if response.status_code == 404:
                    sys.exit(f"{API_ROOT}{path} does not exist.")

            if attempt < MAX_ATTEMPTS:
                print(f"\n    retry {attempt} in {delay:.0f}s ({path}): {last}")
                time.sleep(delay)
                delay = min(delay * 2, 20.0)

        sys.exit(f"gave up on {path}: {last}")

    def chapter(self, book: str, chapter: int) -> dict:
        cache_file = CACHE_DIR / TRANSLATION_ID / f"{book}.{chapter}.json"
        if cache_file.exists():
            try:
                return json.loads(cache_file.read_text(encoding="utf-8"))
            except json.JSONDecodeError:
                cache_file.unlink()

        data = self.get(f"/{TRANSLATION_ID}/{book}/{chapter}.json")
        cache_file.parent.mkdir(parents=True, exist_ok=True)
        cache_file.write_text(json.dumps(data), encoding="utf-8")
        return data


def verses_from_chapter(payload: dict) -> list[tuple[int, str]]:
    """Pull (verse number, text) out of a helloao chapter.

    A verse's content is a list mixing text fragments with markup objects:
    ``{"text": ..., "poem": 1}`` carries scripture, while ``{"noteId": ...}``
    and ``{"lineBreak": true}`` do not. Only text is kept — a footnote marker
    on a projector is noise, and a note body is not scripture.
    """
    out: list[tuple[int, str]] = []

    for node in payload.get("chapter", {}).get("content", []):
        if not isinstance(node, dict) or node.get("type") != "verse":
            continue  # headings and Hebrew subtitles belong to no verse

        number = node.get("number")
        if not isinstance(number, int):
            continue

        parts: list[str] = []
        for item in node.get("content", []):
            if isinstance(item, str):
                parts.append(item)
            elif isinstance(item, dict) and isinstance(item.get("text"), str):
                parts.append(item["text"])

        # Pilcrows are paragraph marks in the KJV's typesetting, not scripture.
        # build-translation-pack.py strips them too; the two packs must look
        # identical on the projector.
        text = " ".join(" ".join(parts).replace("¶", " ").split())
        if text:
            out.append((number, text))

    return out


def check_complete(verses: list[dict], per_book_expected: dict[str, int]) -> None:
    """Refuse to write a pack with holes in it.

    Same rules as build-translation-pack.py, plus a per-book cross-check
    against the count the source itself reports.
    """
    present = {(v["b"], v["c"], v["v"]) for v in verses}

    missing_books = sorted(CANON_SET - {v["b"] for v in verses})
    if missing_books:
        sys.exit(f"missing books {missing_books}. Refusing to write.")

    if len(verses) < 30_800:
        sys.exit(f"only {len(verses)} verses, expected about {EXPECTED_VERSES}. Refusing to write.")

    absent = [f"{b} {c}:{v}" for b, c, v in CANARY_VERSES if (b, c, v) not in present]
    if absent:
        sys.exit(f"these verses are missing: {', '.join(absent)}. The parser is wrong somewhere.")

    counted: dict[str, int] = {}
    chapters: dict[tuple[str, int], set[int]] = {}
    for verse in verses:
        counted[verse["b"]] = counted.get(verse["b"], 0) + 1
        chapters.setdefault((verse["b"], verse["c"]), set()).add(verse["v"])

    for (book, chapter), numbers in sorted(chapters.items()):
        if len(numbers) < max(numbers) * 0.8:
            sys.exit(
                f"{book} {chapter} has {len(numbers)} verses but numbering reaches {max(numbers)}. "
                "Verses were dropped; refusing to write."
            )

    # The source publishes its own per-book verse counts, so disagreement means
    # our parsing lost something rather than the text differing.
    for book, expected in per_book_expected.items():
        got = counted.get(book, 0)
        if expected and abs(got - expected) > 0:
            sys.exit(
                f"{book}: parsed {got} verses but the source reports {expected}. "
                "Refusing to write a pack that disagrees with its source."
            )


def main() -> None:
    client = Client()

    meta = client.get(f"/{TRANSLATION_ID}/books.json")
    translation = meta["translation"]
    books = [b for b in meta["books"] if b["id"] in CANON_SET]

    print(f"Building KJV fallback from {translation['name']} ({translation['id']})")
    print(f"  source: {translation.get('website')}")
    print(f"  books: {len(books)} of 66, chapters: {sum(b['numberOfChapters'] for b in books)}")

    verses: list[dict] = []
    per_book_expected: dict[str, int] = {}

    for book in books:
        code = book["id"]
        per_book_expected[code] = book.get("totalNumberOfVerses", 0)

        for chapter in range(1, book["numberOfChapters"] + 1):
            payload = client.chapter(code, chapter)
            for number, text in verses_from_chapter(payload):
                verses.append({"b": code, "c": chapter, "v": number, "t": text})

        print(f"    {code}: {len(verses)} verses so far", end="\r", flush=True)

    print(f"    KJV: {len(verses)} verses total          ")
    check_complete(verses, per_book_expected)

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    path = OUT_DIR / "kjv.jsonl.gz"

    attribution = (
        "King James Version (Authorized Version), 1769 Blayney revision. "
        "Public domain in the United States; Crown copyright in the United Kingdom, "
        "administered by Cambridge University Press."
    )

    header = {
        "code": "KJV",
        "name": "King James Version",
        "short_name": "KJV",
        "language": "eng",
        "source": "helloao",
        "yvp_version_id": None,
        "attribution": attribution,
        "verse_count": len(verses),
    }

    with gzip.open(path, "wt", encoding="utf-8", compresslevel=9) as handle:
        handle.write(json.dumps(header, separators=(",", ":"), ensure_ascii=False) + "\n")
        for verse in verses:
            handle.write(json.dumps(verse, separators=(",", ":"), ensure_ascii=False) + "\n")

    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    (OUT_DIR / "kjv.jsonl.gz.sha256").write_text(digest + "\n", encoding="utf-8")

    manifest = dict(
        header,
        sha256=digest,
        size_bytes=path.stat().st_size,
        enabled=False,
        source_detail={
            "api": "https://bible.helloao.org (Free Use Bible API)",
            "text_source": translation.get("website"),
            "license_url": translation.get("licenseUrl"),
            "provenance_note": (
                "The 1769 Blayney revision, the same edition Project Gutenberg distributes "
                "as eBook #10 (https://www.gutenberg.org/ebooks/10) and #30. ebible.org's "
                "eng_kjv is used here because it is versified and machine-readable; the "
                "Gutenberg edition is plain prose and would need re-versifying."
            ),
            "uk_crown_copyright": (
                "Public domain in the US. In the UK the KJV is under perpetual Crown "
                "copyright administered by Cambridge University Press under letters patent. "
                "PRD §4.2 lists the UK as a target market, so confirm before enabling."
            ),
        },
    )
    (OUT_DIR / "kjv.manifest.json").write_text(
        json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )

    print(f"    wrote {path.relative_to(REPO_ROOT)}  {path.stat().st_size / 1024 / 1024:.2f} MB")
    print(f"    sha256 {digest[:16]}...")
    print("\n  Staged, not enabled. To ship it, move the three files into")
    print("  src-tauri/assets/translations/ and set enabled=true in the manifest.")


if __name__ == "__main__":
    main()
