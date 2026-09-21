"""Build a SermonAI translation pack from YouVersion Platform.

Run once per version on a developer machine; the app never calls this.

    python scripts/build-translation-pack.py 111            # by version ID
    python scripts/build-translation-pack.py 111 --out NIV  # name the pack

Output is a gzipped JSON Lines file plus a manifest and a SHA-256, ready
either to bundle in src-tauri/assets or to publish to the Pack CDN (FR-59,
FR-60).

Attribution
-----------
YouVersion requires the copyright string to be displayed wherever the text is
shown. It is fetched with the version metadata and written into the pack
manifest, and the app refuses to display text whose attribution is missing. A
pack without one is never written.

Response shapes
---------------
Verified against the live API on 21 September 2026:

* ``/bibles?language_ranges[]=eng`` is enveloped in ``data`` and paginated,
  and ``copyright`` is null for every version there.
* ``/bibles/{id}`` is a bare object and is the only place ``copyright`` is
  populated.
* ``/bibles/{id}/passages/{id}`` is a bare object of id/content/reference.
  Without ``?format=html`` the content is one unbroken string with no verse
  boundaries, so this script always asks for HTML.
"""

from __future__ import annotations

import argparse
import gzip
import hashlib
import html as html_lib
import json
import os
import re
import sys
import time
from pathlib import Path

import requests

API_ROOT = "https://api.youversion.com/v1"
APP_KEY_ENV = "YVP_APP_KEY"
PORTAL_URL = "https://platform.youversion.com"

REPO_ROOT = Path(__file__).resolve().parent.parent
OUT_DIR = REPO_ROOT / "src-tauri" / "assets" / "translations"
CACHE_DIR = REPO_ROOT / ".cache" / "youversion"

# Well inside any published throttle, and polite to a licensed partner API.
REQUESTS_PER_SECOND = 5.0
MIN_INTERVAL = 1.0 / REQUESTS_PER_SECOND

MAX_ATTEMPTS = 6

# Chapter counts for the 66-book Protestant canon, in USFM order. The passage
# endpoint addresses chapters directly, so the pack builder enumerates them
# rather than asking for a book list.
CHAPTERS: dict[str, int] = {
    "GEN": 50, "EXO": 40, "LEV": 27, "NUM": 36, "DEU": 34, "JOS": 24, "JDG": 21,
    "RUT": 4, "1SA": 31, "2SA": 24, "1KI": 22, "2KI": 25, "1CH": 29, "2CH": 36,
    "EZR": 10, "NEH": 13, "EST": 10, "JOB": 42, "PSA": 150, "PRO": 31, "ECC": 12,
    "SNG": 8, "ISA": 66, "JER": 52, "LAM": 5, "EZK": 48, "DAN": 12, "HOS": 14,
    "JOL": 3, "AMO": 9, "OBA": 1, "JON": 4, "MIC": 7, "NAM": 3, "HAB": 3,
    "ZEP": 3, "HAG": 2, "ZEC": 14, "MAL": 4, "MAT": 28, "MRK": 16, "LUK": 24,
    "JHN": 21, "ACT": 28, "ROM": 16, "1CO": 16, "2CO": 13, "GAL": 6, "EPH": 6,
    "PHP": 4, "COL": 4, "1TH": 5, "2TH": 3, "1TI": 6, "2TI": 4, "TIT": 3,
    "PHM": 1, "HEB": 13, "JAS": 5, "1PE": 5, "2PE": 3, "1JN": 5, "2JN": 1,
    "3JN": 1, "JUD": 1, "REV": 22,
}

EXPECTED_VERSES = 31_102

# Verses spread across the shapes that have broken parsers before: poetry,
# letters, Aramaic sections and plain prose.
CANARY_VERSES = [
    ("GEN", 1, 1), ("PSA", 23, 1), ("PSA", 119, 105), ("PRO", 3, 5),
    ("ISA", 40, 31), ("JER", 29, 11), ("DAN", 4, 8), ("EZR", 4, 11),
    ("MAT", 6, 33), ("JHN", 3, 16), ("ROM", 8, 28), ("PHP", 4, 13),
    ("REV", 21, 4),
]


def app_key() -> str:
    key = os.environ.get(APP_KEY_ENV, "").strip()
    if not key:
        env_file = REPO_ROOT / ".env"
        if env_file.exists():
            for line in env_file.read_text(encoding="utf-8").splitlines():
                if line.startswith(f"{APP_KEY_ENV}="):
                    key = line.split("=", 1)[1].strip()
                    break
    if not key:
        sys.exit(
            f"{APP_KEY_ENV} is not set. Register an app at {PORTAL_URL}, then put the key "
            f"in .env or the environment."
        )
    return key


