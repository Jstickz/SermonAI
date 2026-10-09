//! Phase 0 of the encoder upgrade: measure the vector stage on a labelled set,
//! with the method beside every number (FR-15, FR-14, M2 Definition of Done).
//!
//! Reads a TSV of `phrase<TAB>expected<TAB>population`, where `expected` is a
//! USFM verse ID (`JHN.3.16`) or `-` for a line that quotes nothing, and
//! `population` groups the lines in the report (`verbatim`, `paraphrase`,
//! `heldout`, `preaching`, or anything else). Prints, per population:
//!
//! - **catch@1 / @3 / @5** — the expected verse's rank in the top five. The
//!   M2 DoD line is 80% at top-3 on 100 hand-labelled paraphrases, by vector
//!   **or** Claude; this is the vector stage's share of it alone.
//! - **top-1 score min / median / max** — the distribution the FR-14 gate is
//!   tuned from, measured on the same cases.
//! - **gate decisions** at the configured edges (`llm::ParaphraseGate`):
//!   how many of these lines would be accepted without a call, sent to
//!   Claude, or skipped. On a `-` population the *accepts* are false matches
//!   that reach staging with no operator warning, and must be zero.
//!
//! Every miss is listed with what the stage returned instead, because the
//! shape of the misses says more about the encoder than the rate does.
//!
//! ```text
//! cargo run --release --example measure_vector
//! cargo run --release --example measure_vector -- path/to/other.tsv
//! ```
//!
//! Release profile for speed only; the scores are identical in debug.

use std::collections::BTreeMap;
use std::path::PathBuf;

use sermonai_lib::bible::books;
use sermonai_lib::detection::vector::{
    Match, SearchOptions, SemanticSearch, SynonymMode, VerseRef,
};
use sermonai_lib::llm::ParaphraseGate;

const TOP_K: usize = 5;

struct Case {
    phrase: String,
    /// Every verse that would be a right answer. Empty for a line that
    /// quotes nothing. More than one when scripture quotes itself — "the just
    /// shall live by faith" is Habakkuk 2:4, Romans 1:17 and Galatians 3:11,
    /// and a stage that returns any of them has understood the preacher.
    expected: Vec<VerseRef>,
    population: String,
}

struct Scored<'a> {
    case: &'a Case,
    hits: Vec<Match>,
    /// 1-based rank of the expected verse within the top five, if present.
    rank: Option<usize>,
}

fn main() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = args
        .iter()
        .find(|a| a.ends_with(".tsv"))
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest.join("tests/fixtures/paraphrases.tsv"));

    // Phase 1 options, off by default so the bare stage is the baseline:
    //   --synonyms blend|max   search the KJV-vocabulary variants too
    //   --neighbours           search two-verse passages too
    let synonyms = match args
        .iter()
        .position(|a| a == "--synonyms")
        .and_then(|i| args.get(i + 1))
        .map(String::as_str)
    {
        Some("blend") => SynonymMode::Blend,
        Some("max") => SynonymMode::Max,
        Some(other) => panic!("--synonyms takes blend or max, not {other:?}"),
        None => SynonymMode::Off,
    };
    let options = SearchOptions {
        synonyms,
        neighbours: args.iter().any(|a| a == "--neighbours"),
    };

    let cases = load_cases(&path);
    let search = SemanticSearch::load(&manifest.join("assets")).expect("assets");
    let gate = ParaphraseGate::default();

    println!("set:      {}", path.display());
    println!("cases:    {}", cases.len());
    println!(
        "options:  synonyms {:?}, neighbours {}",
        options.synonyms, options.neighbours
    );
    println!(
        "index:    {} verses x {} dims; gate accept >= {:.2}, call >= {:.2}",
        search.index().len(),
        search.index().dims(),
        gate.accept_at_or_above,
        gate.call_at_or_above
    );

    let scored: Vec<Scored> = cases
        .iter()
        .map(|case| {
            let hits = search
                .search_with(&case.phrase, TOP_K, &options)
                .expect("search");
            let rank = hits
                .iter()
                .position(|h| case.expected.iter().any(|want| h.covers(want)))
                .map(|i| i + 1);
            Scored { case, hits, rank }
        })
        .collect();

    // Populations in first-seen order, so the report reads like the file.
    let mut populations: BTreeMap<usize, (&str, Vec<&Scored>)> = BTreeMap::new();
    let mut order: Vec<&str> = Vec::new();
    for s in &scored {
        let pop = s.case.population.as_str();
        let idx = match order.iter().position(|p| *p == pop) {
            Some(i) => i,
            None => {
                order.push(pop);
                order.len() - 1
            }
        };
        populations
            .entry(idx)
            .or_insert((pop, Vec::new()))
            .1
            .push(s);
    }

    for (pop, rows) in populations.values() {
        report(pop, rows, &gate);
    }

    // The DoD line, stated once, on the population named for it.
    if let Some((_, rows)) = populations.values().find(|(p, _)| *p == "paraphrase") {
        let n = rows.len();
        let top3 = rows
            .iter()
            .filter(|s| matches!(s.rank, Some(r) if r <= 3))
            .count();
        let pct = 100.0 * top3 as f64 / n.max(1) as f64;
        println!();
        println!(
            "M2 DoD (vector alone): {top3}/{n} paraphrases in top-3 = {pct:.1}% against 80% \
             — {}",
            if pct >= 80.0 {
                "MET without the Claude stage"
            } else {
                "not met by the vector stage alone; the remainder is Claude's share"
            }
        );
        if n < 100 {
            println!("note: {n} cases, the DoD asks for 100 hand-labelled");
        }
    }
}

