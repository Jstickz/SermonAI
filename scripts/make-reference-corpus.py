"""Generate the spoken-reference corpus for the regex stage (M2 deliverable 1).

Writes src-tauri/tests/fixtures/spoken_references.tsv: one case per line,
`input<TAB>expected`, where expected is a USFM passage ID or `-` for "must
match nothing". Run from the repository root:

    python scripts/make-reference-corpus.py

Two sources, kept separate so their weight is visible:

1. **Verbatim lines from Nova-3.** The test sermon streamed to Deepgram on
   24 Sept 2026 (`examples/transcribe_file.rs`), punctuation and casing as it
   arrived. These are the ground truth the generated variants imitate. Note
   that one reference arrived as three settled utterances — "Jeremiah." /
   "Chapter 29." / "Verse 11," — which is why the scanner takes a window.

2. **Generated variants.** Each target reference rendered in every surface
   form the scanner claims to handle: written with a colon, abbreviated, with
   "chapter"/"verse" keywords in digits and in words, run together in words
   with no separator, ordinal-chapter shorthand both ways round, a range, and
   embedded mid-sentence with the punctuation a settled utterance leaves.

Plus negatives: ordinary sentences that contain a book name, an ordinal or a
number and must not become a candidate. A false positive puts a verse in
staging that nobody said; on the regex stage a miss is recoverable by the
later stages and a false hit is not.

The generator is deterministic. Regenerating changes nothing unless the
tables below change, so the fixture can be reviewed in a diff.
"""

from __future__ import annotations

import os

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, "src-tauri", "tests", "fixtures", "spoken_references.tsv")

# (input, expected) — verbatim from Nova-3, 24 Sept 2026.
MEASURED = [
    ("this morning to the book of Jeremiah. Chapter 29. Verse 11,", "JER.29.11"),
    ("He said in Romans eight twenty eight that all", "ROM.8.28"),
    ("Turn with me if you will to first Thessalonians chapter five.", "1TH.5.1"),
    ("Look at what John writes in his first epistle.", "-"),
    ("Good morning, church.", "-"),
    ("Thoughts of peace, and not of evil, to give you an expected end.", "-"),
    ("How long, oh Lord? Shall I cry? And thou wilt not hear.", "-"),
]

UNITS = "zero one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen".split()
ORDS = "zeroth first second third fourth fifth sixth seventh eighth ninth tenth eleventh twelfth thirteenth fourteenth fifteenth sixteenth seventeenth eighteenth nineteenth".split()
TENS = {20: ("twenty", "twentieth"), 30: ("thirty", "thirtieth"), 40: ("forty", "fortieth"),
        50: ("fifty", "fiftieth"), 60: ("sixty", "sixtieth"), 70: ("seventy", "seventieth"),
        80: ("eighty", "eightieth"), 90: ("ninety", "ninetieth")}


def words(n: int) -> str:
    if n < 20:
        return UNITS[n]
    if n < 100:
        t, u = divmod(n, 10)
        return TENS[t * 10][0] + ("" if u == 0 else f" {UNITS[u]}")
    h, r = divmod(n, 100)
    return "one hundred" + ("" if r == 0 else f" and {words(r)}")


def ordinal(n: int) -> str:
    if n < 20:
        return ORDS[n]
    if n < 100:
        t, u = divmod(n, 10)
        return TENS[t * 10][1] if u == 0 else f"{TENS[t * 10][0]}-{ORDS[u]}"
    h, r = divmod(n, 100)
    return "one hundred and " + ordinal(r) if r else "one hundredth"


# (code, display name, spoken/abbrev alias, chapter, verse, end_verse or None)
# Chosen across both testaments, numbered books, single-chapter books, Psalms
# past 100, and books whose names are also English words.
TARGETS = [
    ("JHN", "John", "Jn", 3, 16, None),
    ("ROM", "Romans", "Rom", 8, 28, None),
    ("JER", "Jeremiah", "Jer", 29, 11, None),
    ("PSA", "Psalm", "Ps", 23, 1, None),
    ("PSA", "Psalm", "Ps", 119, 105, None),
    ("1CO", "1 Corinthians", "1 Cor", 13, 4, 7),
    ("1TH", "1 Thessalonians", "1 Thess", 5, 16, 18),
    ("GEN", "Genesis", "Gen", 1, 1, None),
    ("ISA", "Isaiah", "Isa", 53, 5, None),
    ("MAT", "Matthew", "Matt", 28, 19, 20),
    ("PHP", "Philippians", "Phil", 4, 13, None),
    ("HEB", "Hebrews", "Heb", 11, 1, None),
    ("REV", "Revelation", "Rev", 21, 4, None),
    ("2TI", "2 Timothy", "2 Tim", 3, 16, None),
    ("1JN", "1 John", "1 Jn", 4, 8, None),
    ("PRO", "Proverbs", "Prov", 3, 5, 6),
    ("EPH", "Ephesians", "Eph", 2, 8, None),
    ("ACT", "Acts", "Acts", 2, 38, None),
    ("JAS", "James", "Jas", 1, 5, None),
    ("SNG", "Song of Solomon", "Song", 2, 4, None),
    ("HAB", "Habakkuk", "Hab", 2, 4, None),
    ("1SA", "1 Samuel", "1 Sam", 17, 45, None),
    ("2CH", "2 Chronicles", "2 Chr", 7, 14, None),
    ("LUK", "Luke", "Lk", 15, 20, None),
    ("MRK", "Mark", "Mk", 10, 45, None),
]