class Client:
    """Rate-limited YouVersion client with retries and an on-disk cache."""

    def __init__(self, key: str) -> None:
        self.session = requests.Session()
        self.session.headers["X-YVP-App-Key"] = key
        self.session.headers["Accept"] = "application/json"
        self._last_request = 0.0

    def _wait(self) -> None:
        elapsed = time.monotonic() - self._last_request
        if elapsed < MIN_INTERVAL:
            time.sleep(MIN_INTERVAL - elapsed)
        self._last_request = time.monotonic()

    def get(self, path: str) -> dict:
        delay = 1.0
        last = ""

        for attempt in range(1, MAX_ATTEMPTS + 1):
            self._wait()
            try:
                response = self.session.get(f"{API_ROOT}{path}", timeout=60)
            except requests.RequestException as exc:
                last = f"{type(exc).__name__}: {exc}"
            else:
                if response.status_code == 200:
                    return unwrap(response.json())

                last = f"HTTP {response.status_code}: {response.text[:200]}"

                if response.status_code in (401, 403):
                    sys.exit(
                        f"YouVersion refused {path}: {last}\n"
                        f"The app key may be wrong, or this version's licence may not be "
                        f"approved yet at {PORTAL_URL}."
                    )
                if response.status_code == 404:
                    sys.exit(f"YouVersion has no {path}.")
                if response.status_code == 429:
                    delay = max(delay, float(response.headers.get("Retry-After", delay)))

            if attempt < MAX_ATTEMPTS:
                print(f"\n    retry {attempt} in {delay:.0f}s ({path}): {last[:80]}")
                time.sleep(delay)
                delay = min(delay * 2, 30.0)

        sys.exit(f"YouVersion kept failing for {path}: {last}")

    def chapter(self, version_id: int, book: str, chapter: int) -> dict:
        """Fetch one chapter as HTML, reusing a cached copy when present.

        format=html is not optional: the plain response has no verse markers,
        so a chapter cannot be split into the per-verse rows a pack stores.
        """
        cache_file = CACHE_DIR / str(version_id) / f"{book}.{chapter}.json"
        if cache_file.exists():
            try:
                return json.loads(cache_file.read_text(encoding="utf-8"))
            except json.JSONDecodeError:
                cache_file.unlink()

        data = self.get(f"/bibles/{version_id}/passages/{book}.{chapter}?format=html")
        cache_file.parent.mkdir(parents=True, exist_ok=True)
        cache_file.write_text(json.dumps(data), encoding="utf-8")
        return data


def unwrap(payload):
    """Accept a bare object or one wrapped in data/passage/bible/version."""
    if isinstance(payload, dict):
        for key in ("data", "passage", "bible", "version", "result", "response"):
            inner = payload.get(key)
            if isinstance(inner, (dict, list)):
                return inner
    return payload


def field(record: dict, *names: str, default: str = "") -> str:
    """First present, non-empty field among several possible spellings."""
    for name in names:
        value = record.get(name)
        if isinstance(value, str) and value.strip():
            return value.strip()
    return default


# YouVersion marks a verse with <span class="yv-v" v="16"></span> and prints
# the number separately in <span class="yv-vlbl">16</span>. Keep this in step
# with src-tauri/src/bible/sanitize.rs, which parses the same markup.
VERSE_SPAN = re.compile(r'<span[^>]*class="yv-v"[^>]*\sv="(?P<num>\d+)"[^>]*>', re.IGNORECASE)
TAG = re.compile(r"<[^>]+>")
# The printed verse label, footnotes and cross references are not scripture.
DROPPED_BLOCK = re.compile(
    r'<span[^>]*class="(?:yv-vlbl|note|footnote|crossref|label)"[^>]*>.*?</span>'
    r"|<(note|sup|script|style)\b.*?</\1>",
    re.IGNORECASE | re.DOTALL,
)


def sanitize(markup: str) -> list[tuple[int, str]]:
    """Split chapter HTML into (verse number, plain text).

    Mirrors src-tauri/src/bible/sanitize.rs: footnotes and cross references are
    dropped rather than shown, because they are not scripture and the projector
    must never display them.
    """
    cleaned = DROPPED_BLOCK.sub(" ", markup)

    verses: list[tuple[int, str]] = []
    matches = list(VERSE_SPAN.finditer(cleaned))

    for index, match in enumerate(matches):
        number = match.group("num")
        if number is None:
            continue

        end = matches[index + 1].start() if index + 1 < len(matches) else len(cleaned)
        body = cleaned[match.end() : end]
        text = html_lib.unescape(TAG.sub(" ", body))
        text = re.sub(r"\s+", " ", text).replace("¶", " ").strip()

        if text:
            verses.append((int(number), text))

    # Merge split verses (poetry often opens a new span per line).
    merged: dict[int, list[str]] = {}
    for number, text in verses:
        merged.setdefault(number, []).append(text)

    return [(n, " ".join(parts).strip()) for n, parts in sorted(merged.items())]


