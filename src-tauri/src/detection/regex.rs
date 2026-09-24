//! The regex stage: direct scripture references in transcript text (FR-12,
//! FR-13). Standard, spoken and shorthand forms, under 5 ms.
//!
//! Not a single regular expression, despite the name the PRD gives the stage.
//! Spoken numbers — "eight twenty eight", "the twenty-ninth chapter" — need to
//! be *read*, and a book name can be one word or three, so this is a word
//! walker that reuses [`spoken_numbers::read`] and the parser's alias table.
//! It runs in linear time over a few dozen words and is comfortably inside the
//! budget.
//!
//! ## What the transcript actually looks like
//!
//! Measured on 24 September 2026 by streaming the test sermon to Nova-3 and
//! printing the settled text (`examples/transcribe_file.rs`):
//!
//! ```text
//! [   8.6s] this morning to the book of Jeremiah.
//! [  11.5s] Chapter 29.
//! [  13.5s] Verse 11,
//! [  55.9s] He said in Romans eight twenty eight that all
//! [  67.5s] Turn with me if you will to first Thessalonians chapter five.
//! [ 124.9s] Look at what John writes in his first epistle.
//! ```
//!
//! Three things follow, and each shaped this module:
//!
//! - **A reference is split across settled utterances**, with a full stop
//!   between the book and its chapter. Scanning one final at a time would
//!   never see "Jeremiah 29:11" at all. The caller passes a **window of recent
//!   words spanning utterance boundaries**, and this module strips the
//!   punctuation that the boundaries leave behind.
//! - **Digits follow a keyword; words follow nothing.** "Chapter 29" arrives as
//!   digits, "Romans eight twenty eight" arrives spelled out, with no
//!   separator. So `8:28` has to be recovered from `eight twenty eight` — which
//!   works because a unit word ends a number, so "eight" is read alone and
//!   "twenty eight" after it.
//! - **"John writes in his first epistle" is not 1 John.** A book name only
//!   counts when a chapter follows it, and an ordinal only belongs to a book
//!   when a book name follows *it*.
//!
//! ## Two-stage detection
//!
//! This stage runs on interim text as well as final text (PRD §18.1). It is
//! pure — words in, hits out — and knows nothing about which it was given;
//! the pipeline marks a hit provisional or confirmed. So nothing here may have
//! side effects, and nothing here decides what is projectable.
//!
//! ## Guards against the wrong verse
//!
//! A verse on the projector cannot be recalled, so this stage would rather
//! miss than invent:
//!
//! - a chapter beyond the book's count is rejected — "John, thirty people
//!   came" is not John 30 ([`books::CHAPTER_COUNTS`]);
//! - a verse above 176 is rejected by [`spoken_numbers::read`];
//! - a two-letter alias ("is" for Isaiah, "ps" for Psalms) is accepted only
//!   in the standard written form with a colon, never spoken;
//! - a bare "Book N" with no keyword and no verse is a hit, but a low-confidence
//!   one, because "Mark three things" is a real sentence.

use crate::bible::books;
use crate::bible::reference::{self, UsfmRef};

use super::spoken_numbers;

/// Which surface form was matched. Drives the confidence the pipeline assigns
/// (FR-16), and is shown on the detection card's source badge (PRD §13.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    /// "John 3:16", "1 Cor 13:4-7", "Ps 23.1".
    Standard,
    /// "John chapter three verse sixteen", "Romans eight twenty eight".
    Spoken,
    /// "the third chapter of John", "the twenty-ninth chapter of Jeremiah,
    /// verse eleven".
    Shorthand,
    /// "Romans 8", "Jude 3" — a book and one number, nothing else.
    BareChapter,
}

impl Form {
    /// The regex stage's own confidence in a hit of this form, before the
    /// pipeline weighs it against other stages (FR-16).
    ///
    /// Written forms with a colon are almost never accidental. A spoken
    /// reference with "chapter" or "verse" in it is nearly as safe. Numbers run
    /// together after a book name are usually a reference but sometimes a
    /// count. A bare "Mark 3" is the weakest: a real sentence says that.
    pub fn confidence(self) -> f32 {
        match self {
            Form::Standard => 0.97,
            Form::Shorthand => 0.92,
            Form::Spoken => 0.85,
            Form::BareChapter => 0.55,
        }
    }
}

/// One reference found in the text.
#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub reference: UsfmRef,
    pub form: Form,
    /// Word indices into the input, `start..end`, so the caller can highlight
    /// the words and dedupe hits across overlapping windows.
    pub start: usize,
    pub end: usize,
}