SINGLE_CHAPTER = [  # (code, name, verse)
    ("JUD", "Jude", 3),
    ("PHM", "Philemon", 6),
    ("OBA", "Obadiah", 15),
    ("3JN", "3 John", 4),
]

# Numbered books said with an ordinal word, which is how they are spoken.
SPOKEN_ORDINAL_BOOK = {"1": "first", "2": "second", "3": "third"}


def spoken_name(name: str) -> str:
    head, _, rest = name.partition(" ")
    return f"{SPOKEN_ORDINAL_BOOK[head]} {rest}" if head in SPOKEN_ORDINAL_BOOK else name


def usfm(code: str, ch: int, v: int, end: int | None) -> str:
    return f"{code}.{ch}.{v}" + (f"-{end}" if end else "")


def variants(code, name, abbr, ch, v, end):
    spoken = spoken_name(name)
    single = usfm(code, ch, v, None)
    ranged = usfm(code, ch, v, end) if end else None
    chapter_only = f"{code}.{ch}.1"
    out = [
        # Written
        (f"{name} {ch}:{v}", single),
        (f"{abbr} {ch}:{v}", single),
        (f"{name} {ch}.{v}", single),
        (f"turn with me to {name} {ch}:{v} this morning", single),
        # Spoken, keywords, digits — how Nova-3 renders it when keywords precede
        (f"{spoken} chapter {ch} verse {v}", single),
        (f"{spoken}. Chapter {ch}. Verse {v},", single),
        (f"the book of {name}, chapter {ch}, verse {v}.", single),
        # Spoken, keywords, words
        (f"{spoken} chapter {words(ch)} verse {words(v)}", single),
        (f"{spoken} chapter {words(ch)} and verse {words(v)}", single),
        # Run together, no separator — "Romans eight twenty eight"
        (f"he said in {spoken} {words(ch)} {words(v)} that", single),
        (f"{spoken} {words(ch)} {words(v)}", single),
        # Shorthand, both ways round
        (f"{spoken}, the {ordinal(ch)} chapter, verse {words(v)}", single),
        (f"the {ordinal(ch)} chapter of {spoken}, verse {words(v)}", single),
        (f"the {ordinal(ch)} chapter of {spoken} and the {ordinal(v)} verse", single),
        (f"the {ordinal(ch)} chapter of {spoken}", chapter_only),
        # Chapter only
        (f"{spoken} chapter {words(ch)}", chapter_only),
    ]
    if ranged:
        out += [
            (f"{name} {ch}:{v}-{end}", ranged),
            (f"{spoken} chapter {ch} verses {v} to {end}", ranged),
            (f"{spoken} chapter {words(ch)} verses {words(v)} through {words(end)}", ranged),
        ]
    return out


def single_chapter_variants(code, name, v):
    spoken = spoken_name(name)
    target = f"{code}.1.{v}"
    return [
        (f"{name} {v}", target),
        (f"{spoken} verse {words(v)}", target),
        (f"the book of {name}, verse {v}.", target),
    ]


NEGATIVES = [
    # A book name that is also a word, followed by a number that is a count.
    "it is 3 o'clock and we should begin",
    "mark my words, this will happen",
    "acts of kindness are never wasted",
    "a job interview at nine in the morning",
    "there were numbers of people at the gate",
    "judges in the land were corrupt",
    "the kings of the earth took counsel",
    "the song we sang earlier",
    # An ordinal that is not a numbered book.
    "the first thing we do is pray",
    "John writes in his first epistle",
    "in the second place, consider this",
    "for the third time I say rejoice",
    # A chapter the book does not have.
    "John, thirty people came to him",
    "Romans twenty",
    "Jude four two",
    "Matthew chapter thirty",
    "Genesis 51:1",
    "Psalm 151",
    # Numbers without a book.
    "chapter three of the story of our church",
    "verse two of the hymn",
    "twenty nine eleven",
    "3:16 in the afternoon",
    # Names of people that are also books.
    "brother James will lead us",
    "sister Ruth is unwell this week",
    "Daniel and Joel are on the sound desk",
    "Peter, come forward",
    "my son Samuel is seven",
    # Genuine but out of range verses
    "John 3:200",
    "Psalm 23 verse one thousand",
]


def main() -> None:
    rows: list[tuple[str, str]] = []
    rows += MEASURED
    for t in TARGETS:
        rows += variants(*t)
    for s in SINGLE_CHAPTER:
        rows += single_chapter_variants(*s)
    rows += [(n, "-") for n in NEGATIVES]

    seen = set()
    unique = []
    for inp, exp in rows:
        if inp not in seen:
            seen.add(inp)
            unique.append((inp, exp))

    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w", encoding="utf-8", newline="\n") as f:
        f.write("# input\texpected USFM passage ID, or - for no reference\n")
        f.write(f"# generated by scripts/make-reference-corpus.py; {len(unique)} cases\n")
        for inp, exp in unique:
            f.write(f"{inp}\t{exp}\n")

    positives = sum(1 for _, e in unique if e != "-")
    print(f"wrote {OUT}")
    print(f"  {len(unique)} cases: {positives} references, {len(unique) - positives} negatives")
    print(f"  {len(MEASURED)} verbatim from Nova-3, the rest generated")


if __name__ == "__main__":
    main()
