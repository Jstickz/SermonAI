//! Spoken or written scripture references to USFM passage IDs.
//!
//! "John 3:16" becomes `JHN.3.16`, "Psalm 139:13-16" becomes `PSA.139.13-16`.
//! This sits between the detection stages, which produce human references, and
//! the Bible client, which addresses passages by USFM ID.
//!
//! Parsing is deliberately forgiving. References reach this code from speech
//! recognition, so they arrive with inconsistent spacing, missing periods,
//! abbreviations a preacher said out loud, and ordinals spelled as words. A
//! reference we fail to parse is a verse that never reaches the screen.

use std::fmt;

use super::books::CANON;

/// A parsed reference, addressable as a USFM passage ID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsfmRef {
    /// Index into [`CANON`].
    pub book: u8,
    pub chapter: u16,
    pub verse: u16,
    /// Last verse of a range, if the reference named one.
    pub end_verse: Option<u16>,
}

impl UsfmRef {
    pub fn code(&self) -> &'static str {
        CANON[self.book as usize].0
    }

    pub fn book_name(&self) -> &'static str {
        CANON[self.book as usize].1
    }

    /// Passage ID as the Bible API addresses it, e.g. `JHN.3.16` or
    /// `PSA.139.13-16`.
    pub fn passage_id(&self) -> String {
        match self.end_verse {
            Some(end) => format!("{}.{}.{}-{}", self.code(), self.chapter, self.verse, end),
            None => format!("{}.{}.{}", self.code(), self.chapter, self.verse),
        }
    }

    /// Operator-facing form, e.g. "John 3:16" or "Psalms 139:13-16".
    pub fn display(&self) -> String {
        match self.end_verse {
            Some(end) => format!(
                "{} {}:{}-{}",
                self.book_name(),
                self.chapter,
                self.verse,
                end
            ),
            None => format!("{} {}:{}", self.book_name(), self.chapter, self.verse),
        }
    }
}

impl fmt::Display for UsfmRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.passage_id())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// The book name matched nothing in the canon.
    UnknownBook(String),
    /// No chapter or verse numbers were found.
    MissingNumbers,
    /// Chapter or verse was zero, or a range ran backwards.
    ImpossibleNumbers,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Operator-facing: name the thing that failed (Branding §9.3).
            ParseError::UnknownBook(book) => write!(f, "no book of the Bible called \"{book}\""),
            ParseError::MissingNumbers => f.write_str("no chapter and verse in that reference"),
            ParseError::ImpossibleNumbers => f.write_str("that chapter and verse cannot exist"),
        }
    }
}

/// Books with a single chapter. A preacher saying "Jude 3" means Jude 1:3, and
/// USFM still requires the chapter, so these expand to `.1.`.
const SINGLE_CHAPTER: [&str; 5] = ["OBA", "PHM", "2JN", "3JN", "JUD"];