/// Every direct reference in `text`, in order of appearance.
///
/// `text` may span several settled utterances and carry their punctuation.
/// Hits never overlap; scanning resumes after each one.
pub fn scan(text: &str) -> Vec<Hit> {
    let tokens = tokenize(text);
    let words: Vec<&str> = tokens.iter().map(String::as_str).collect();

    let mut hits = Vec::new();
    let mut i = 0;
    while i < words.len() {
        if let Some(hit) = match_at(&words, i) {
            i = hit.end;
            hits.push(hit);
        } else {
            i += 1;
        }
    }
    hits
}

/// Try both shapes at one position. Shorthand first: it *starts* with a number,
/// and a book-first attempt at "third" would find nothing and move on anyway,
/// but checking the rarer shape first keeps the common path short.
fn match_at(words: &[&str], i: usize) -> Option<Hit> {
    match_shorthand(words, i).or_else(|| match_book_first(words, i))
}

// ---------------------------------------------------------------------------
// Book-first: "<book> [chapter] <n> [<sep> <n> [<range> <n>]]"
// ---------------------------------------------------------------------------

fn match_book_first(words: &[&str], i: usize) -> Option<Hit> {
    let (code, book_words, written_only) = read_book(words, i)?;
    let mut pos = i + book_words;

    let mut form = Form::Spoken;
    let mut keyword = false;
    let single_chapter = books::chapters(code)? == 1;

    // "Jude verse three": a single-chapter book goes straight to the verse.
    // For any other book "verse" with no chapter is ambiguous and is left to
    // the operator rather than guessed.
    if single_chapter && is_verse_word(words.get(pos).copied()) {
        let (v, used) = spoken_numbers::read(&words[pos + 1..])?;
        let end = pos + 1 + used;
        if magnitude_follows(words, end) {
            return None;
        }
        let reference = build(code, 1, Some(v), None)?;
        return Some(Hit {
            reference,
            form: Form::Spoken,
            start: i,
            end,
        });
    }

    // Optional "chapter" keyword. Its presence is a strong signal that the
    // number after it is a chapter and not a count.
    if is_chapter_word(words.get(pos).copied()) {
        keyword = true;
        pos += 1;
    }

    // "Jeremiah, the twenty-ninth chapter, verse eleven": the wireframe's own
    // phrasing. An ordinal chapter after the book, then the keyword.
    let (chapter, used) = if words.get(pos).copied() == Some("the") {
        let (n, used) = spoken_numbers::read(&words[pos + 1..])?;
        if !is_chapter_word(words.get(pos + 1 + used).copied()) {
            return None;
        }
        keyword = true;
        form = Form::Shorthand;
        (n, used + 2)
    } else {
        spoken_numbers::read(&words[pos..])?
    };
    pos += used;
    if magnitude_follows(words, pos) {
        return None;
    }

    // A book name followed by a number, but the book was a two-letter alias
    // ("is 3 o'clock"): only the written colon form is trusted for those.
    if written_only && words.get(pos).copied() != Some(":") {
        return None;
    }

    // Verse: ":" or "." separator, a "verse" keyword, or nothing — "Romans
    // eight twenty eight" has no separator at all.
    let mut verse: Option<u16> = None;
    let mut end_verse: Option<u16> = None;

    // "chapter three and verse sixteen": the "and" is filler, but only when a
    // verse keyword follows it — "chapter five and John 3:16" is two
    // references, and the "and" belongs to neither.
    let mut look = pos;
    if words.get(look).copied() == Some("and") && is_verse_word(words.get(look + 1).copied()) {
        look += 1;
    }

    if let Some(next) = words.get(look).copied() {
        if next == ":" {
            form = Form::Standard;
            pos = look + 1;
            let (v, used) = spoken_numbers::read(&words[pos..])?;
            verse = Some(v);
            pos += used;
        } else if is_verse_word(Some(next)) {
            keyword = true;
            pos = look + 1;
            let (v, used) = spoken_numbers::read(&words[pos..])?;
            verse = Some(v);
            pos += used;
        } else if spoken_numbers::starts_number(next) && !is_range_word(Some(next)) {
            // Run together: "eight twenty eight". Only when the second number
            // reads cleanly; "John 3 people" reads nothing and stays a chapter.
            if let Some((v, used)) = spoken_numbers::read(&words[pos..]) {
                verse = Some(v);
                pos += used;
            }
        }
    }

    if verse.is_some() && magnitude_follows(words, pos) {
        return None;
    }

    // Range: "4-7", "four to seven", "four through seven".
    if verse.is_some() && is_range_word(words.get(pos).copied()) {
        if let Some((e, used)) = spoken_numbers::read(&words[pos + 1..]) {
            if e > verse.unwrap_or(0) {
                end_verse = Some(e);
                pos += 1 + used;
            }
        }
    }

    if verse.is_none() {
        form = if keyword {
            Form::Spoken
        } else {
            Form::BareChapter
        };
    }

    let reference = build(code, chapter, verse, end_verse)?;
    Some(Hit {
        reference,
        form,
        start: i,
        end: pos,
    })
}

