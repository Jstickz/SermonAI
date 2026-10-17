//! The 66-book Protestant canon.
//!
//! Order matters: the verse index stores a book as one byte of index into
//! `CANON`, so reordering this list invalidates `verse-index.bin`.
//!
//! Display names are what the operator sees on a detection card and what goes
//! into a summary PDF, so they are the ordinary spoken forms rather than USFM
//! codes (FR-13, PRD §13.3).

/// (USFM code, display name), in canonical order.
pub const CANON: [(&str, &str); 66] = [
    ("GEN", "Genesis"),
    ("EXO", "Exodus"),
    ("LEV", "Leviticus"),
    ("NUM", "Numbers"),
    ("DEU", "Deuteronomy"),
    ("JOS", "Joshua"),
    ("JDG", "Judges"),
    ("RUT", "Ruth"),
    ("1SA", "1 Samuel"),
    ("2SA", "2 Samuel"),
    ("1KI", "1 Kings"),
    ("2KI", "2 Kings"),
    ("1CH", "1 Chronicles"),
    ("2CH", "2 Chronicles"),
    ("EZR", "Ezra"),
    ("NEH", "Nehemiah"),
    ("EST", "Esther"),
    ("JOB", "Job"),
    ("PSA", "Psalms"),
    ("PRO", "Proverbs"),
    ("ECC", "Ecclesiastes"),
    ("SNG", "Song of Solomon"),
    ("ISA", "Isaiah"),
    ("JER", "Jeremiah"),
    ("LAM", "Lamentations"),
    ("EZK", "Ezekiel"),
    ("DAN", "Daniel"),
    ("HOS", "Hosea"),
    ("JOL", "Joel"),
    ("AMO", "Amos"),
    ("OBA", "Obadiah"),
    ("JON", "Jonah"),
    ("MIC", "Micah"),
    ("NAM", "Nahum"),
    ("HAB", "Habakkuk"),
    ("ZEP", "Zephaniah"),
    ("HAG", "Haggai"),
    ("ZEC", "Zechariah"),
    ("MAL", "Malachi"),
    ("MAT", "Matthew"),
    ("MRK", "Mark"),
    ("LUK", "Luke"),
    ("JHN", "John"),
    ("ACT", "Acts"),
    ("ROM", "Romans"),
    ("1CO", "1 Corinthians"),
    ("2CO", "2 Corinthians"),
    ("GAL", "Galatians"),
    ("EPH", "Ephesians"),
    ("PHP", "Philippians"),
    ("COL", "Colossians"),
    ("1TH", "1 Thessalonians"),
    ("2TH", "2 Thessalonians"),
    ("1TI", "1 Timothy"),
    ("2TI", "2 Timothy"),
    ("TIT", "Titus"),
    ("PHM", "Philemon"),
    ("HEB", "Hebrews"),
    ("JAS", "James"),
    ("1PE", "1 Peter"),
    ("2PE", "2 Peter"),
    ("1JN", "1 John"),
    ("2JN", "2 John"),
    ("3JN", "3 John"),
    ("JUD", "Jude"),
    ("REV", "Revelation"),
];

/// Chapters in each book, in [`CANON`] order.
///
/// The regex detection stage needs this to tell a reference from a number
/// that happens to follow a name: "John, thirty people came" is not John 30,
/// because John has 21 chapters. Without the check every such phrase becomes a
/// provisional candidate, and an operator who sees candidates appear and
/// retract will stop trusting the panel (PRD §18.1).
///
/// Verified in tests: the values sum to 1,189, the Protestant canon's chapter
/// count, and the books with one chapter match `reference::SINGLE_CHAPTER`.
pub const CHAPTER_COUNTS: [u8; 66] = [
    50, 40, 27, 36, 34, 24, 21, 4, 31, 24, // Genesis .. 2 Samuel
    22, 25, 29, 36, 10, 13, 10, 42, 150, 31, // 1 Kings .. Proverbs
    12, 8, 66, 52, 5, 48, 12, 14, 3, 9, // Ecclesiastes .. Amos
    1, 4, 7, 3, 3, 3, 2, 14, 4, // Obadiah .. Malachi
    28, 16, 24, 21, 28, 16, 16, 13, 6, 6, // Matthew .. Ephesians
    4, 4, 5, 3, 6, 4, 3, 1, 13, 5, // Philippians .. James
    5, 3, 5, 1, 1, 1, 22, // 1 Peter .. Revelation
];