fn report(pop: &str, rows: &[&Scored], gate: &ParaphraseGate) {
    let n = rows.len();
    let labelled = rows.iter().filter(|s| !s.case.expected.is_empty()).count();
    println!();
    println!("== {pop} (n={n}) ==");

    let mut top1: Vec<f32> = rows
        .iter()
        .map(|s| s.hits.first().map(|h| h.score).unwrap_or(0.0))
        .collect();
    top1.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    if labelled > 0 {
        let at = |k: usize| {
            rows.iter()
                .filter(|s| matches!(s.rank, Some(r) if r <= k))
                .count()
        };
        println!(
            "catch@1 {:>3}/{labelled} ({:.0}%)   catch@3 {:>3}/{labelled} ({:.0}%)   catch@5 {:>3}/{labelled} ({:.0}%)",
            at(1),
            pct(at(1), labelled),
            at(3),
            pct(at(3), labelled),
            at(5),
            pct(at(5), labelled),
        );
    }
    println!(
        "top-1 score  min {:.3}  median {:.3}  max {:.3}",
        top1.first().copied().unwrap_or(0.0),
        median(&top1),
        top1.last().copied().unwrap_or(0.0),
    );

    let accept = top1
        .iter()
        .filter(|s| **s >= gate.accept_at_or_above)
        .count();
    let call = top1
        .iter()
        .filter(|s| **s >= gate.call_at_or_above && **s < gate.accept_at_or_above)
        .count();
    let skip = n - accept - call;
    println!("gate         accept {accept}   call {call}   skip {skip}");

    if labelled == 0 {
        // A line that quotes nothing and is accepted anyway reaches staging as
        // a confirmed card with the stage's own name on it. Say so loudly.
        let false_accepts: Vec<&&Scored> = rows
            .iter()
            .filter(|s| {
                s.hits
                    .first()
                    .map(|h| h.score >= gate.accept_at_or_above)
                    .unwrap_or(false)
            })
            .collect();
        if false_accepts.is_empty() {
            println!("false accepts 0");
        } else {
            println!(
                "FALSE ACCEPTS {} — these would reach staging unasked:",
                false_accepts.len()
            );
            for s in false_accepts {
                let h = &s.hits[0];
                println!("  {:.3}  {:<14} {:?}", h.score, label(h), s.case.phrase);
            }
        }
        return;
    }

    // Accepted-but-wrong is the worst outcome for a labelled line: the stage
    // was sure, and sure of the wrong verse.
    let wrong_accepts: Vec<&&Scored> = rows
        .iter()
        .filter(|s| s.rank != Some(1))
        .filter(|s| {
            s.hits
                .first()
                .map(|h| h.score >= gate.accept_at_or_above)
                .unwrap_or(false)
        })
        .collect();
    if !wrong_accepts.is_empty() {
        println!(
            "WRONG ACCEPTS {} — top-1 over the accept edge but not the labelled verse:",
            wrong_accepts.len()
        );
        for s in wrong_accepts {
            print_miss(s);
        }
    }

    let misses: Vec<&&Scored> = rows
        .iter()
        .filter(|s| !matches!(s.rank, Some(r) if r <= 3))
        .collect();
    if !misses.is_empty() {
        println!("misses (not in top-3):");
        for s in misses {
            print_miss(s);
        }
    }
}

fn print_miss(s: &Scored) {
    let want = if s.case.expected.is_empty() {
        "-".to_string()
    } else {
        s.case
            .expected
            .iter()
            .map(|v| v.display())
            .collect::<Vec<_>>()
            .join(" | ")
    };
    let got = s
        .hits
        .first()
        .map(|h| format!("{} {:.3}", label(h), h.score))
        .unwrap_or_else(|| "nothing".into());
    let rank = s
        .rank
        .map(|r| format!("rank {r}"))
        .unwrap_or_else(|| "absent".into());
    println!(
        "  want {want:<14} got {got:<26} {rank:<8} {:?}",
        s.case.phrase
    );
}

fn pct(part: usize, whole: usize) -> f64 {
    100.0 * part as f64 / whole.max(1) as f64
}

fn median(sorted: &[f32]) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    let mid = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        (sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        sorted[mid]
    }
}

fn load_cases(path: &PathBuf) -> Vec<Case> {
    let text = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("could not read the set at {}: {e}", path.display()));
    text.lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|line| {
            let mut cols = line.split('\t');
            let phrase = cols.next().unwrap_or("").trim().to_string();
            let expected = cols.next().unwrap_or("-").trim();
            let population = cols.next().unwrap_or("unlabelled").trim().to_string();
            let expected = if expected == "-" {
                Vec::new()
            } else {
                expected
                    .split('|')
                    .map(|id| {
                        parse_usfm(id.trim())
                            .unwrap_or_else(|| panic!("bad USFM {id:?} in {line:?}"))
                    })
                    .collect()
            };
            Case {
                phrase,
                expected,
                population,
            }
        })
        .collect()
}

/// `JHN.3.16` → the index's zero-based book number plus chapter and verse.
/// A range (`ROM.8.38-39`) resolves to its first verse, which is the one a
/// one-verse index can return.
fn parse_usfm(s: &str) -> Option<VerseRef> {
    let mut parts = s.split('.');
    let code = parts.next()?;
    let chapter: u8 = parts.next()?.parse().ok()?;
    let verse: u8 = parts.next()?.split('-').next()?.parse().ok()?;
    let book = books::CANON.iter().position(|(c, _)| *c == code)? as u8;
    Some(VerseRef {
        book,
        chapter,
        verse,
    })
}

/// "John 6:35" for a verse, "John 6:35-36" for a passage hit.
fn label(m: &Match) -> String {
    match m.end_verse {
        Some(end) => format!("{}-{end}", m.verse.display()),
        None => m.verse.display(),
    }
}
