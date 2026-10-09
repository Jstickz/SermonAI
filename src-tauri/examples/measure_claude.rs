//! The M2 Definition of Done measure: 80% of hand-labelled paraphrases in the
//! top three **by vector or Claude** (FR-14, FR-15). This runs every line of
//! the labelled set through both stages directly — no audio, no gate — so the
//! two stages can be read apart and together, and prices the calls.
//!
//! Reported per population:
//!
//! - **vector alone**: labelled verse in the vector stage's top three.
//! - **Claude alone**: labelled verse among the references Claude returns
//!   for the line (it returns zero, one or a few; the first three count).
//! - **either**: one or the other. This is the DoD line.
//! - **as gated**: what the live pipeline would do — accept the vector match
//!   at ≥ 0.80 without asking, ask Claude in the band, skip below 0.55 — so
//!   the DoD figure is shown both as a ceiling and as shipped.
//!
//! On the preaching lines, a Claude reference is a **false positive**: the
//! pipeline would raise a card for a verse nobody quoted (capped at 0.90
//! confidence, so never auto-live, but on the operator's screen).
//!
//! Credentials come from the app's own provider (keychain first), the same
//! way Start builds the stage, so the key is the one Settings tested. The
//! example never loads `.env`.
//!
//! ```text
//! cargo run --release --example measure_claude              # all lines
//! cargo run --release --example measure_claude -- --limit 3 # smoke test
//! ```
//!
//! Each line is one call; expect a few minutes and a few cents.

use std::path::PathBuf;
use std::time::Instant;

use sermonai_lib::bible::books;
use sermonai_lib::bible::reference::UsfmRef;
use sermonai_lib::credentials::local::LocalProvider;
use sermonai_lib::credentials::Credentials;
use sermonai_lib::detection::llm::{ParaphraseHit, ParaphraseStage};
use sermonai_lib::detection::vector::{SemanticSearch, VerseRef};
use sermonai_lib::llm::anthropic::AnthropicClient;
use sermonai_lib::llm::{LlmConfig, ParaphraseGate};

const TOP_K: usize = 5;

struct Case {
    phrase: String,
    expected: Vec<VerseRef>,
    population: String,
}

struct Outcome<'a> {
    case: &'a Case,
    vector_top1: f32,
    vector_rank: Option<usize>,
    claude_hits: Vec<ParaphraseHit>,
    claude_rank: Option<usize>,
    claude_err: Option<String>,
}