// ---------------------------------------------------------------------------
// Shorthand: "[the] <nth> chapter of <book> [, [the] verse <n> | <nth> verse]"
// ---------------------------------------------------------------------------

fn match_shorthand(words: &[&str], i: usize) -> Option<Hit> {
    let mut pos = i;
    if words.get(pos).copied() == Some("the") {
        pos += 1;
    }

    let (chapter, used) = spoken_numbers::read(&words[pos..])?;
    pos += used;

    if !is_chapter_word(words.get(pos).copied()) {
        return None;
    }
    pos += 1;
    if words.get(pos).copied() != Some("of") {
        return None;
    }
    pos += 1;

    let (code, book_words, written_only) = read_book(words, pos)?;
    if written_only {
        return None;
    }
    pos += book_words;

    // Optional verse: ", verse eleven" or ", and the eleventh verse".
    let mut verse = None;
    let mut look = pos;
    if words.get(look).copied() == Some("and") {
        look += 1;
    }
    if words.get(look).copied() == Some("the") {
        look += 1;
    }
    if is_verse_word(words.get(look).copied()) {
        if let Some((v, used)) = spoken_numbers::read(&words[look + 1..]) {
            verse = Some(v);
            pos = look + 1 + used;
        }
    } else if let Some((v, used)) = spoken_numbers::read(&words[look..]) {
        // "the eleventh verse"
        if is_verse_word(words.get(look + used).copied()) {
            verse = Some(v);
            pos = look + used + 1;
        }
    }

    let reference = build(code, chapter, verse, None)?;
    Some(Hit {
        reference,
        form: Form::Shorthand,
        start: i,
        end: pos,
    })
}

// ---------------------------------------------------------------------------
// Pieces
// ---------------------------------------------------------------------------

/// Read a book name starting at `i`.
///
/// Returns the USFM code, how many words it took, and whether the match came
/// from a two-letter alias that is only safe in the written colon form.
///
/// Longest first — "song of solomon" before "song" — and a leading ordinal
/// ("first", "1st", "i", or a digit 1–3) is joined to the name so the parser's
/// own normalisation handles it. An ordinal with no book after it is not a
/// book: "the first thing" reads nothing here.
fn read_book(words: &[&str], i: usize) -> Option<(&'static str, usize, bool)> {
    for take in (1..=4).rev() {
        let Some(slice) = words.get(i..i + take) else {
            continue;
        };
        // A book name never contains a separator token or a chapter keyword.
        if slice
            .iter()
            .any(|w| *w == ":" || *w == "-" || is_chapter_word(Some(w)) || is_verse_word(Some(w)))
        {
            continue;
        }
        // Nor does it start with a number unless that number is 1–3 and a
        // name follows — the parser's `normalize_ordinal` handles "1 john".
        if let Some(first) = slice.first() {
            let numeric = first.chars().all(|c| c.is_ascii_digit());
            if numeric && (take == 1 || !matches!(*first, "1" | "2" | "3")) {
                continue;
            }
        }

        let joined = slice.join(" ");
        let normalized = reference::normalize_ordinal(&joined);
        let flat: String = normalized
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect();
        if flat.is_empty() {
            continue;
        }

        if let Some(code) = reference::resolve_book(&flat) {
            // Single words that resolve but are ordinary English need more
            // evidence than a spoken number after them. Two letters ("is",
            // "ps", "ge") are the worst offenders and are limited to the
            // written form; the caller enforces that.
            let written_only = take == 1 && flat.chars().count() <= 2;
            return Some((code, take, written_only));
        }
    }
    None
}

fn is_chapter_word(word: Option<&str>) -> bool {
    matches!(word, Some("chapter" | "chapters" | "chap" | "ch"))
}

fn is_verse_word(word: Option<&str>) -> bool {
    matches!(word, Some("verse" | "verses" | "v" | "vv" | "vs"))
}

