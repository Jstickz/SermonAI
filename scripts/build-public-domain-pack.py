"""Build a public-domain translation pack from ebible.org, not YouVersion.

FR-59 bundles KJV, WEB and ASV. Neither KJV nor ASV can come from YouVersion:
KJV is not licensed to our app key (the only King James among their 1,485
versions is Thai), and ASV returns no copyright string, which PRD 15.2 forbids
displaying. Both are public domain, so this builds them from ebible.org
instead, where YouVersion's terms do not apply.

    python scripts/build-public-domain-pack.py KJV
    python scripts/build-public-domain-pack.py ASV

Output goes straight into ``src-tauri/assets/translations`` and ships in the
installer, because public-domain text is ours to redistribute. Licensed
translations are never built this way: they are fetched through the API and
cached (PRD 15.2).

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

# Public-domain translations we bundle, with the attribution stored alongside
# their text.
#
# These texts are public domain, so no publisher requires a notice. We store
# one anyway: PRD 14.2 makes `attribution` NOT NULL, and the renderers refuse
# to display a verse without one (15.2). A source and public-domain statement
# satisfies that honestly and tells a reader where the text came from.
TRANSLATIONS = {
    "KJV": {
        "id": "eng_kjv",
        "name": "King James Version",
        "attribution": (
            "King James Version (Authorized Version), 1769 Blayney revision. "
            "Public domain. Text from eBible.org."
        ),
    },
    "ASV": {
        "id": "eng_asv",
        "name": "American Standard Version",
        "attribution": (
            "American Standard Version (1901). Public domain. Text from eBible.org."
        ),
    },
    "BSB": {
        "id": "eng_bsb",
        "name": "Berean Standard Bible",
        "attribution": "Berean Standard Bible. Public domain. Text from eBible.org.",
    },
}

REPO_ROOT = Path(__file__).resolve().parent.parent

OUT_DIR = REPO_ROOT / "src-tauri" / "assets" / "translations"
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

    def __init__(self, translation_id: str) -> None:
        self.translation_id = translation_id
        self.session = requests.Session()
        self.session.headers["Accept"] = "application/json"
        self._last = 0.0

    def chapter(self, book: str, chapter: int) -> dict:
        cache_file = CACHE_DIR / self.translation_id / f"{book}.{chapter}.json"
        if cache_file.exists():
            try:
                return json.loads(cache_file.read_text(encoding="utf-8"))
            except json.JSONDecodeError:
                cache_file.unlink()

        data = self.get(f"/{self.translation_id}/{book}/{chapter}.json")
        cache_file.parent.mkdir(parents=True, exist_ok=True)
        cache_file.write_text(json.dumps(data), encoding="utf-8")
        return data


def verses_from_chapter(payload: dict) -> tuple[list[tuple[int, str]], list[int]]:
    """Pull (verse number, text) out of a helloao chapter.

    A verse's content is a list mixing text fragments with markup objects:
    ``{"text": ..., "poem": 1}`` carries scripture, while ``{"noteId": ...}``
    and ``{"lineBreak": true}`` do not. Only text is kept — a footnote marker
    on a projector is noise, and a note body is not scripture.

    Returns the verses and, separately, the numbers of verse nodes the source
    published with no text of their own. Those are editorial omissions, not
    holes: the ASV revisers moved Matthew 17:21, 18:11 and 23:14 (and about
    thirteen others) to the margin, so ebible.org emits the verse node with
    only a footnote inside. Keeping the two apart matters — an absent node
    means the parser lost a verse, which is the bug that silently dropped 122
    verses from the API.Bible packs, and it must keep failing the build.
    """
    out: list[tuple[int, str]] = []
    omitted: list[int] = []

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
        else:
            omitted.append(number)

    return out, omitted


# An editorial omission is a verse the translation itself declines to print.
# A handful is normal for a revision working from a different Greek text; a
# flood means the parser is dropping verse bodies, so the build stops.
MAX_OMISSIONS = 40


def check_complete(
    verses: list[dict],
    per_book_expected: dict[str, int],
    omissions: list[tuple[str, int, int]],
) -> None:
    """Refuse to write a pack with holes in it.

    Same rules as build-translation-pack.py, plus a per-book cross-check
    against the count the source itself reports. Editorial omissions count
    towards that cross-check — the source counts the verse node it published,
    so the arithmetic only balances if we do too.
    """
    present = {(v["b"], v["c"], v["v"]) for v in verses}
    omitted_at = {(b, c, v) for b, c, v in omissions}

    if len(omissions) > MAX_OMISSIONS:
        sys.exit(
            f"{len(omissions)} verses came back with no text, more than the {MAX_OMISSIONS} "
            "a translation plausibly omits on purpose. Refusing to write."
        )

    missing_books = sorted(CANON_SET - {v["b"] for v in verses})
    if missing_books:
        sys.exit(f"missing books {missing_books}. Refusing to write.")

    if len(verses) < 30_800:
        sys.exit(f"only {len(verses)} verses, expected about {EXPECTED_VERSES}. Refusing to write.")

    absent = [f"{b} {c}:{v}" for b, c, v in CANARY_VERSES if (b, c, v) not in present]
    # None of the canaries is a verse any translation omits, so an omission
    # here is still a failure.
    if absent:
        sys.exit(f"these verses are missing: {', '.join(absent)}. The parser is wrong somewhere.")

    counted: dict[str, int] = {}
    chapters: dict[tuple[str, int], set[int]] = {}
    for verse in verses:
        counted[verse["b"]] = counted.get(verse["b"], 0) + 1
        chapters.setdefault((verse["b"], verse["c"]), set()).add(verse["v"])

    for (book, chapter), numbers in sorted(chapters.items()):
        accounted = len(numbers) + sum(1 for b, c, _ in omissions if (b, c) == (book, chapter))
        if accounted < max(numbers) * 0.8:
            sys.exit(
                f"{book} {chapter} has {len(numbers)} verses but numbering reaches {max(numbers)}. "
                "Verses were dropped; refusing to write."
            )

    # The source publishes its own per-book verse counts, so disagreement means
    # our parsing lost something rather than the text differing.
    for book, expected in per_book_expected.items():
        got = counted.get(book, 0)
        skipped = sum(1 for b, _, _ in omissions if b == book)
        if expected and got + skipped != expected:
            sys.exit(
                f"{book}: parsed {got} verses ({skipped} omitted by the translation) "
                f"but the source reports {expected}. "
                "Refusing to write a pack that disagrees with its source."
            )


def build(code: str) -> None:
    spec = TRANSLATIONS[code]
    client = Client(spec["id"])

    meta = client.get(f"/{spec['id']}/books.json")
    translation = meta["translation"]
    books = [b for b in meta["books"] if b["id"] in CANON_SET]

    print(f"Building {code} from {translation['name']} ({translation['id']})")
    print(f"  source: {translation.get('website')}")
    print(f"  books: {len(books)} of 66, chapters: {sum(b['numberOfChapters'] for b in books)}")

    verses: list[dict] = []
    per_book_expected: dict[str, int] = {}
    omissions: list[tuple[str, int, int]] = []

    for book in books:
        book_code = book["id"]
        per_book_expected[book_code] = book.get("totalNumberOfVerses", 0)

        for chapter in range(1, book["numberOfChapters"] + 1):
            payload = client.chapter(book_code, chapter)
            found, skipped = verses_from_chapter(payload)
            for number, text in found:
                verses.append({"b": book_code, "c": chapter, "v": number, "t": text})
            omissions.extend((book_code, chapter, number) for number in skipped)

        print(f"    {book_code}: {len(verses)} verses so far", flush=True)

    print(f"    {code}: {len(verses)} verses total          ")
    if omissions:
        listed = ", ".join(f"{b} {c}:{v}" for b, c, v in omissions)
        print(f"    {len(omissions)} omitted by this translation: {listed}")
    check_complete(verses, per_book_expected, omissions)

    OUT_DIR.mkdir(parents=True, exist_ok=True)
    path = OUT_DIR / f"{code.lower()}.jsonl.gz"

    header = {
        "code": code,
        "name": spec["name"],
        "short_name": code,
        "language": "eng",
        "source": "ebible",
        "yvp_version_id": None,
        "attribution": spec["attribution"],
        "verse_count": len(verses),
        # Verses this translation prints in the margin rather than the text.
        # The app needs these to say "the ASV omits this verse" instead of
        # projecting a blank when someone looks up Matthew 17:21.
        "omitted_verses": [f"{b}.{c}.{v}" for b, c, v in omissions],
    }

    with gzip.open(path, "wt", encoding="utf-8", compresslevel=9) as handle:
        handle.write(json.dumps(header, separators=(",", ":"), ensure_ascii=False) + "\n")
        for verse in verses:
            handle.write(json.dumps(verse, separators=(",", ":"), ensure_ascii=False) + "\n")

    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    (OUT_DIR / f"{code.lower()}.jsonl.gz.sha256").write_text(digest + "\n", encoding="utf-8")

    manifest = dict(
        header,
        sha256=digest,
        size_bytes=path.stat().st_size,
        source_detail={
            "api": "https://bible.helloao.org (Free Use Bible API)",
            "text_source": translation.get("website"),
            "license_url": translation.get("licenseUrl"),
            "omissions_note": (
                "Verse nodes the source published with no text of their own. The 1901 "
                "revisers followed a Greek text that lacks these readings and moved them "
                "to the margin; ebible.org still counts the node, so the pack records "
                "them as omitted rather than missing."
            ) if omissions else None,
            "why_not_youversion": (
                "KJV is not licensed to our YouVersion app key and ASV returns no "
                "copyright string there. Both are public domain, so they are built "
                "from ebible.org, where YouVersion's terms do not apply."
            ),
        },
    )
    if code == "KJV":
        manifest["source_detail"]["provenance_note"] = (
            "The 1769 Blayney revision, the same edition Project Gutenberg distributes "
            "as eBook #10 (https://www.gutenberg.org/ebooks/10). ebible.org's copy is "
            "used because it is versified and machine-readable; Gutenberg's is plain "
            "prose and would need re-versifying."
        )
        manifest["source_detail"]["uk_crown_copyright"] = (
            "Public domain in the US. In the UK the KJV is under perpetual Crown "
            "copyright administered by Cambridge University Press under letters patent. "
            "Recorded here as a known consideration for UK distribution (PRD 4.2)."
        )

    (OUT_DIR / f"{code.lower()}.manifest.json").write_text(
        json.dumps(manifest, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )

    size_mb = path.stat().st_size / 1024 / 1024
    print(f"    wrote {path.relative_to(REPO_ROOT)}  {size_mb:.2f} MB  sha256 {digest[:16]}...")


def main() -> None:
    import argparse

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("codes", nargs="+", help=f"one or more of: {', '.join(TRANSLATIONS)}")
    args = parser.parse_args()

    unknown = [c for c in (c.upper() for c in args.codes) if c not in TRANSLATIONS]
    if unknown:
        sys.exit(f"unknown translation(s): {', '.join(unknown)}")

    for code in (c.upper() for c in args.codes):
        build(code)


if __name__ == "__main__":
    main()
