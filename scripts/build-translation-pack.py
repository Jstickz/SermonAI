"""Build a SermonAI translation pack from API.Bible.

Run once per translation on a developer machine; the app never calls this.
Output is a gzipped JSON Lines file plus a SHA-256, ready either to bundle in
src-tauri/assets (KJV, WEB, ASV per FR-59) or to publish to the Pack CDN
(everything else, per FR-60).

    python scripts/build-translation-pack.py KJV
    python scripts/build-translation-pack.py --all

Each output line is one verse:
    {"b": "GEN", "c": 1, "v": 1, "t": "In the beginning..."}

Book IDs are USFM three-letter codes, which is what the detection regex and
`bible_verses.book_id` use (PRD 14.2).
"""

from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import os
import re
import sys
import time
from pathlib import Path

import requests

API_ROOT = "https://api.scripture.api.bible/v1"

# Resolved against API.Bible on 8 Sept 2026. Names matter: "World English
# Bible" has British, Updated and American-without-Strong's editions, and PRD
# 26.2 means the plain American edition.
TRANSLATIONS = {
    "KJV": ("de4e12af7f28f599-01", "King James Version", "Public domain"),
    "WEB": ("9879dbb7cfe39e4d-01", "World English Bible", "Public domain"),
    "ASV": ("06125adad2d5898a-01", "American Standard Version", "Public domain"),
}

REPO_ROOT = Path(__file__).resolve().parent.parent
OUT_DIR = REPO_ROOT / "src-tauri" / "assets" / "translations"

# The 66-book Protestant canon, in USFM codes.
#
# API.Bible's KJV and ASV also carry the Apocrypha (80 books, 36,820 verses).
# We keep only these 66, which yields exactly the 31,102 verses the PRD counts
# in 15.4 and matches the book-name vocabulary in FR-09. Deuterocanonical
# support would be a translation-catalog decision for M6, not a silent default.
CANON = (
    "GEN EXO LEV NUM DEU JOS JDG RUT 1SA 2SA 1KI 2KI 1CH 2CH EZR NEH EST JOB PSA PRO "
    "ECC SNG ISA JER LAM EZK DAN HOS JOL AMO OBA JON MIC NAM HAB ZEP HAG ZEC MAL "
    "MAT MRK LUK JHN ACT ROM 1CO 2CO GAL EPH PHP COL 1TH 2TH 1TI 2TI TIT PHM HEB "
    "JAS 1PE 2PE 1JN 2JN 3JN JUD REV"
).split()

CANON_SET = frozenset(CANON)

# Sanity target for a complete Protestant Bible.
EXPECTED_VERSES = 31_102

# API.Bible asks for courtesy between calls; a whole Bible is ~1,189 chapters.
REQUEST_PAUSE_SECONDS = 0.05

# API.Bible returns the occasional 502; a whole Bible is ~1,250 requests.
MAX_ATTEMPTS = 6

# Raw chapter responses, so a re-run resumes instead of refetching. Gitignored.
CACHE_DIR = REPO_ROOT / ".cache" / "api-bible"


def api_key() -> str:
    key = os.environ.get("API_BIBLE_KEY")
    if not key:
        env_file = REPO_ROOT / ".env"
        if env_file.exists():
            for line in env_file.read_text(encoding="utf-8").splitlines():
                if line.startswith("API_BIBLE_KEY="):
                    key = line.split("=", 1)[1].strip()
                    break
    if not key:
        sys.exit("API_BIBLE_KEY is not set. Put it in .env or the environment.")
    return key


def get(session: requests.Session, path: str, **params) -> dict:
    """GET one API.Bible endpoint, retrying the failures that just happen.

    A whole Bible is ~1,250 requests and API.Bible returns the occasional 502,
    so a single transient failure must not throw away an hour of fetching.
    """
    delay = 1.0
    last = ""

    for attempt in range(1, MAX_ATTEMPTS + 1):
        try:
            response = session.get(f"{API_ROOT}{path}", params=params, timeout=60)
        except requests.RequestException as exc:
            last = f"{type(exc).__name__}: {exc}"
        else:
            if response.status_code == 200:
                return response.json()["data"]

            last = f"HTTP {response.status_code}: {response.text[:200]}"

            # 4xx other than rate limiting will not fix itself.
            if response.status_code != 429 and response.status_code < 500:
                sys.exit(f"API.Bible returned {last} for {path}")

            if response.status_code == 429:
                delay = max(delay, float(response.headers.get("Retry-After", delay)))

        if attempt < MAX_ATTEMPTS:
            print(f"\n    retry {attempt}/{MAX_ATTEMPTS - 1} in {delay:.0f}s ({path}): {last[:80]}")
            time.sleep(delay)
            delay = min(delay * 2, 30.0)

    sys.exit(f"API.Bible kept failing for {path} after {MAX_ATTEMPTS} attempts: {last}")


