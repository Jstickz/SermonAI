//! Custom vocabulary sent with every stream (FR-09, PRD §8.1).
//!
//! Deepgram's general model has not heard much preaching. Left alone it writes
//! "Thessalonians" as "the salonians", "Habakkuk" as "have a cook", and — seen
//! in the live FR-07 run — "John" as "join". Those are precisely the words this
//! app exists to catch: a misheard book name is a scripture that never reaches
//! the projector.
//!
//! Nova-3 calls this **keyterm prompting**. Terms are passed on the connection
//! URL and bias recognition towards them without forbidding anything else.
//!
//! ## What goes in, and what deliberately does not
//!
//! The 66 book names come from [`CANON`], not a second list written out here.
//! A copy would drift, and the failure would be silent: recognition quietly
//! worse for whichever book someone forgot to add in both places.
//!
//! Archaic and liturgical terms are here because a preacher reading the King
//! James aloud produces words no general model expects — "beseech", "thence",
//! "propitiation" — and because a church's own vocabulary is a per-church
//! setting that arrives in M8 (FR-09 enhancement), not something to hard-code.
//!
//! **Numbers are not included.** "Three sixteen" is recognised perfectly well
//! already, and spending a keyterm slot on a digit displaces a book name.

use crate::bible::books::CANON;

/// Deepgram caps keyterm prompting at 100 terms per request. Past that the
/// request is rejected outright, so the list is trimmed rather than sent and
/// refused at the start of a service.
pub const MAX_KEYTERMS: usize = 100;

/// Words a preacher reading the King James aloud will say and a general speech
/// model will not expect.
///
/// Kept short on purpose. Every entry displaces something, and the 66 book
/// names matter more than any of these: a mangled ordinary word is a typo in
/// the transcript, a mangled book name is a missed verse.
const ARCHAIC_TERMS: &[&str] = &[
    // King James verb and pronoun forms, by far the most common source of
    // nonsense in a read passage.
    "thee",
    "thou",
    "thy",
    "thine",
    "ye",
    "saith",
    "hath",
    "doth",
    "shalt",
    "wilt",
    "unto",
    "beseech",
    "behold",
    "verily",
    "whosoever",
    "thence",
    "henceforth",
    // Theological vocabulary that is rare in general speech and central here.
    "propitiation",
    "justification",
    "sanctification",
    "righteousness",
    "covenant",
    "atonement",
    "redemption",
    "Gentiles",
    "Pharisees",
    "epistle",
    "doxology",
    "benediction",
    "Godhead",
];

/// The terms to send with a stream.
///
/// Book names first: if the list is ever trimmed, the words whose loss costs a
/// verse survive and the ones whose loss costs a typo do not.
pub fn keyterms() -> Vec<String> {
    let mut terms: Vec<String> = CANON
        .iter()
        // The display name, which is what a preacher says. "1 Samuel" rather
        // than "1SA", and the USFM code would bias towards nothing spoken.
        .map(|(_, name)| (*name).to_string())
        .chain(ARCHAIC_TERMS.iter().map(|term| (*term).to_string()))
        .collect();

    terms.truncate(MAX_KEYTERMS);
    terms
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_book_of_the_canon_is_sent() {
        let terms = keyterms();

        // All 66, by name and not by USFM code: the point is to bias towards
        // what a preacher says out loud.
        for (code, name) in CANON.iter() {
            assert!(
                terms.iter().any(|t| t == name),
                "{name} ({code}) is missing from the vocabulary"
            );
        }
    }

    #[test]
    fn the_list_fits_inside_deepgrams_limit() {
        // Past 100 the request is rejected outright, which would fail at the
        // start of a service rather than degrade.
        assert!(
            keyterms().len() <= MAX_KEYTERMS,
            "{} terms exceeds the {MAX_KEYTERMS} cap",
            keyterms().len()
        );
    }

    #[test]
    fn book_names_survive_a_trim_and_archaic_terms_give_way() {
        // The ordering rule, asserted rather than assumed: if the archaic list
        // grows past the cap one day, the books must still all be there.
        let terms = keyterms();
        let books = 66;

        assert!(terms.len() >= books);
        for (index, (_, name)) in CANON.iter().enumerate() {
            assert_eq!(&terms[index], name, "book order changed");
        }
    }

    #[test]
    fn the_words_that_broke_in_testing_are_covered() {
        let terms = keyterms();

        // "John" became "join" in the live FR-07 run. The other two are the
        // classic mishearings for these books.
        for word in ["John", "1 Thessalonians", "Habakkuk"] {
            assert!(terms.iter().any(|t| t == word), "{word} should be biased");
        }
    }

    #[test]
    fn no_term_is_blank_or_padded() {
        // stream_url skips blanks, but a padded term would be sent with %20 on
        // either end and bias towards a string nobody says.
        for term in keyterms() {
            assert!(!term.trim().is_empty(), "blank term");
            assert_eq!(
                term.trim(),
                term,
                "term has surrounding whitespace: {term:?}"
            );
        }
    }
}