/// Extra spellings beyond the canonical display name, lowercase and stripped
/// of spaces and periods. The leading ordinal of a numbered book is normalised
/// to a digit before lookup, so "1corinthians" covers "1 Cor.", "I Corinthians"
/// and "first corinthians".
const ALIASES: &[(&str, &str)] = &[
    ("gen", "GEN"),
    ("ge", "GEN"),
    ("gn", "GEN"),
    ("exo", "EXO"),
    ("ex", "EXO"),
    ("lev", "LEV"),
    ("lv", "LEV"),
    ("num", "NUM"),
    ("nm", "NUM"),
    ("deut", "DEU"),
    ("deu", "DEU"),
    ("dt", "DEU"),
    ("josh", "JOS"),
    ("jos", "JOS"),
    ("judg", "JDG"),
    ("jdg", "JDG"),
    ("rut", "RUT"),
    ("ru", "RUT"),
    ("1sam", "1SA"),
    ("1sa", "1SA"),
    ("2sam", "2SA"),
    ("2sa", "2SA"),
    ("1kgs", "1KI"),
    ("1ki", "1KI"),
    ("2kgs", "2KI"),
    ("2ki", "2KI"),
    ("1chron", "1CH"),
    ("1chr", "1CH"),
    ("1ch", "1CH"),
    ("2chron", "2CH"),
    ("2chr", "2CH"),
    ("2ch", "2CH"),
    ("ezra", "EZR"),
    ("ezr", "EZR"),
    ("neh", "NEH"),
    ("esth", "EST"),
    ("est", "EST"),
    ("jb", "JOB"),
    ("psalm", "PSA"),
    ("psalms", "PSA"),
    ("psa", "PSA"),
    ("ps", "PSA"),
    ("pslm", "PSA"),
    ("prov", "PRO"),
    ("pro", "PRO"),
    ("prv", "PRO"),
    ("eccl", "ECC"),
    ("ecc", "ECC"),
    ("song", "SNG"),
    ("songofsongs", "SNG"),
    ("songofsolomon", "SNG"),
    ("sos", "SNG"),
    ("isa", "ISA"),
    ("is", "ISA"),
    ("jer", "JER"),
    ("lam", "LAM"),
    ("ezek", "EZK"),
    ("eze", "EZK"),
    ("ezk", "EZK"),
    ("dan", "DAN"),
    ("dn", "DAN"),
    ("hos", "HOS"),
    ("joel", "JOL"),
    ("jol", "JOL"),
    ("amos", "AMO"),
    ("amo", "AMO"),
    ("obad", "OBA"),
    ("oba", "OBA"),
    ("jonah", "JON"),
    ("jon", "JON"),
    ("mic", "MIC"),
    ("nah", "NAM"),
    ("nam", "NAM"),
    ("hab", "HAB"),
    ("zeph", "ZEP"),
    ("zep", "ZEP"),
    ("hag", "HAG"),
    ("zech", "ZEC"),
    ("zec", "ZEC"),
    ("mal", "MAL"),
    ("matt", "MAT"),
    ("mat", "MAT"),
    ("mt", "MAT"),
    ("mark", "MRK"),
    ("mrk", "MRK"),
    ("mk", "MRK"),
    ("luke", "LUK"),
    ("luk", "LUK"),
    ("lk", "LUK"),
    ("john", "JHN"),
    ("jhn", "JHN"),
    ("jn", "JHN"),
    ("acts", "ACT"),
    ("act", "ACT"),
    ("rom", "ROM"),
    ("rm", "ROM"),
    ("philemon", "PHM"),
];

/// Aliases that would be ambiguous in the table above because several books
/// share a prefix. Checked after the exact-match pass.
const NEW_TESTAMENT_ALIASES: &[(&str, &str)] = &[
    ("1cor", "1CO"),
    ("1co", "1CO"),
    ("2cor", "2CO"),
    ("2co", "2CO"),
    ("gal", "GAL"),
    ("eph", "EPH"),
    ("phil", "PHP"),
    ("php", "PHP"),
    ("philip", "PHP"),
    ("col", "COL"),
    ("1thess", "1TH"),
    ("1thes", "1TH"),
    ("1th", "1TH"),
    ("2thess", "2TH"),
    ("2thes", "2TH"),
    ("2th", "2TH"),
    ("1tim", "1TI"),
    ("1ti", "1TI"),
    ("2tim", "2TI"),
    ("2ti", "2TI"),
    ("tit", "TIT"),
    ("phlm", "PHM"),
    ("phm", "PHM"),
    ("heb", "HEB"),
    ("jas", "JAS"),
    ("jam", "JAS"),
    ("james", "JAS"),
    ("1pet", "1PE"),
    ("1pe", "1PE"),
    ("2pet", "2PE"),
    ("2pe", "2PE"),
    ("1jn", "1JN"),
    ("2jn", "2JN"),
    ("3jn", "3JN"),
];

