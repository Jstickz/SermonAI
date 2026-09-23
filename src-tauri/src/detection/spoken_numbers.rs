//! Turning spoken numbers into digits (FR-13).
//!
//! A preacher says "the twenty-ninth chapter, verse eleven", not "29:11". The
//! transcript carries the words, so the detection stage has to read them.
//!
//! ## Scope, deliberately narrow
//!
//! Only numbers that can be a chapter or a verse: **1 to 176**, because Psalm
//! 119 has 176 verses and no chapter or verse anywhere exceeds that. A general
//! number parser would accept "three thousand" and produce a reference no Bible
//! contains, which is worse than not matching at all — a wrong verse on the
//! projector cannot be recalled.
//!
//! Ordinals and cardinals are both accepted because both are said:
//! "chapter three" and "the third chapter" are the same reference.
//!
//! Deepgram usually writes small numbers as digits, so `16` arrives far more
//! often than `sixteen`. This exists for when it does not, and for the forms it
//! reliably spells out — ordinals especially, since "29th" is rare in speech.

/// Units and teens, indexed by value.
const UNITS: [&str; 20] = [
    "zero",
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "eleven",
    "twelve",
    "thirteen",
    "fourteen",
    "fifteen",
    "sixteen",
    "seventeen",
    "eighteen",
    "nineteen",
];

/// Ordinal forms of the above, where they differ from "<cardinal>th".
const UNIT_ORDINALS: [&str; 20] = [
    "zeroth",
    "first",
    "second",
    "third",
    "fourth",
    "fifth",
    "sixth",
    "seventh",
    "eighth",
    "ninth",
    "tenth",
    "eleventh",
    "twelfth",
    "thirteenth",
    "fourteenth",
    "fifteenth",
    "sixteenth",
    "seventeenth",
    "eighteenth",
    "nineteenth",
];

/// Tens from twenty upwards, with their ordinal forms.
const TENS: [(&str, &str, u16); 8] = [
    ("twenty", "twentieth", 20),
    ("thirty", "thirtieth", 30),
    ("forty", "fortieth", 40),
    ("fifty", "fiftieth", 50),
    ("sixty", "sixtieth", 60),
    ("seventy", "seventieth", 70),
    ("eighty", "eightieth", 80),
    ("ninety", "ninetieth", 90),
];

/// The largest chapter or verse in the canon: Psalm 119:176.
pub const MAX_REFERENCE_NUMBER: u16 = 176;

/// Read a spoken or written number from the start of `words`.
///
/// Returns the value and how many words it consumed, so a caller can carry on
/// from the right place. `None` when the first word is not a number, or when
/// the result could not be a chapter or verse.
///
/// Handles "16", "sixteen", "sixteenth", "twenty nine", "twenty-ninth",
/// "one hundred", "a hundred and nineteen", and "119".
pub fn read(words: &[&str]) -> Option<(u16, usize)> {
    let first = words.first()?;

    // Digits win: Deepgram writes most numbers this way, and "29th" too.
    if let Some(value) = parse_digits(first) {
        return in_range(value).map(|v| (v, 1));
    }

    let mut total: u16 = 0;
    let mut used = 0usize;
    let mut saw_any = false;

    while used < words.len() {
        let word = strip_ordinal_suffix(words[used]);

        // "a hundred and nineteen" — "a" only counts before "hundred".
        if (word == "a" || word == "one")
            && words.get(used + 1).map(|w| clean(w)).as_deref() == Some("hundred")
        {
            total += 100;
            used += 2;
            saw_any = true;
            // "and" is optional filler: "a hundred and nineteen".
            if words.get(used).map(|w| clean(w)).as_deref() == Some("and") {
                used += 1;
            }
            continue;
        }

        if word == "hundred" && saw_any {
            total *= 100;
            used += 1;
            if words.get(used).map(|w| clean(w)).as_deref() == Some("and") {
                used += 1;
            }
            continue;
        }

        if let Some(tens) = tens_value(&word) {
            total += tens;
            used += 1;
            saw_any = true;
            // "twenty nine" and "twenty-nine" both reach here; the hyphenated
            // form is split by the caller.
            continue;
        }

        if let Some(unit) = unit_value(&word) {
            total += unit;
            used += 1;
            saw_any = true;
            // Nothing follows a unit in a number this small.
            break;
        }

        break;
    }

    if !saw_any {
        return None;
    }

    in_range(total).map(|v| (v, used))
}

/// Whether a word could begin a number, for a cheap pre-check.
pub fn starts_number(word: &str) -> bool {
    let cleaned = strip_ordinal_suffix(word);
    parse_digits(word).is_some()
        || unit_value(&cleaned).is_some()
        || tens_value(&cleaned).is_some()
        || cleaned == "a"
        || cleaned == "hundred"
}

fn in_range(value: u16) -> Option<u16> {
    // Rejecting out-of-range is the point: a general parser would happily
    // return 3000 and produce a reference that exists in no Bible.
    (1..=MAX_REFERENCE_NUMBER).contains(&value).then_some(value)
}

/// Strip punctuation and lowercase. Transcripts carry commas and full stops.
fn clean(word: &str) -> String {
    word.trim_matches(|c: char| !c.is_alphanumeric())
        .to_lowercase()
}

