//! What does the vector stage's top score look like for a verbatim verse, a
//! paraphrase, and ordinary preaching that quotes nothing?
//!
//! The Claude paraphrase stage (FR-14) is gated on this score: high enough to
//! accept the vector stage's answer without asking, low enough to be sure there
//! is no scripture here, and a middle band where Claude is worth a call. The
//! band edges have to come from the distribution of real scores, not from a
//! round number, and the three populations have to actually separate — if
//! paraphrases and plain speech score alike, no threshold exists and the gate
//! is theatre.
//!
//! ```text
//! cargo run --release --example score_bands
//! ```
//!
//! Release profile matters only for speed here; the scores are identical.

use std::path::PathBuf;

use sermonai_lib::detection::vector::{SemanticSearch, VerseRef};

/// A phrase and, for the two labelled populations, the verse it should hit.
type Case<'a> = (&'a str, Option<(u8, u8, u8)>);

fn assets_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets")
}

fn main() {
    let search = SemanticSearch::load(&assets_dir()).expect("assets");

    // KJV wording, because the shipped index is built from the KJV.
    let verbatim: &[(&str, (u8, u8, u8))] = &[
        (
            "For God so loved the world, that he gave his only begotten Son",
            (42, 3, 16),
        ),
        (
            "For I know the thoughts that I think toward you, saith the LORD, thoughts of peace",
            (23, 29, 11),
        ),
        ("The LORD is my shepherd; I shall not want", (18, 23, 1)),
        (
            "In the beginning God created the heaven and the earth",
            (0, 1, 1),
        ),
        (
            "And we know that all things work together for good to them that love God",
            (44, 8, 28),
        ),
        (
            "I can do all things through Christ which strengtheneth me",
            (49, 4, 13),
        ),
        (
            "Trust in the LORD with all thine heart; and lean not unto thine own understanding",
            (19, 3, 5),
        ),
        (
            "Now faith is the substance of things hoped for, the evidence of things not seen",
            (57, 11, 1),
        ),
    ];

    // The recall test's eight modern paraphrases, with their targets.
    let paraphrase: &[(&str, (u8, u8, u8))] = &[
        (
            "God loved everyone so much he sent his only son",
            (42, 3, 16),
        ),
        (
            "the Lord takes care of me like a shepherd looks after sheep",
            (18, 23, 1),
        ),
        (
            "all things work together for good for those who love God",
            (44, 8, 28),
        ),
        (
            "I can do everything through Christ who strengthens me",
            (49, 4, 13),
        ),
        (
            "I know the plans I have for you, plans to prosper you",
            (23, 29, 11),
        ),
        ("trust in the Lord with all your heart", (19, 3, 5)),
        ("be strong and courageous, do not be afraid", (5, 1, 9)),
        ("faith is being sure of what we hope for", (57, 11, 1)),
    ];

    // Verbatim from the Nova-3 transcript of the test sermon, 24 Sept: the
    // preacher's own words, quoting nothing. The lines that read like
    // scripture but are not any verse are the ones that matter.
    let preaching: &[&str] = &[
        "Good morning church. Thank you for being here today.",
        "I want us to open our Bibles this morning",
        "And I want you to receive this word not as history",
        "But as a promise God is speaking over your life today",
        "Because before we can walk into what God has for us",
        "we have to first trust the one who wrote our story",
        "Paul understood this.",
        "And I know some of us are in a season where it does not look good",
        "God is not surprised by your situation",
        "The prophet asked the same question we ask",
        "So I want to leave you with this",
        "Beloved, hear that again",
        "please stand as we sing the closing hymn together",
        "the offering baskets are at the back of the hall",
        "our youth camp registration closes this Friday",
    ];

    println!("top1     top2    gap     phrase");

    let report = |label: &str, cases: &[Case]| {
        println!("\n== {label} ==");
        let mut tops = Vec::new();
        for (phrase, expected) in cases {
            let hits = search.search(phrase, 5).expect("search");
            let top1 = hits.first().map(|h| h.score).unwrap_or(0.0);
            let top2 = hits.get(1).map(|h| h.score).unwrap_or(0.0);
            let mark = match expected {
                Some((b, c, v)) => {
                    let want = VerseRef {
                        book: *b,
                        chapter: *c,
                        verse: *v,
                    };
                    let rank = hits.iter().position(|h| h.verse == want);
                    match rank {
                        Some(0) => "top1 ".to_string(),
                        Some(r) => format!("top{} ", r + 1),
                        None => "MISS ".to_string(),
                    }
                }
                None => "     ".to_string(),
            };
            println!(
                "{top1:<8.3} {top2:<7.3} {:<7.3} {mark}{}  -> {}",
                top1 - top2,
                phrase,
                hits.first().map(|h| h.verse.display()).unwrap_or_default()
            );
            tops.push(top1);
        }
        tops.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = tops.len();
        println!(
            "   top-1 score: min {:.3}  median {:.3}  max {:.3}  (n={n})",
            tops[0],
            tops[n / 2],
            tops[n - 1]
        );
    };

    report(
        "VERBATIM (KJV) — the vector stage should be trusted alone",
        &verbatim
            .iter()
            .map(|(p, t)| (*p, Some(*t)))
            .collect::<Vec<_>>(),
    );
    report(
        "PARAPHRASE — where Claude earns its call",
        &paraphrase
            .iter()
            .map(|(p, t)| (*p, Some(*t)))
            .collect::<Vec<_>>(),
    );
    report(
        "PREACHING, no scripture — Claude should not be called",
        &preaching.iter().map(|p| (*p, None)).collect::<Vec<_>>(),
    );
}