/// Parse a reference into a USFM passage ID.
///
/// ```
/// # use sermonai_lib::bible::reference::parse;
/// assert_eq!(parse("John 3:16").unwrap().passage_id(), "JHN.3.16");
/// assert_eq!(parse("1 Corinthians 13:4-7").unwrap().passage_id(), "1CO.13.4-7");
/// assert_eq!(parse("Jude 3").unwrap().passage_id(), "JUD.1.3");
/// ```
pub fn parse(reference: &str) -> Result<UsfmRef, ParseError> {
    let (book_part, number_part) = split_book_and_numbers(reference);

    if book_part.is_empty() {
        return Err(ParseError::UnknownBook(reference.trim().to_string()));
    }

    let code = resolve_book(&book_part).ok_or(ParseError::UnknownBook(book_part))?;
    let book = CANON
        .iter()
        .position(|(c, _)| *c == code)
        .expect("alias tables only contain canonical codes") as u8;

    let numbers = parse_numbers(&number_part)?;

    let (chapter, verse, end_verse) = if SINGLE_CHAPTER.contains(&code) && numbers.chapter_only {
        // "Jude 3" is Jude 1:3, not Jude 3:1.
        (1, numbers.first, numbers.end)
    } else if numbers.chapter_only {
        // "John 3" with no verse: start at verse 1 so the operator still gets
        // something on screen; stepping forward covers the rest of the chapter.
        (numbers.first, 1, None)
    } else {
        (numbers.first, numbers.second, numbers.end)
    };

    if chapter == 0 || verse == 0 {
        return Err(ParseError::ImpossibleNumbers);
    }
    if end_verse.is_some_and(|end| end <= verse) {
        return Err(ParseError::ImpossibleNumbers);
    }

    Ok(UsfmRef {
        book,
        chapter,
        verse,
        end_verse,
    })
}

/// Split "1 Corinthians 13:4-7" into ("1corinthians", "13:4-7").
fn split_book_and_numbers(reference: &str) -> (String, String) {
    let normalized = normalize_ordinal(reference);

    // The book name runs until the first digit that starts the chapter. A
    // leading digit belongs to the book ("1 Corinthians"), so skip position 0.
    let bytes: Vec<char> = normalized.chars().collect();
    let mut split_at = bytes.len();
    for (i, ch) in bytes.iter().enumerate() {
        if ch.is_ascii_digit() && i > 0 {
            split_at = i;
            break;
        }
    }

    let book: String = bytes[..split_at]
        .iter()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    let numbers: String = bytes[split_at..].iter().collect();

    (book, numbers)
}

/// Turn spoken and Roman ordinals into digits: "First John" and "I John"
/// both become "1john".
fn normalize_ordinal(reference: &str) -> String {
    let lower = reference.trim().to_lowercase();

    const ORDINALS: [(&str, &str); 9] = [
        ("first ", "1"),
        ("second ", "2"),
        ("third ", "3"),
        ("iii ", "3"),
        ("ii ", "2"),
        ("i ", "1"),
        ("1st ", "1"),
        ("2nd ", "2"),
        ("3rd ", "3"),
    ];

    for (word, digit) in ORDINALS {
        if let Some(rest) = lower.strip_prefix(word) {
            return format!("{digit}{rest}");
        }
    }

    lower
}

fn resolve_book(normalized: &str) -> Option<&'static str> {
    // Canonical display names first: "song of solomon" -> "songofsolomon".
    for (code, name) in CANON {
        let flat: String = name
            .to_lowercase()
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect();
        if flat == normalized {
            return Some(code);
        }
        // USFM code typed directly, e.g. "JHN".
        if code.to_lowercase() == normalized {
            return Some(code);
        }
    }

    ALIASES
        .iter()
        .chain(NEW_TESTAMENT_ALIASES.iter())
        .find(|(alias, _)| *alias == normalized)
        .map(|(_, code)| *code)
}

struct Numbers {
    first: u16,
    second: u16,
    end: Option<u16>,
    chapter_only: bool,
}