/// Remove an ordinal suffix from a digit form: "29th" -> "29".
fn strip_ordinal_suffix(word: &str) -> String {
    let cleaned = clean(word);
    for suffix in ["st", "nd", "rd", "th"] {
        if let Some(stem) = cleaned.strip_suffix(suffix) {
            if !stem.is_empty() && stem.chars().all(|c| c.is_ascii_digit()) {
                return stem.to_string();
            }
        }
    }
    cleaned
}

fn parse_digits(word: &str) -> Option<u16> {
    let cleaned = strip_ordinal_suffix(word);
    if cleaned.is_empty() || !cleaned.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    cleaned.parse().ok()
}

fn unit_value(word: &str) -> Option<u16> {
    UNITS
        .iter()
        .position(|w| *w == word)
        .or_else(|| UNIT_ORDINALS.iter().position(|w| *w == word))
        .map(|v| v as u16)
        .filter(|v| *v > 0)
}

fn tens_value(word: &str) -> Option<u16> {
    TENS.iter()
        .find(|(card, ord, _)| *card == word || *ord == word)
        .map(|(_, _, value)| *value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read_all(input: &str) -> Option<(u16, usize)> {
        // Hyphens separate words in speech-to-text output: "twenty-ninth" is
        // two number words joined, not one token.
        let replaced = input.replace('-', " ");
        let words: Vec<&str> = replaced.split_whitespace().collect();
        read(&words)
    }

    #[test]
    fn digits_are_read_directly() {
        assert_eq!(read_all("16"), Some((16, 1)));
        assert_eq!(read_all("119"), Some((119, 1)));
        // Deepgram writes ordinals this way when it writes them at all.
        assert_eq!(read_all("29th"), Some((29, 1)));
        assert_eq!(read_all("1st"), Some((1, 1)));
    }

    #[test]
    fn cardinals_and_ordinals_mean_the_same_reference() {
        // "chapter three" and "the third chapter" are the same verse.
        assert_eq!(read_all("three"), Some((3, 1)));
        assert_eq!(read_all("third"), Some((3, 1)));
        assert_eq!(read_all("sixteen"), Some((16, 1)));
        assert_eq!(read_all("sixteenth"), Some((16, 1)));
    }

    #[test]
    fn compound_numbers_read_as_one_value() {
        // The wireframe's own example: "the twenty-ninth chapter".
        assert_eq!(read_all("twenty ninth"), Some((29, 2)));
        assert_eq!(read_all("twenty-ninth"), Some((29, 2)));
        assert_eq!(read_all("twenty nine"), Some((29, 2)));
        assert_eq!(read_all("forty two"), Some((42, 2)));
        assert_eq!(read_all("ninety nine"), Some((99, 2)));
    }

    #[test]
    fn a_bare_ten_multiple_is_itself() {
        assert_eq!(read_all("twenty"), Some((20, 1)));
        assert_eq!(read_all("thirtieth"), Some((30, 1)));
    }

    #[test]
    fn hundreds_are_read_for_the_long_psalms() {
        // Psalm 119 is the reason this exists at all.
        assert_eq!(read_all("one hundred nineteen"), Some((119, 3)));
        assert_eq!(read_all("a hundred and nineteen"), Some((119, 4)));
        assert_eq!(read_all("one hundred"), Some((100, 2)));
        assert_eq!(read_all("one hundred and seventy six"), Some((176, 5)));
    }

    #[test]
    fn punctuation_does_not_hide_a_number() {
        // Transcripts arrive punctuated: "chapter three, verse sixteen."
        assert_eq!(read_all("three,"), Some((3, 1)));
        assert_eq!(read_all("sixteen."), Some((16, 1)));
    }

    #[test]
    fn nothing_outside_a_real_chapter_or_verse_is_accepted() {
        // The guard that matters. A general number parser would return these
        // happily and produce a reference no Bible contains — and a wrong
        // verse on the projector cannot be recalled.
        assert_eq!(read_all("zero"), None);
        assert_eq!(read_all("0"), None);
        assert_eq!(read_all("200"), None);
        assert_eq!(read_all("1984"), None);
        // 176 is Psalm 119's last verse, and the ceiling.
        assert_eq!(read_all("176"), Some((176, 1)));
        assert_eq!(read_all("177"), None);
    }

    #[test]
    fn words_that_are_not_numbers_are_left_alone() {
        assert_eq!(read_all("chapter"), None);
        assert_eq!(read_all("Jeremiah"), None);
        assert_eq!(read_all("and"), None);
        assert!(!starts_number("verse"));
        assert!(starts_number("sixteen"));
        assert!(starts_number("29th"));
    }

    #[test]
    fn the_count_returned_lets_a_caller_carry_on() {
        // "twenty ninth chapter" — two words consumed, "chapter" still to read.
        let replaced = "twenty ninth chapter".replace('-', " ");
        let words: Vec<&str> = replaced.split_whitespace().collect();
        let (value, used) = read(&words).expect("should read");
        assert_eq!(value, 29);
        assert_eq!(words[used], "chapter");
    }
}