#[tokio::main]
async fn main() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let args: Vec<String> = std::env::args().skip(1).collect();
    let limit = args
        .iter()
        .position(|a| a == "--limit")
        .and_then(|i| args.get(i + 1))
        .and_then(|n| n.parse::<usize>().ok());
    let path = args
        .iter()
        .find(|a| a.ends_with(".tsv"))
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest.join("tests/fixtures/paraphrases.tsv"));

    let mut cases = load_cases(&path);
    if let Some(n) = limit {
        cases.truncate(n);
    }

    let search = SemanticSearch::load(&manifest.join("assets")).expect("assets");
    let llm = LlmConfig::load();
    let gate = llm.gate.clone();
    let credentials = Credentials::new(Box::new(LocalProvider::new()));
    let client = AnthropicClient::from_credentials(&credentials, llm.workspace_id.clone())
        .expect("Anthropic credential: paste and test the key in Settings → Services & keys");
    let mut stage = ParaphraseStage::new(client, llm.models.paraphrase.clone(), gate.clone());

    println!("set:    {}", path.display());
    println!("cases:  {}", cases.len());
    println!("model:  {}", stage.model());
    println!(
        "gate:   accept >= {:.2}, call >= {:.2}",
        gate.accept_at_or_above, gate.call_at_or_above
    );
    println!();

    let started = Instant::now();
    let mut outcomes: Vec<Outcome> = Vec::with_capacity(cases.len());
    for (i, case) in cases.iter().enumerate() {
        let hits = search.search(&case.phrase, TOP_K).expect("search");
        let vector_top1 = hits.first().map(|h| h.score).unwrap_or(0.0);
        let vector_rank = hits
            .iter()
            .position(|h| case.expected.contains(&h.verse))
            .map(|r| r + 1);

        let (claude_hits, claude_err) = match stage.detect(&case.phrase).await {
            Ok(h) => (h, None),
            Err(e) => {
                let msg = e.to_string();
                if stage.disabled_reason().is_some() {
                    eprintln!("\nstopped after {} calls: {msg}", i + 1);
                    break;
                }
                (Vec::new(), Some(msg))
            }
        };
        let claude_rank = claude_hits
            .iter()
            .take(3)
            .position(|h| {
                case.expected
                    .iter()
                    .any(|want| usfm_matches(&h.reference, want))
            })
            .map(|r| r + 1);

        eprint!(
            "\r{:>3}/{}  vector {:.3} {}  claude {}{}   ",
            i + 1,
            cases.len(),
            vector_top1,
            vector_rank
                .map(|r| format!("r{r}"))
                .unwrap_or_else(|| "--".into()),
            claude_rank
                .map(|r| format!("r{r}"))
                .unwrap_or_else(|| "--".into()),
            if claude_err.is_some() { " ERR" } else { "" }
        );

        outcomes.push(Outcome {
            case,
            vector_top1,
            vector_rank,
            claude_hits,
            claude_rank,
            claude_err,
        });
    }
    eprintln!();
    let elapsed = started.elapsed();

    // Populations in first-seen order.
    let mut order: Vec<&str> = Vec::new();
    for o in &outcomes {
        if !order.contains(&o.case.population.as_str()) {
            order.push(&o.case.population);
        }
    }
    for pop in &order {
        let rows: Vec<&Outcome> = outcomes
            .iter()
            .filter(|o| o.case.population == *pop)
            .collect();
        report(pop, &rows, &gate);
    }

    let usage = stage.usage();
    println!();
    println!(
        "calls {}  failed {}  input {} tok  cached {} tok  output {} tok  wall {:.0}s",
        usage.calls,
        usage.failed,
        usage.input_tokens,
        usage.cached_read_tokens,
        usage.output_tokens,
        elapsed.as_secs_f64()
    );
    println!(
        "cost  ${:.4} on {} at llm::Pricing (per call ${:.5})",
        usage.estimated_cost_usd(stage.model()),
        stage.model(),
        usage.estimated_cost_usd(stage.model()) / usage.calls.max(1) as f64
    );

    if let Some(rows) = order.iter().find(|p| **p == "paraphrase").map(|p| {
        outcomes
            .iter()
            .filter(|o| o.case.population == *p)
            .collect::<Vec<_>>()
    }) {
        let n = rows.len();
        let either = rows
            .iter()
            .filter(|o| in_top3(o.vector_rank) || o.claude_rank.is_some())
            .count();
        let gated = rows.iter().filter(|o| gated_hit(o, &gate)).count();
        let pct = |k: usize| 100.0 * k as f64 / n.max(1) as f64;
        println!();
        println!(
            "M2 DoD: {either}/{n} = {:.1}% in top-3 by vector or Claude (ceiling, every line asked); \
             {gated}/{n} = {:.1}% as gated in the shipped pipeline — target 80%: {}",
            pct(either),
            pct(gated),
            if pct(gated) >= 80.0 {
                "MET as shipped"
            } else if pct(either) >= 80.0 {
                "met only if the gate is widened"
            } else {
                "NOT MET"
            }
        );
    }
}

fn in_top3(rank: Option<usize>) -> bool {
    matches!(rank, Some(r) if r <= 3)
}

/// What the live pipeline does with this line, given the vector score.
fn gated_hit(o: &Outcome, gate: &ParaphraseGate) -> bool {
    if o.vector_top1 >= gate.accept_at_or_above {
        in_top3(o.vector_rank)
    } else if o.vector_top1 >= gate.call_at_or_above {
        // In the band the vector candidates are still in the top three for
        // the operator; Claude's answer is added to them.
        in_top3(o.vector_rank) || o.claude_rank.is_some()
    } else {
        false
    }
}

