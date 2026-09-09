//! Verifies the bundled semantic search end to end (FR-15).
//!
//! The load-bearing question is whether the Rust query encoder lands in the
//! same embedding space as the Python script that built `verse-index.bin`. If
//! it does not, nothing errors — search just returns confident nonsense. So
//! these tests assert on retrieval of known verses rather than on shapes.

use std::path::PathBuf;
use std::time::Instant;

use sermonai_lib::detection::vector::{SemanticSearch, VerseRef};
use sermonai_lib::detection::{EMBEDDING_DIMS, VERSE_COUNT};

fn assets_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")
}

fn search() -> SemanticSearch {
    SemanticSearch::load(&assets_dir()).expect("bundled encoder and verse index should load")
}

#[test]
fn index_holds_the_whole_canon_at_the_expected_width() {
    let search = search();
    assert_eq!(search.index().len(), VERSE_COUNT);
    assert_eq!(search.index().dims(), EMBEDDING_DIMS);
    assert_eq!(search.encoder().dims(), EMBEDDING_DIMS);
}

#[test]
fn a_verse_quoted_verbatim_retrieves_itself() {
    let search = search();

    // Book indexes into the 66-book canon: John is 42, Jeremiah 23, Psalms 18.
    let cases = [
        (
            "For God so loved the world, that he gave his only begotten Son",
            VerseRef {
                book: 42,
                chapter: 3,
                verse: 16,
            },
        ),
        (
            "For I know the thoughts that I think toward you, saith the LORD, thoughts of peace",
            VerseRef {
                book: 23,
                chapter: 29,
                verse: 11,
            },
        ),
        (
            "The LORD is my shepherd; I shall not want",
            VerseRef {
                book: 18,
                chapter: 23,
                verse: 1,
            },
        ),
        (
            "In the beginning God created the heaven and the earth",
            VerseRef {
                book: 0,
                chapter: 1,
                verse: 1,
            },
        ),
    ];

    for (phrase, expected) in cases {
        let hits = search.search(phrase, 5).expect("search");
        assert!(!hits.is_empty(), "no hits for {phrase:?}");

        let found = hits.iter().any(|h| h.verse == expected);
        assert!(
            found,
            "{:?} should retrieve {} but returned {:?}",
            phrase,
            expected.display(),
            hits.iter().map(|h| h.verse.display()).collect::<Vec<_>>()
        );

        // A verbatim quote should also be a strong match, not a marginal one.
        assert!(
            hits[0].score > 0.5,
            "top score for {phrase:?} was only {:.3}",
            hits[0].score
        );
    }
}

/// Measures paraphrase recall rather than asserting a specific verse.
///
/// This stage is genuinely weak at paraphrase and the numbers below say so.
/// A static bag-of-tokens model has no contextual capacity, so a preacher
/// saying "plans to prosper you" does not reliably reach Jeremiah 29:11. That
/// is what the Claude stage exists for (FR-14) and why nothing goes on screen
/// without the operator staging it (ADR 0002).
///
/// Measured 9 Sept 2026 on this 8-case set: top-1 2, top-3 4, top-5 4. The
/// floor here catches a regression — a broken tokenizer or a mismatched index
/// would collapse it to 0 — without pretending the stage is better than it is.
/// M2 tunes this against a 300-variant corpus.
#[test]
fn paraphrase_recall_stays_above_the_measured_floor() {
    let search = search();

    let cases = [
        (
            "God loved everyone so much he sent his only son",
            VerseRef {
                book: 42,
                chapter: 3,
                verse: 16,
            },
        ),
        (
            "the Lord takes care of me like a shepherd looks after sheep",
            VerseRef {
                book: 18,
                chapter: 23,
                verse: 1,
            },
        ),
        (
            "all things work together for good for those who love God",
            VerseRef {
                book: 44,
                chapter: 8,
                verse: 28,
            },
        ),
        (
            "I can do everything through Christ who strengthens me",
            VerseRef {
                book: 49,
                chapter: 4,
                verse: 13,
            },
        ),
        (
            "I know the plans I have for you, plans to prosper you",
            VerseRef {
                book: 23,
                chapter: 29,
                verse: 11,
            },
        ),
        (
            "trust in the Lord with all your heart",
            VerseRef {
                book: 19,
                chapter: 3,
                verse: 5,
            },
        ),
        (
            "be strong and courageous, do not be afraid",
            VerseRef {
                book: 5,
                chapter: 1,
                verse: 9,
            },
        ),
        (
            "faith is being sure of what we hope for",
            VerseRef {
                book: 57,
                chapter: 11,
                verse: 1,
            },
        ),
    ];

    let mut top5 = 0;
    for (phrase, expected) in cases {
        let hits = search.search(phrase, 5).expect("search");
        if hits.iter().any(|h| h.verse == expected) {
            top5 += 1;
        }
    }

    println!("paraphrase recall: top-5 {top5}/{}", cases.len());
    assert!(
        top5 >= 3,
        "paraphrase top-5 recall fell to {top5}/8; it measured 4/8 when the index was built. \
         Suspect the tokenizer, the encoder assets, or an index built from a different translation."
    );
}

#[test]
fn references_render_for_operators_and_for_the_bible_cache() {
    let john = VerseRef {
        book: 42,
        chapter: 3,
        verse: 16,
    };
    assert_eq!(john.display(), "John 3:16");
    assert_eq!(john.usfm(), "JHN 3:16");
}

#[test]
fn an_unintelligible_phrase_does_not_panic() {
    let search = search();
    // Silence, punctuation and noise all reach this stage during a service.
    for phrase in ["", "   ", "...", "mmm hmm"] {
        let hits = search.search(phrase, 5).expect("search should not fail");
        assert!(hits.len() <= 5);
    }
}

/// FR-15 budget: the vector stage must return in under 5 ms.
///
/// Only asserted in release builds. A debug build is roughly an order of
/// magnitude slower here because the dot-product loop is not optimised, and
/// failing on that would be measuring the wrong thing.
#[test]
fn search_meets_the_five_millisecond_budget() {
    let search = search();
    let phrase = "he restores my soul and leads me beside still waters";

    // Warm the page cache and any lazy allocation.
    for _ in 0..3 {
        search.search(phrase, 5).expect("search");
    }

    let runs = 20;

    // Split the two costs: tokenizing and pooling the phrase, versus the
    // 8 million multiply-adds across the index. Optimising the wrong one is
    // easy without this.
    let started = Instant::now();
    for _ in 0..runs {
        search.encoder().embed_quantized(phrase).expect("embed");
    }
    let embed_time = started.elapsed() / runs;

    let query = search.encoder().embed_quantized(phrase).expect("embed");
    let started = Instant::now();
    for _ in 0..runs {
        search.index().search(&query, 5);
    }
    let search_time = started.elapsed() / runs;

    let started = Instant::now();
    for _ in 0..runs {
        search.search(phrase, 5).expect("search");
    }
    let per_query = started.elapsed() / runs;

    println!(
        "vector stage: {:.2} ms per query ({:.2} ms embed + {:.2} ms search) over {} verses, {} dims, {} build",
        per_query.as_secs_f64() * 1000.0,
        embed_time.as_secs_f64() * 1000.0,
        search_time.as_secs_f64() * 1000.0,
        search.index().len(),
        search.index().dims(),
        if cfg!(debug_assertions) { "debug" } else { "release" }
    );

    if !cfg!(debug_assertions) {
        assert!(
            per_query.as_secs_f64() * 1000.0 < 5.0,
            "FR-15 budget missed: {:.2} ms per query",
            per_query.as_secs_f64() * 1000.0
        );
    }
}