def cached_chapter(session: requests.Session, bible_id: str, chapter_id: str) -> dict:
    """Fetch a chapter, reusing a previous run's copy if we already have it.

    The cache is what makes this script resumable: a 502 two thirds of the way
    through Psalms costs one chapter, not the whole Bible.
    """
    cache_file = CACHE_DIR / bible_id / f"{chapter_id}.json"
    if cache_file.exists():
        try:
            return json.loads(cache_file.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            cache_file.unlink()  # truncated by an interrupted write; refetch

    data = get(
        session,
        f"/bibles/{bible_id}/chapters/{chapter_id}",
        **{
            "content-type": "json",
            "include-verse-numbers": "false",
            "include-notes": "false",
            "include-titles": "false",
            "include-chapter-numbers": "false",
        },
    )

    cache_file.parent.mkdir(parents=True, exist_ok=True)
    cache_file.write_text(json.dumps(data), encoding="utf-8")
    return data


def clean(text: str) -> str:
    """Normalise verse text for display and for embedding.

    Deliberately conservative: this text is projected in front of a
    congregation and quoted in summaries, so the only things removed are
    typographic marks that are not part of the verse.

    We do NOT strip leading digits. Verse numbers are already excluded by the
    API request, and a blanket strip would eat the opening of a verse that
    genuinely starts with a numeral.
    """
    # KJV/ASV carry pilcrows as paragraph markers inside the verse text.
    text = text.replace("¶", " ")
    return re.sub(r"\s+", " ", text).strip()


def fetch_translation(session: requests.Session, code: str) -> list[dict]:
    bible_id, name, _ = TRANSLATIONS[code]
    print(f"  {code}: {name} ({bible_id})")

    verses: list[dict] = []
    books = get(session, f"/bibles/{bible_id}/books")

    for book in books:
        book_id = book["id"]  # USFM code, e.g. GEN
        if book_id not in CANON_SET:
            continue  # Apocrypha; see CANON

        chapters = get(session, f"/bibles/{bible_id}/books/{book_id}/chapters")

        for chapter in chapters:
            # API.Bible exposes book introductions as chapter "intro"; skip them.
            if chapter["number"] == "intro":
                continue

            data = cached_chapter(session, bible_id, chapter["id"])

            for verse_number, text in verses_from_chapter(data):
                verses.append(
                    {"b": book_id, "c": int(chapter["number"]), "v": verse_number, "t": text}
                )

            time.sleep(REQUEST_PAUSE_SECONDS)

        print(f"    {book_id}: {len(verses)} verses so far", end="\r", flush=True)

    print(f"    {code}: {len(verses)} verses total          ")
    return verses


def verses_from_chapter(data: dict) -> list[tuple[int, str]]:
    """Walk API.Bible's JSON content tree and collect (verse number, text)."""
    out: dict[int, list[str]] = {}

    def walk(node) -> None:
        if isinstance(node, list):
            for item in node:
                walk(item)
            return
        if not isinstance(node, dict):
            return

        attrs = node.get("attrs") or {}
        verse_id = attrs.get("verseId") or attrs.get("verseOrgIds")
        if node.get("name") == "verse" and verse_id:
            return  # marker node, text lives in the following siblings

        if node.get("type") == "text" and node.get("text"):
            marker = attrs.get("verseId")
            if marker:
                number = int(str(marker).split(".")[-1])
                out.setdefault(number, []).append(node["text"])

        walk(node.get("items", []))
        walk(node.get("content", []))

    walk(data.get("content", []))
    return [(n, clean(" ".join(parts))) for n, parts in sorted(out.items()) if clean(" ".join(parts))]


def write_pack(code: str, verses: list[dict]) -> None:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    _, name, licence = TRANSLATIONS[code]

    header = {
        "code": code,
        "name": name,
        "language": "eng",
        "license": licence,
        "source": "api_bible",
        "verse_count": len(verses),
    }

    path = OUT_DIR / f"{code.lower()}.jsonl.gz"
    with gzip.open(path, "wt", encoding="utf-8", compresslevel=9) as handle:
        handle.write(json.dumps(header, separators=(",", ":")) + "\n")
        for verse in verses:
            handle.write(json.dumps(verse, separators=(",", ":"), ensure_ascii=False) + "\n")

    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    (OUT_DIR / f"{code.lower()}.jsonl.gz.sha256").write_text(digest + "\n", encoding="utf-8")

    size_mb = path.stat().st_size / 1024 / 1024
    print(f"    wrote {path.name}  {size_mb:.2f} MB  sha256 {digest[:16]}...")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("codes", nargs="*", help="translation codes, e.g. KJV WEB ASV")
    parser.add_argument("--all", action="store_true", help="build every bundled translation")
    args = parser.parse_args()

    codes = list(TRANSLATIONS) if args.all else [c.upper() for c in args.codes]
    if not codes:
        parser.error("give at least one translation code, or --all")

    unknown = [c for c in codes if c not in TRANSLATIONS]
    if unknown:
        sys.exit(f"unknown translation(s): {', '.join(unknown)}")

    session = requests.Session()
    session.headers["api-key"] = api_key()

    print(f"Building {len(codes)} translation pack(s) into {OUT_DIR}")
    for code in codes:
        verses = fetch_translation(session, code)

        # A pack with holes is worse than no pack: nobody finds out until a
        # verse fails to appear mid-service. Fail on anything structurally
        # wrong, but only warn on small count differences, which are normal —
        # translations disagree about bracketed verses such as Matthew 17:21.
        missing = sorted(CANON_SET - {v["b"] for v in verses})
        if missing or len(verses) < 30_000:
            sys.exit(
                f"{code}: got {len(verses)} verses across {66 - len(missing)} of 66 books. "
                f"Missing: {missing or 'none'}. Refusing to write an incomplete pack."
            )

        if len(verses) != EXPECTED_VERSES:
            delta = len(verses) - EXPECTED_VERSES
            print(f"    note: {len(verses)} verses ({delta:+d} vs KJV) — expected for this translation")

        write_pack(code, verses)


if __name__ == "__main__":
    main()