/// Verses in each chapter of each book, in canonical order: the KJV's
/// versification, which is what the regex stage, the cache's passage ids and
/// the detection index all address. Derived from the bundled KJV pack on
/// 17 Oct 2026 (31,102 verses over 1,189 chapters) and verified in tests
/// against both totals and the chapter table above.
///
/// This is what makes "John 5:150" a parse error rather than an empty
/// screen (FR-30, FR-31; M3 polish item 1): a reference is only a reference
/// if the chapter and verse exist. Translations that omit a verse (the ASV's
/// sixteen) still have the slot; the cache answers "not in this version".
pub const VERSE_COUNTS: [&[u8]; 66] = [
    // GEN
    &[
        31, 25, 24, 26, 32, 22, 24, 22, 29, 32, 32, 20, 18, 24, 21, 16, 27, 33, 38, 18, 34, 24, 20,
        67, 34, 35, 46, 22, 35, 43, 55, 32, 20, 31, 29, 43, 36, 30, 23, 23, 57, 38, 34, 34, 28, 34,
        31, 22, 33, 26,
    ],
    // EXO
    &[
        22, 25, 22, 31, 23, 30, 25, 32, 35, 29, 10, 51, 22, 31, 27, 36, 16, 27, 25, 26, 36, 31, 33,
        18, 40, 37, 21, 43, 46, 38, 18, 35, 23, 35, 35, 38, 29, 31, 43, 38,
    ],
    // LEV
    &[
        17, 16, 17, 35, 19, 30, 38, 36, 24, 20, 47, 8, 59, 57, 33, 34, 16, 30, 37, 27, 24, 33, 44,
        23, 55, 46, 34,
    ],
    // NUM
    &[
        54, 34, 51, 49, 31, 27, 89, 26, 23, 36, 35, 16, 33, 45, 41, 50, 13, 32, 22, 29, 35, 41, 30,
        25, 18, 65, 23, 31, 40, 16, 54, 42, 56, 29, 34, 13,
    ],
    // DEU
    &[
        46, 37, 29, 49, 33, 25, 26, 20, 29, 22, 32, 32, 18, 29, 23, 22, 20, 22, 21, 20, 23, 30, 25,
        22, 19, 19, 26, 68, 29, 20, 30, 52, 29, 12,
    ],
    // JOS
    &[
        18, 24, 17, 24, 15, 27, 26, 35, 27, 43, 23, 24, 33, 15, 63, 10, 18, 28, 51, 9, 45, 34, 16,
        33,
    ],
    // JDG
    &[
        36, 23, 31, 24, 31, 40, 25, 35, 57, 18, 40, 15, 25, 20, 20, 31, 13, 31, 30, 48, 25,
    ],
    // RUT
    &[22, 23, 18, 22],
    // 1SA
    &[
        28, 36, 21, 22, 12, 21, 17, 22, 27, 27, 15, 25, 23, 52, 35, 23, 58, 30, 24, 42, 15, 23, 29,
        22, 44, 25, 12, 25, 11, 31, 13,
    ],
    // 2SA
    &[
        27, 32, 39, 12, 25, 23, 29, 18, 13, 19, 27, 31, 39, 33, 37, 23, 29, 33, 43, 26, 22, 51, 39,
        25,
    ],
    // 1KI
    &[
        53, 46, 28, 34, 18, 38, 51, 66, 28, 29, 43, 33, 34, 31, 34, 34, 24, 46, 21, 43, 29, 53,
    ],
    // 2KI
    &[
        18, 25, 27, 44, 27, 33, 20, 29, 37, 36, 21, 21, 25, 29, 38, 20, 41, 37, 37, 21, 26, 20, 37,
        20, 30,
    ],
    // 1CH
    &[
        54, 55, 24, 43, 26, 81, 40, 40, 44, 14, 47, 40, 14, 17, 29, 43, 27, 17, 19, 8, 30, 19, 32,
        31, 31, 32, 34, 21, 30,
    ],
    // 2CH
    &[
        17, 18, 17, 22, 14, 42, 22, 18, 31, 19, 23, 16, 22, 15, 19, 14, 19, 34, 11, 37, 20, 12, 21,
        27, 28, 23, 9, 27, 36, 27, 21, 33, 25, 33, 27, 23,
    ],
    // EZR
    &[11, 70, 13, 24, 17, 22, 28, 36, 15, 44],
    // NEH
    &[11, 20, 32, 23, 19, 19, 73, 18, 38, 39, 36, 47, 31],
    // EST
    &[22, 23, 15, 17, 14, 14, 10, 17, 32, 3],
    // JOB
    &[
        22, 13, 26, 21, 27, 30, 21, 22, 35, 22, 20, 25, 28, 22, 35, 22, 16, 21, 29, 29, 34, 30, 17,
        25, 6, 14, 23, 28, 25, 31, 40, 22, 33, 37, 16, 33, 24, 41, 30, 24, 34, 17,
    ],
    // PSA
    &[
        6, 12, 8, 8, 12, 10, 17, 9, 20, 18, 7, 8, 6, 7, 5, 11, 15, 50, 14, 9, 13, 31, 6, 10, 22,
        12, 14, 9, 11, 12, 24, 11, 22, 22, 28, 12, 40, 22, 13, 17, 13, 11, 5, 26, 17, 11, 9, 14,
        20, 23, 19, 9, 6, 7, 23, 13, 11, 11, 17, 12, 8, 12, 11, 10, 13, 20, 7, 35, 36, 5, 24, 20,
        28, 23, 10, 12, 20, 72, 13, 19, 16, 8, 18, 12, 13, 17, 7, 18, 52, 17, 16, 15, 5, 23, 11,
        13, 12, 9, 9, 5, 8, 28, 22, 35, 45, 48, 43, 13, 31, 7, 10, 10, 9, 8, 18, 19, 2, 29, 176, 7,
        8, 9, 4, 8, 5, 6, 5, 6, 8, 8, 3, 18, 3, 3, 21, 26, 9, 8, 24, 13, 10, 7, 12, 15, 21, 10, 20,
        14, 9, 6,
    ],
    // PRO
    &[
        33, 22, 35, 27, 23, 35, 27, 36, 18, 32, 31, 28, 25, 35, 33, 33, 28, 24, 29, 30, 31, 29, 35,
        34, 28, 28, 27, 28, 27, 33, 31,
    ],
    // ECC
    &[18, 26, 22, 16, 20, 12, 29, 17, 18, 20, 10, 14],
    // SNG
    &[17, 17, 11, 16, 16, 13, 13, 14],
    // ISA
    &[
        31, 22, 26, 6, 30, 13, 25, 22, 21, 34, 16, 6, 22, 32, 9, 14, 14, 7, 25, 6, 17, 25, 18, 23,
        12, 21, 13, 29, 24, 33, 9, 20, 24, 17, 10, 22, 38, 22, 8, 31, 29, 25, 28, 28, 25, 13, 15,
        22, 26, 11, 23, 15, 12, 17, 13, 12, 21, 14, 21, 22, 11, 12, 19, 12, 25, 24,
    ],
    // JER
    &[
        19, 37, 25, 31, 31, 30, 34, 22, 26, 25, 23, 17, 27, 22, 21, 21, 27, 23, 15, 18, 14, 30, 40,
        10, 38, 24, 22, 17, 32, 24, 40, 44, 26, 22, 19, 32, 21, 28, 18, 16, 18, 22, 13, 30, 5, 28,
        7, 47, 39, 46, 64, 34,
    ],
    // LAM
    &[22, 22, 66, 22, 22],
    // EZK
    &[
        28, 10, 27, 17, 17, 14, 27, 18, 11, 22, 25, 28, 23, 23, 8, 63, 24, 32, 14, 49, 32, 31, 49,
        27, 17, 21, 36, 26, 21, 26, 18, 32, 33, 31, 15, 38, 28, 23, 29, 49, 26, 20, 27, 31, 25, 24,
        23, 35,
    ],
    // DAN
    &[21, 49, 30, 37, 31, 28, 28, 27, 27, 21, 45, 13],
    // HOS
    &[11, 23, 5, 19, 15, 11, 16, 14, 17, 15, 12, 14, 16, 9],
    // JOL
    &[20, 32, 21],
    // AMO
    &[15, 16, 15, 13, 27, 14, 17, 14, 15],
    // OBA
    &[21],
    // JON
    &[17, 10, 10, 11],
    // MIC
    &[16, 13, 12, 13, 15, 16, 20],
    // NAM
    &[15, 13, 19],
    // HAB
    &[17, 20, 19],
    // ZEP
    &[18, 15, 20],
    // HAG
    &[15, 23],
    // ZEC
    &[21, 13, 10, 14, 11, 15, 14, 23, 17, 12, 17, 14, 9, 21],
    // MAL
    &[14, 17, 18, 6],
    // MAT
    &[
        25, 23, 17, 25, 48, 34, 29, 34, 38, 42, 30, 50, 58, 36, 39, 28, 27, 35, 30, 34, 46, 46, 39,
        51, 46, 75, 66, 20,
    ],
    // MRK
    &[
        45, 28, 35, 41, 43, 56, 37, 38, 50, 52, 33, 44, 37, 72, 47, 20,
    ],
    // LUK
    &[
        80, 52, 38, 44, 39, 49, 50, 56, 62, 42, 54, 59, 35, 35, 32, 31, 37, 43, 48, 47, 38, 71, 56,
        53,
    ],
    // JHN
    &[
        51, 25, 36, 54, 47, 71, 53, 59, 41, 42, 57, 50, 38, 31, 27, 33, 26, 40, 42, 31, 25,
    ],
    // ACT
    &[
        26, 47, 26, 37, 42, 15, 60, 40, 43, 48, 30, 25, 52, 28, 41, 40, 34, 28, 41, 38, 40, 30, 35,
        27, 27, 32, 44, 31,
    ],
    // ROM
    &[
        32, 29, 31, 25, 21, 23, 25, 39, 33, 21, 36, 21, 14, 23, 33, 27,
    ],
    // 1CO
    &[
        31, 16, 23, 21, 13, 20, 40, 13, 27, 33, 34, 31, 13, 40, 58, 24,
    ],
    // 2CO
    &[24, 17, 18, 18, 21, 18, 16, 24, 15, 18, 33, 21, 14],
    // GAL
    &[24, 21, 29, 31, 26, 18],
    // EPH
    &[23, 22, 21, 32, 33, 24],
    // PHP
    &[30, 30, 21, 23],
    // COL
    &[29, 23, 25, 18],
    // 1TH
    &[10, 20, 13, 18, 28],
    // 2TH
    &[12, 17, 18],
    // 1TI
    &[20, 15, 16, 16, 25, 21],
    // 2TI
    &[18, 26, 17, 22],
    // TIT
    &[16, 15, 15],
    // PHM
    &[25],
    // HEB
    &[14, 18, 19, 16, 14, 20, 28, 13, 28, 39, 40, 29, 25],
    // JAS
    &[27, 26, 18, 17, 20],
    // 1PE
    &[25, 25, 22, 19, 14],
    // 2PE
    &[21, 22, 18],
    // 1JN
    &[10, 29, 24, 21, 21],
    // 2JN
    &[13],
    // 3JN
    &[14],
    // JUD
    &[25],
    // REV
    &[
        20, 29, 22, 11, 14, 17, 17, 13, 21, 11, 19, 17, 18, 20, 8, 21, 18, 24, 21, 15, 27, 21,
    ],
];

