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
}