fn report(pop: &str, rows: &[&Outcome], gate: &ParaphraseGate) {
    let n = rows.len();
    let labelled = rows.iter().filter(|o| !o.case.expected.is_empty()).count();
    let errors = rows.iter().filter(|o| o.claude_err.is_some()).count();
    println!(
        "== {pop} (n={n}{}) ==",
        if errors > 0 {
            format!(", {errors} call errors")
        } else {
            String::new()
        }
    );

    if labelled > 0 {
        let v = rows.iter().filter(|o| in_top3(o.vector_rank)).count();
        let c = rows.iter().filter(|o| o.claude_rank.is_some()).count();
        let e = rows
            .iter()
            .filter(|o| in_top3(o.vector_rank) || o.claude_rank.is_some())
            .count();
        let g = rows.iter().filter(|o| gated_hit(o, gate)).count();
        let pct = |k: usize| 100.0 * k as f64 / labelled as f64;
        println!(
            "catch@3  vector {v:>3}/{labelled} ({:.0}%)   claude {c:>3}/{labelled} ({:.0}%)   either {e:>3}/{labelled} ({:.0}%)   as gated {g:>3}/{labelled} ({:.0}%)",
            pct(v),
            pct(c),
            pct(e),
            pct(g)
        );
        let claude_only: Vec<&&Outcome> = rows
            .iter()
            .filter(|o| !in_top3(o.vector_rank) && o.claude_rank.is_some())
            .collect();
        let vector_only = rows
            .iter()
            .filter(|o| in_top3(o.vector_rank) && o.claude_rank.is_none())
            .count();
        println!(
            "         claude recovers {} the vector stage missed; vector holds {vector_only} Claude missed",
            claude_only.len()
        );
        let neither: Vec<&&Outcome> = rows
            .iter()
            .filter(|o| !in_top3(o.vector_rank) && o.claude_rank.is_none())
            .collect();
        if !neither.is_empty() {
            println!("neither stage (not in top-3 by either):");
            for o in neither {
                println!(
                    "  want {:<22} vector {:.3}  claude said {:<28} {:?}",
                    o.case
                        .expected
                        .iter()
                        .map(|v| v.display())
                        .collect::<Vec<_>>()
                        .join(" | "),
                    o.vector_top1,
                    describe(&o.claude_hits, o.claude_err.as_deref()),
                    o.case.phrase
                );
            }
        }
        return;
    }

    // Lines that quote nothing: any reference is a false positive.
    let vector_false = rows
        .iter()
        .filter(|o| o.vector_top1 >= gate.accept_at_or_above)
        .count();
    let claude_false: Vec<&&Outcome> = rows.iter().filter(|o| !o.claude_hits.is_empty()).collect();
    println!(
        "false positives  vector accepts {vector_false}   claude references {}",
        claude_false.len()
    );
    for o in claude_false {
        println!(
            "  claude {:<28} {:?}",
            describe(&o.claude_hits, None),
            o.case.phrase
        );
    }
}

fn describe(hits: &[ParaphraseHit], err: Option<&str>) -> String {
    if let Some(e) = err {
        return format!("ERR {}", e.chars().take(40).collect::<String>());
    }
    if hits.is_empty() {
        return "[]".into();
    }
    hits.iter()
        .take(3)
        .map(|h| format!("{} {:.2}", display_usfm(&h.reference), h.confidence))
        .collect::<Vec<_>>()
        .join(", ")
}

fn display_usfm(r: &UsfmRef) -> String {
    let name = books::name(r.book).unwrap_or("?");
    match r.end_verse {
        Some(end) => format!("{name} {}:{}-{end}", r.chapter, r.verse),
        None => format!("{name} {}:{}", r.chapter, r.verse),
    }
}

fn usfm_matches(hit: &UsfmRef, want: &VerseRef) -> bool {
    hit.book == want.book
        && hit.chapter == u16::from(want.chapter)
        && (hit.verse == u16::from(want.verse)
            || hit
                .end_verse
                .map(|end| hit.verse <= u16::from(want.verse) && u16::from(want.verse) <= end)
                .unwrap_or(false))
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