/// Chapters in the book with this USFM code, or `None` for an unknown code.
pub fn chapters(code: &str) -> Option<u8> {
    CANON
        .iter()
        .position(|(c, _)| *c == code)
        .map(|i| CHAPTER_COUNTS[i])
}

/// Verses in `chapter` (1-based) of the book at `index`, or `None` when the
/// book or chapter does not exist.
pub fn verses(index: u8, chapter: u16) -> Option<u8> {
    let book = VERSE_COUNTS.get(index as usize)?;
    if chapter == 0 {
        return None;
    }
    book.get(chapter as usize - 1).copied()
}

/// USFM code for a book index, or `None` if the index is out of range.
pub fn code(index: u8) -> Option<&'static str> {
    CANON.get(index as usize).map(|(code, _)| *code)
}

/// Display name for a book index, e.g. 42 -> "John".
pub fn name(index: u8) -> Option<&'static str> {
    CANON.get(index as usize).map(|(_, name)| *name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canon_is_the_66_books_in_order() {
        assert_eq!(CANON.len(), 66);
        assert_eq!(code(0), Some("GEN"));
        assert_eq!(name(65), Some("Revelation"));
        // 39 Old Testament books, so Matthew starts the New at index 39.
        assert_eq!(code(39), Some("MAT"));
        assert_eq!(name(42), Some("John"));
    }

    #[test]
    fn chapter_counts_add_up_to_the_canon() {
        // 1,189 is the well-known total. A single transposed digit in the
        // table above breaks this, which is the point of asserting the sum
        // rather than only a few values.
        let total: u32 = CHAPTER_COUNTS.iter().map(|&c| u32::from(c)).sum();
        assert_eq!(total, 1_189);

        // The ends and the outliers, by name rather than by index.
        assert_eq!(chapters("GEN"), Some(50));
        assert_eq!(chapters("PSA"), Some(150));
        assert_eq!(chapters("JHN"), Some(21));
        assert_eq!(chapters("REV"), Some(22));
        assert_eq!(chapters("XYZ"), None);
    }

    #[test]
    fn verse_counts_match_the_canon_and_the_chapter_table() {
        let verses_total: u32 = VERSE_COUNTS
            .iter()
            .flat_map(|b| b.iter())
            .map(|&v| u32::from(v))
            .sum();
        assert_eq!(verses_total, 31_102, "the KJV's verse count");
        for (i, book) in VERSE_COUNTS.iter().enumerate() {
            assert_eq!(
                book.len(),
                CHAPTER_COUNTS[i] as usize,
                "{}: chapter table and verse table disagree",
                CANON[i].0
            );
        }
        // The ones a wrong reference is most likely to name.
        assert_eq!(verses(42, 5), Some(47), "John 5");
        assert_eq!(verses(42, 21), Some(25), "John 21");
        assert_eq!(verses(18, 119), Some(176), "Psalm 119");
        assert_eq!(verses(18, 151), None, "there is no Psalm 151");
        assert_eq!(verses(64, 1), Some(25), "Jude");
        assert_eq!(verses(64, 2), None, "Jude has one chapter");
        assert_eq!(verses(0, 0), None);
        assert_eq!(verses(66, 1), None);
    }

    #[test]
    fn the_single_chapter_books_are_the_ones_the_parser_knows() {
        // reference.rs expands "Jude 3" to Jude 1:3 for exactly these five.
        // Two lists that must agree, kept in two places, drift — so one test
        // holds them together.
        let ones: Vec<&str> = CANON
            .iter()
            .zip(CHAPTER_COUNTS.iter())
            .filter(|(_, &n)| n == 1)
            .map(|((code, _), _)| *code)
            .collect();
        assert_eq!(ones, ["OBA", "PHM", "2JN", "3JN", "JUD"]);
    }
}
