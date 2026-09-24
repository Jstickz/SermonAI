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

/// Chapters in the book with this USFM code, or `None` for an unknown code.
pub fn chapters(code: &str) -> Option<u8> {
    CANON
        .iter()
        .position(|(c, _)| *c == code)
        .map(|i| CHAPTER_COUNTS[i])
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