/// "one thousand" reads as 1 in `spoken_numbers::read`, because a unit ends a
/// number and "thousand" is never a chapter or verse. So the word after a
/// number has to be checked: if it is a magnitude, the number was not one.
fn magnitude_follows(words: &[&str], pos: usize) -> bool {
    matches!(
        words.get(pos).copied(),
        Some("thousand" | "million" | "billion" | "hundred")
    )
}

fn is_range_word(word: Option<&str>) -> bool {
    matches!(word, Some("-" | "to" | "through" | "thru"))
}

/// Build the reference through the parser, so the single-chapter rule and the
/// chapter-only convention live in one place, then check the chapter exists.
fn build(
    code: &'static str,
    chapter: u16,
    verse: Option<u16>,
    end_verse: Option<u16>,
) -> Option<UsfmRef> {
    // "John 30" is not a reference; John has 21 chapters. Checked before the
    // parser because the parser is happy to build any chapter number.
    let single_chapter = books::chapters(code)? == 1;
    if !single_chapter && chapter > u16::from(books::chapters(code)?) {
        return None;
    }
    // "Jude 4:2" names a chapter Jude does not have. "Jude 3" alone is fine:
    // the parser reads it as verse 3 of the only chapter.
    if single_chapter && verse.is_some() && chapter != 1 {
        return None;
    }

    let text = match (verse, end_verse) {
        (Some(v), Some(e)) => format!("{code} {chapter}:{v}-{e}"),
        (Some(v), None) => format!("{code} {chapter}:{v}"),
        (None, _) => format!("{code} {chapter}"),
    };
    reference::parse(&text).ok()
}