/// Parse the "13:4-7" tail. Accepts ":", ".", "v" and plain spaces as the
/// chapter/verse separator, because speech-to-text produces all of them.
fn parse_numbers(tail: &str) -> Result<Numbers, ParseError> {
    let mut groups: Vec<u16> = Vec::new();
    let mut current = String::new();
    let mut saw_dash = false;
    let mut dash_at = None;

    for ch in tail.chars() {
        if ch.is_ascii_digit() {
            current.push(ch);
            continue;
        }

        if !current.is_empty() {
            groups.push(current.parse().map_err(|_| ParseError::ImpossibleNumbers)?);
            current.clear();
        }

        if (ch == '-' || ch == '\u{2013}' || ch == '\u{2014}') && !saw_dash {
            saw_dash = true;
            dash_at = Some(groups.len());
        }
    }

    if !current.is_empty() {
        groups.push(current.parse().map_err(|_| ParseError::ImpossibleNumbers)?);
    }

    match groups.len() {
        0 => Err(ParseError::MissingNumbers),
        1 => Ok(Numbers {
            first: groups[0],
            second: 0,
            end: None,
            chapter_only: true,
        }),
        _ => {
            // With a dash after the first number ("Jude 3-5") the range is over
            // verses of a single-chapter book; otherwise it is chapter:verse.
            if dash_at == Some(1) {
                Ok(Numbers {
                    first: groups[0],
                    second: 0,
                    end: Some(groups[1]),
                    chapter_only: true,
                })
            } else {
                Ok(Numbers {
                    first: groups[0],
                    second: groups[1],
                    end: groups.get(2).copied(),
                    chapter_only: false,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(reference: &str) -> String {
        parse(reference)
            .unwrap_or_else(|e| panic!("{reference:?} should parse: {e}"))
            .passage_id()
    }

    #[test]
    fn common_references_across_the_canon() {
        // Twenty references a preacher is likely to name in a single service.
        assert_eq!(id("John 3:16"), "JHN.3.16");
        assert_eq!(id("Genesis 1:1"), "GEN.1.1");
        assert_eq!(id("Exodus 20:3"), "EXO.20.3");
        assert_eq!(id("Leviticus 19:18"), "LEV.19.18");
        assert_eq!(id("Numbers 6:24"), "NUM.6.24");
        assert_eq!(id("Deuteronomy 6:5"), "DEU.6.5");
        assert_eq!(id("Joshua 1:9"), "JOS.1.9");
        assert_eq!(id("Psalm 23:1"), "PSA.23.1");
        assert_eq!(id("Psalms 139:14"), "PSA.139.14");
        assert_eq!(id("Proverbs 3:5"), "PRO.3.5");
        assert_eq!(id("Isaiah 40:31"), "ISA.40.31");
        assert_eq!(id("Jeremiah 29:11"), "JER.29.11");
        assert_eq!(id("Matthew 6:33"), "MAT.6.33");
        assert_eq!(id("Mark 16:15"), "MRK.16.15");
        assert_eq!(id("Luke 4:18"), "LUK.4.18");
        assert_eq!(id("Acts 2:38"), "ACT.2.38");
        assert_eq!(id("Romans 8:28"), "ROM.8.28");
        assert_eq!(id("Ephesians 2:8"), "EPH.2.8");
        assert_eq!(id("Philippians 4:13"), "PHP.4.13");
        assert_eq!(id("Revelation 21:4"), "REV.21.4");
    }

    #[test]
    fn numbered_books_in_every_spelling_a_preacher_uses() {
        for spelling in [
            "1 Corinthians 13:4",
            "1Corinthians 13:4",
            "1 Cor 13:4",
            "1 Cor. 13:4",
            "I Corinthians 13:4",
            "First Corinthians 13:4",
            "1st Corinthians 13:4",
        ] {
            assert_eq!(id(spelling), "1CO.13.4", "failed on {spelling:?}");
        }

        assert_eq!(id("2 Timothy 3:16"), "2TI.3.16");
        assert_eq!(id("II Timothy 3:16"), "2TI.3.16");
        assert_eq!(id("Second Timothy 3:16"), "2TI.3.16");
        assert_eq!(id("1 John 4:8"), "1JN.4.8");
        assert_eq!(id("3 John 4"), "3JN.1.4");
        assert_eq!(id("1 Samuel 16:7"), "1SA.16.7");
        assert_eq!(id("2 Chronicles 7:14"), "2CH.7.14");
        assert_eq!(id("1 Thessalonians 5:16"), "1TH.5.16");
        assert_eq!(id("1 Peter 5:7"), "1PE.5.7");
    }

    /// A single-chapter book named with one number means a verse, not a
    /// chapter: "Jude 3" is Jude 1:3.
    #[test]
    fn single_chapter_books_expand_to_chapter_one() {
        assert_eq!(id("Jude 3"), "JUD.1.3");
        assert_eq!(id("Jude 1:3"), "JUD.1.3");
        assert_eq!(id("Obadiah 15"), "OBA.1.15");
        assert_eq!(id("Philemon 6"), "PHM.1.6");
        assert_eq!(id("2 John 6"), "2JN.1.6");
        assert_eq!(id("3 John 11"), "3JN.1.11");
        assert_eq!(id("Jude 3-5"), "JUD.1.3-5");
    }

    #[test]
    fn verse_ranges() {
        assert_eq!(id("Psalm 139:13-16"), "PSA.139.13-16");
        assert_eq!(id("1 Corinthians 13:4-7"), "1CO.13.4-7");
        assert_eq!(id("Romans 8:28-30"), "ROM.8.28-30");
        // En dash, which speech-to-text and copy-paste both produce.
        assert_eq!(id("John 3:16\u{2013}17"), "JHN.3.16-17");
    }

    #[test]
    fn separators_that_speech_to_text_produces() {
        assert_eq!(id("John 3.16"), "JHN.3.16");
        assert_eq!(id("John 3 16"), "JHN.3.16");
        assert_eq!(id("john 3:16"), "JHN.3.16");
        assert_eq!(id("  John   3:16  "), "JHN.3.16");
        assert_eq!(id("JHN 3:16"), "JHN.3.16");
    }

    #[test]
    fn a_chapter_with_no_verse_starts_at_verse_one() {
        assert_eq!(id("John 3"), "JHN.3.1");
        assert_eq!(id("Psalm 23"), "PSA.23.1");
    }

    #[test]
    fn books_whose_names_collide_on_a_prefix() {
        assert_eq!(id("Philippians 4:13"), "PHP.4.13");
        assert_eq!(id("Philemon 6"), "PHM.1.6");
        assert_eq!(id("Phil 4:13"), "PHP.4.13");
        assert_eq!(id("James 1:5"), "JAS.1.5");
        assert_eq!(id("Job 1:21"), "JOB.1.21");
        assert_eq!(id("Joel 2:28"), "JOL.2.28");
        assert_eq!(id("Jonah 2:9"), "JON.2.9");
        assert_eq!(id("John 1:1"), "JHN.1.1");
    }

    #[test]
    fn display_round_trips_for_the_operator() {
        let parsed = parse("1 cor 13:4-7").unwrap();
        assert_eq!(parsed.display(), "1 Corinthians 13:4-7");
        assert_eq!(parsed.book_name(), "1 Corinthians");
        assert_eq!(parse("Psalm 23:1").unwrap().display(), "Psalms 23:1");
    }

    #[test]
    fn bad_references_name_what_failed() {
        assert_eq!(
            parse("Hezekiah 3:16"),
            Err(ParseError::UnknownBook("hezekiah".into()))
        );
        assert_eq!(parse("John"), Err(ParseError::MissingNumbers));
        assert_eq!(parse("John 0:16"), Err(ParseError::ImpossibleNumbers));
        assert_eq!(parse("John 3:16-10"), Err(ParseError::ImpossibleNumbers));
        assert!(parse("").is_err());
        assert!(parse("   ").is_err());

        // The message is shown to an operator mid-service.
        let message = parse("Hezekiah 3:16").unwrap_err().to_string();
        assert!(
            message.contains("Hezekiah") || message.contains("hezekiah"),
            "{message}"
        );
    }

    #[test]
    fn every_canonical_book_name_parses() {
        for (code, name) in CANON {
            let reference = format!("{name} 1:1");
            let parsed = parse(&reference).unwrap_or_else(|e| panic!("{name} should parse: {e}"));
            assert_eq!(parsed.code(), code, "{name} resolved to the wrong book");
        }
    }
}