def check_complete(verses: list[dict], version_id: int) -> None:
    present = {(v["b"], v["c"], v["v"]) for v in verses}

    missing_books = sorted(set(CHAPTERS) - {v["b"] for v in verses})
    if missing_books:
        sys.exit(f"version {version_id}: missing books {missing_books}. Refusing to write.")

    if len(verses) < 30_800:
        sys.exit(
            f"version {version_id}: only {len(verses)} verses, expected about {EXPECTED_VERSES}. "
            "Refusing to write an incomplete pack."
        )

    absent = [f"{b} {c}:{v}" for b, c, v in CANARY_VERSES if (b, c, v) not in present]
    if absent:
        sys.exit(
            f"version {version_id}: these verses are missing: {', '.join(absent)}. "
            "The verse-marker parsing is probably wrong for some paragraph style."
        )

    chapters: dict[tuple[str, int], set[int]] = {}
    for verse in verses:
        chapters.setdefault((verse["b"], verse["c"]), set()).add(verse["v"])

    for (book, chapter), numbers in sorted(chapters.items()):
        if len(numbers) < max(numbers) * 0.8:
            sys.exit(
                f"version {version_id}: {book} {chapter} has {len(numbers)} verses but numbering "
                f"reaches {max(numbers)}. Verses were dropped; refusing to write."
            )


def build(version_id: int, out_name: str | None) -> None:
    client = Client(app_key())

    meta = client.get(f"/bibles/{version_id}")
    name = field(meta, "localized_title", "title", default=f"Version {version_id}")
    short = field(meta, "localized_abbreviation", "abbreviation", default=str(version_id))
    language = field(meta, "language_tag", default="und")
    attribution = field(meta, "copyright")

    # Attribution is a licensing obligation, so there is no pack without one.
    if not attribution:
        sys.exit(
            f"version {version_id} ({short}) returned no copyright string. YouVersion requires "
            "attribution wherever the text is displayed, so this pack will not be built."
        )

    code = (out_name or short).upper()
    print(f"Building {code} — {name} ({language}), version {version_id}")
    print(f"  attribution: {attribution[:80]}")

    # The version lists the books it contains, which may include the
    # Apocrypha (80 for WEBUS). Packs carry the 66-book Protestant canon only.
    available = {b for b in meta.get("books", []) if b in CHAPTERS} or set(CHAPTERS)
    skipped = sorted(set(meta.get("books", [])) - set(CHAPTERS))
    if skipped:
        print(f"  skipping {len(skipped)} non-canonical books: {', '.join(skipped[:6])}...")

    verses: list[dict] = []
    for book, chapter_count in CHAPTERS.items():
        if book not in available:
            sys.exit(f"version {version_id} does not contain {book}; it cannot make a complete pack.")
        for chapter in range(1, chapter_count + 1):
            payload = client.chapter(version_id, book, chapter)
            markup = field(payload, "content")
            for number, text in sanitize(markup):
                verses.append({"b": book, "c": chapter, "v": number, "t": text})
        print(f"    {book}: {len(verses)} verses so far", end="\r", flush=True)

    print(f"    {code}: {len(verses)} verses total          ")
    check_complete(verses, version_id)

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    path = OUT_DIR / f"{code.lower()}.jsonl.gz"

    header = {
        "code": code,
        "name": name,
        "short_name": short,
        "language": language,
        "source": "youversion",
        "yvp_version_id": version_id,
        "attribution": attribution,
        "verse_count": len(verses),
    }

    with gzip.open(path, "wt", encoding="utf-8", compresslevel=9) as handle:
        handle.write(json.dumps(header, separators=(",", ":"), ensure_ascii=False) + "\n")
        for verse in verses:
            handle.write(json.dumps(verse, separators=(",", ":"), ensure_ascii=False) + "\n")

    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    (OUT_DIR / f"{code.lower()}.jsonl.gz.sha256").write_text(digest + "\n", encoding="utf-8")

    manifest = dict(header, sha256=digest, size_bytes=path.stat().st_size)
    (OUT_DIR / f"{code.lower()}.manifest.json").write_text(
        json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )

    print(f"    wrote {path.name}  {path.stat().st_size / 1024 / 1024:.2f} MB  sha256 {digest[:16]}...")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("version_id", type=int, help="numeric YouVersion version ID")
    parser.add_argument("--out", help="pack code to write, defaults to the version's short name")
    args = parser.parse_args()

    build(args.version_id, args.out)


if __name__ == "__main__":
    main()