/// Lowercase words with punctuation stripped, and the separators that carry
/// meaning kept as their own tokens.
///
/// "John 3:16-18." becomes `john 3 : 16 - 18`. "twenty-ninth" becomes
/// `twenty ninth`: a hyphen between letters joins number words, between digits
/// it is a range. A full stop between digits ("3.16") is a separator, at the
/// end of a word it is punctuation — the utterance boundaries in the measured
/// transcript leave one after every book name.
fn tokenize(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for raw in text.split_whitespace() {
        let lower = raw.to_lowercase();
        let chars: Vec<char> = lower.chars().collect();
        let mut current = String::new();

        let flush = |current: &mut String, out: &mut Vec<String>| {
            if !current.is_empty() {
                out.push(std::mem::take(current));
            }
        };

        for (k, &c) in chars.iter().enumerate() {
            let prev_digit = k > 0 && chars[k - 1].is_ascii_digit();
            let next_digit = chars.get(k + 1).is_some_and(|n| n.is_ascii_digit());

            match c {
                ':' => {
                    flush(&mut current, &mut out);
                    out.push(":".into());
                }
                '.' if prev_digit && next_digit => {
                    flush(&mut current, &mut out);
                    out.push(":".into());
                }
                '-' | '\u{2013}' | '\u{2014}' if prev_digit && next_digit => {
                    flush(&mut current, &mut out);
                    out.push("-".into());
                }
                '-' | '\u{2013}' => {
                    // "twenty-ninth": two number words.
                    flush(&mut current, &mut out);
                }
                c if c.is_alphanumeric() => current.push(c),
                // Apostrophes, commas, full stops at word ends: dropped.
                _ => {}
            }
        }
        flush(&mut current, &mut out);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(text: &str) -> Option<String> {
        let hits = scan(text);
        assert!(
            hits.len() <= 1,
            "expected at most one hit in {text:?}, got {hits:?}"
        );
        hits.into_iter().next().map(|h| h.reference.passage_id())
    }

    #[test]
    fn the_measured_transcript_lines_are_found() {
        // Verbatim from Nova-3 on 24 Sept, punctuation and casing included.
        // The first is three settled utterances joined — the case that a
        // per-final scan can never see.
        assert_eq!(
            one("this morning to the book of Jeremiah. Chapter 29. Verse 11,"),
            Some("JER.29.11".into())
        );
        assert_eq!(
            one("He said in Romans eight twenty eight that all"),
            Some("ROM.8.28".into())
        );
        assert_eq!(
            one("Turn with me if you will to first Thessalonians chapter five."),
            Some("1TH.5.1".into())
        );
        // And the one that must NOT match.
        assert_eq!(one("Look at what John writes in his first epistle."), None);
    }

    #[test]
    fn the_wireframes_own_example_is_shorthand() {
        assert_eq!(
            one("Jeremiah, the twenty-ninth chapter, verse eleven"),
            Some("JER.29.11".into())
        );
        assert_eq!(
            one("the twenty-ninth chapter of Jeremiah, verse eleven"),
            Some("JER.29.11".into())
        );
        assert_eq!(
            one("the third chapter of John and the sixteenth verse"),
            Some("JHN.3.16".into())
        );
        assert_eq!(one("the third chapter of John"), Some("JHN.3.1".into()));
    }

    #[test]
    fn standard_forms_and_ranges() {
        assert_eq!(one("John 3:16"), Some("JHN.3.16".into()));
        assert_eq!(one("John 3.16"), Some("JHN.3.16".into()));
        assert_eq!(one("1 Corinthians 13:4-7"), Some("1CO.13.4-7".into()));
        assert_eq!(one("1 Cor 13:4–7"), Some("1CO.13.4-7".into()));
        assert_eq!(one("Psalm 23:1"), Some("PSA.23.1".into()));
        assert_eq!(one("Ps 23:1"), Some("PSA.23.1".into()));
        assert_eq!(
            one("John chapter 3 verses 16 to 18"),
            Some("JHN.3.16-18".into())
        );
    }

    #[test]
    fn single_chapter_books_take_the_verse_directly() {
        assert_eq!(one("Jude 3"), Some("JUD.1.3".into()));
        assert_eq!(one("Jude verse three"), Some("JUD.1.3".into()));
        assert_eq!(one("Philemon 6"), Some("PHM.1.6".into()));
    }

    #[test]
    fn a_chapter_the_book_does_not_have_is_not_a_reference() {
        // The guard that keeps counts from becoming candidates.
        assert_eq!(one("John, thirty people came"), None);
        assert_eq!(one("Romans 20"), None);
        assert_eq!(one("Jude 4:2"), None);
        assert_eq!(one("Jude four two"), None);
    }

    #[test]
    fn ordinary_english_that_looks_like_a_reference() {
        assert_eq!(one("it is 3 o'clock"), None); // "is" = Isaiah, spoken: refused
        assert_eq!(one("Is 53:5"), Some("ISA.53.5".into())); // written: accepted
        assert_eq!(one("the first thing we do"), None);
        assert_eq!(one("acts of kindness"), None);
        assert_eq!(one("a job interview at nine"), None);
        // "one" reads as 1 and "thousand" would be ignored: caught by looking
        // at the word after the number.
        assert_eq!(one("Psalm 23 verse one thousand"), None);
    }

    #[test]
    fn bare_chapters_are_hits_but_weak_ones() {
        let hits = scan("Mark three things");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].form, Form::BareChapter);
        assert!(hits[0].form.confidence() < Form::Spoken.confidence());
    }

    #[test]
    fn several_references_in_one_window_come_back_in_order() {
        let hits = scan("Paul reminds us in Romans 8:28, and John 3:16 says, and Psalm 23");
        let ids: Vec<String> = hits.iter().map(|h| h.reference.passage_id()).collect();
        assert_eq!(ids, ["ROM.8.28", "JHN.3.16", "PSA.23.1"]);
        assert!(hits[0].end <= hits[1].start && hits[1].end <= hits[2].start);
    }

    #[test]
    fn spans_point_at_the_words() {
        let hits = scan("turn to John 3:16 please");
        assert_eq!((hits[0].start, hits[0].end), (2, 6)); // john 3 : 16
    }

    #[test]
    fn the_stage_is_inside_its_budget() {
        // A 60-second window at speaking pace is around 150 words. PRD §18.1
        // gives the regex stage 5 ms p95; this runs a hundred windows.
        let window = "Paul reminds us in Romans eight twenty eight that all things work together for good, and the twenty-ninth chapter of Jeremiah, verse eleven, and turn with me to first Thessalonians chapter five and John 3:16 and the third chapter of John and the sixteenth verse; ".repeat(3);
        let started = std::time::Instant::now();
        for _ in 0..100 {
            let hits = scan(&window);
            assert!(hits.len() >= 15);
        }
        let per_scan = started.elapsed() / 100;
        // Printed on success as well as failure: the DoD wants the figure
        // recorded, and a passing test that keeps its measurement to itself
        // leaves nothing to record. Visible with `--nocapture`.
        eprintln!(
            "regex stage: {per_scan:?} per ~150-word window ({} profile)",
            if cfg!(debug_assertions) { "debug" } else { "release" }
        );
        // The 5 ms is a budget for the shipped build. A debug build is an
        // order of magnitude slower and is not what the PRD is about; the
        // release-profile figure is what `cargo test --release` reports.
        let budget_ms = if cfg!(debug_assertions) { 50 } else { 5 };
        assert!(
            per_scan.as_millis() < budget_ms,
            "regex stage took {per_scan:?} per window against {budget_ms} ms"
        );
    }
}
