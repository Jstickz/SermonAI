//! The paraphrase stage: Claude, asked only when the vector stage is unsure
//! (FR-14, amended 24 September 2026).
//!
//! ## Not a timer
//!
//! FR-14 originally fired this ten seconds after the last regex hit, whether
//! or not anything in those ten seconds resembled scripture. It is now gated
//! on the vector stage's top score (`llm::ParaphraseGate`): high enough and
//! the vector answer stands on its own; low enough and the preacher is simply
//! preaching; in between, Claude is asked. Measured bands are in
//! `docs/testing/vector-score-bands.md`. A cooldown then bounds cost inside
//! the band, because FR-14 sends the whole 60-second buffer and one call
//! already covers many utterances.
//!
//! ## What Claude is asked, and what is done with the answer
//!
//! The rolling buffer, and a request for strict JSON: references the speaker
//! is quoting or paraphrasing, each with the words that suggested it and a
//! confidence. Every reference then goes through the same parser and chapter
//! check the regex stage uses — a model can name a verse that does not exist
//! as fluently as one that does, and a verse on the projector cannot be
//! recalled. Anything that fails to parse is dropped, not shown.
//!
//! ## Cost is counted, not assumed
//!
//! Every call's tokens are accumulated into [`Usage`] and priced from
//! `llm::Pricing`, so the stop log says what this service cost and how many
//! calls it took. That number, from real services, is what sets the gate
//! thresholds properly; the ones shipped came from eight paraphrases.

use std::time::{Duration, Instant};

use serde::Deserialize;

use crate::bible::books;
use crate::bible::reference::{self, UsfmRef};
use crate::error::Result;
use crate::llm::anthropic::AnthropicClient;
use crate::llm::{ParaphraseGate, Pricing};

/// PRD §18.1 gives the LLM stage 1.5 s p95 and 3 s p99. Waiting longer than
/// the p99 produces a candidate for words the preacher has moved on from.
pub const DEADLINE: Duration = Duration::from_secs(3);

/// Bounds the reply and therefore the cost. A window of sixty seconds rarely
/// carries more than three or four references; ten fit comfortably.
const MAX_REPLY_TOKENS: u32 = 300;

/// What the gate decided about one utterance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// The vector stage's top match is convincing on its own.
    AcceptVector,
    /// Unsure: ask Claude.
    Call,
    /// Inside the cooldown since the last call: would mostly re-read the
    /// same buffer.
    Cooldown,
    /// Nothing here resembles scripture.
    Skip,
}

/// The gate as a pure decision, so it can be tested without a network.
pub fn decide(
    gate: &ParaphraseGate,
    vector_top1: f32,
    since_last_call: Option<Duration>,
) -> Decision {
    if vector_top1 >= gate.accept_at_or_above {
        return Decision::AcceptVector;
    }
    if vector_top1 < gate.call_at_or_above {
        return Decision::Skip;
    }
    match since_last_call {
        Some(since) if since < Duration::from_secs(gate.cooldown_secs) => Decision::Cooldown,
        _ => Decision::Call,
    }
}

/// One reference Claude proposed and the parser accepted.
#[derive(Debug, Clone, PartialEq)]
pub struct ParaphraseHit {
    pub reference: UsfmRef,
    /// The transcript words that suggested it, as Claude quoted them.
    pub evidence: String,
    /// Claude's own 0–1 confidence, clamped.
    pub confidence: f32,
}

/// Calls and tokens so far, for the stop log and the cost estimate.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Usage {
    /// Attempts. `calls - failed` answered.
    pub calls: u64,
    /// Attempts that returned an error. Tokens are counted only for answers,
    /// so a run of failures shows as calls with zero tokens — which is what
    /// the first live service logged: 57 calls, 0 tokens, all 400s.
    pub failed: u64,
    pub input_tokens: u64,
    pub cached_read_tokens: u64,
    pub output_tokens: u64,
    /// Gate decisions, so the log can say how often each branch was taken.
    pub accepted_vector: u64,
    pub skipped: u64,
    pub cooled_down: u64,
}

impl Usage {
    pub fn estimated_cost_usd(&self, model: &str) -> f64 {
        Pricing::for_model(model).cost(
            self.input_tokens,
            self.cached_read_tokens,
            self.output_tokens,
        )
    }
}

pub struct ParaphraseStage {
    client: AnthropicClient,
    model: String,
    gate: ParaphraseGate,
    last_call: Option<Instant>,
    usage: Usage,
    /// Set after a configuration error — a rejected key, a missing workspace
    /// header. Retrying every ten seconds for a whole service cannot fix
    /// those, and the first live run did exactly that 57 times. Once set the
    /// stage answers no more calls until the next Start.
    disabled: Option<String>,
}

impl ParaphraseStage {
    pub fn new(client: AnthropicClient, model: impl Into<String>, gate: ParaphraseGate) -> Self {
        Self {
            client,
            model: model.into(),
            gate,
            last_call: None,
            usage: Usage::default(),
            disabled: None,
        }
    }

    /// Why this stage has stopped calling, if it has.
    pub fn disabled_reason(&self) -> Option<&str> {
        self.disabled.as_deref()
    }

    /// Apply the gate to one utterance's vector score, and count the outcome.
    pub fn consider(&mut self, vector_top1: f32) -> Decision {
        let decision = decide(&self.gate, vector_top1, self.last_call.map(|t| t.elapsed()));
        match decision {
            Decision::AcceptVector => self.usage.accepted_vector += 1,
            Decision::Skip => self.usage.skipped += 1,
            Decision::Cooldown => self.usage.cooled_down += 1,
            Decision::Call => {}
        }
        decision
    }

    /// Ask Claude about the rolling buffer. Call only after [`consider`]
    /// returned [`Decision::Call`]; this does not re-check the gate.
    pub async fn detect(&mut self, rolling_buffer: &str) -> Result<Vec<ParaphraseHit>> {
        if let Some(reason) = &self.disabled {
            return Err(crate::error::Error::Detection(reason.clone()));
        }
        self.last_call = Some(Instant::now());
        self.usage.calls += 1;

        let completion = match self
            .client
            .message(
                &self.model,
                SYSTEM_PROMPT,
                rolling_buffer,
                MAX_REPLY_TOKENS,
                DEADLINE,
            )
            .await
        {
            Ok(c) => c,
            Err(err) => {
                self.usage.failed += 1;
                // A configuration error will not change by asking again.
                // Stop for the service and say so once; the operator fixes
                // it in Settings and the next Start tries afresh.
                if matches!(err, crate::error::Error::Config(_)) {
                    tracing::warn!(%err, "paraphrase stage disabled for this service after a configuration error");
                    self.disabled = Some(err.to_string());
                }
                return Err(err);
            }
        };

        self.usage.input_tokens += completion.input_tokens;
        self.usage.cached_read_tokens += completion.cached_read_tokens;
        self.usage.output_tokens += completion.output_tokens;

        Ok(parse_reply(&completion.text))
    }

    pub fn usage(&self) -> Usage {
        self.usage
    }

    pub fn model(&self) -> &str {
        &self.model
    }
}

/// The instruction Claude sees every call. Stable text, so the vendor caches
/// it (see `AnthropicClient::message`) and each call pays the cached-read
/// price for it rather than the input price.
const SYSTEM_PROMPT: &str = "\
You listen to a live sermon transcript and identify Bible verses the speaker is \
quoting or closely paraphrasing. The transcript is speech recognition output: \
punctuation is unreliable and words may be misheard.

Reply with a JSON array only, no prose. Each element: \
{\"reference\": \"Book Chapter:Verse\", \"evidence\": \"the transcript words that suggested it\", \"confidence\": 0.0-1.0}. \
Use full book names and Arabic numerals (\"Jeremiah 29:11\", \"1 Corinthians 13:4\"). \
Include a reference only when the speaker is quoting or paraphrasing the verse's \
words, not merely discussing its theme. If nothing is quoted or paraphrased, reply [].";

#[derive(Deserialize)]
struct Proposed {
    reference: String,
    #[serde(default)]
    evidence: String,
    #[serde(default)]
    confidence: f32,
}

/// Read Claude's reply, keeping only references the canon can actually hold.
///
/// Tolerant of a reply wrapped in a code fence or prose, because models do
/// that despite instructions, and intolerant of a reference that does not
/// parse or names a chapter the book lacks, because the projector is.
fn parse_reply(text: &str) -> Vec<ParaphraseHit> {
    let Some(json) = extract_json_array(text) else {
        return Vec::new();
    };
    let Ok(proposed) = serde_json::from_str::<Vec<Proposed>>(json) else {
        return Vec::new();
    };

    proposed
        .into_iter()
        .filter_map(|p| {
            let reference = reference::parse(&p.reference).ok()?;
            let code = reference.code();
            let chapters = books::chapters(code)?;
            if chapters > 1 && reference.chapter > u16::from(chapters) {
                return None;
            }
            Some(ParaphraseHit {
                reference,
                evidence: p.evidence,
                confidence: p.confidence.clamp(0.0, 1.0),
            })
        })
        .collect()
}

/// The first `[ ... ]` in the text, so a fenced or prefaced reply still reads.
fn extract_json_array(text: &str) -> Option<&str> {
    let start = text.find('[')?;
    let end = text.rfind(']')?;
    (end >= start).then(|| &text[start..=end])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gate() -> ParaphraseGate {
        ParaphraseGate::default()
    }

    #[test]
    fn the_gate_follows_the_measured_bands() {
        // From docs/testing/vector-score-bands.md. Every verbatim quote
        // scored at least 0.808; no unquoted sentence scored above 0.696.
        assert_eq!(decide(&gate(), 0.997, None), Decision::AcceptVector);
        assert_eq!(decide(&gate(), 0.808, None), Decision::AcceptVector);
        // The overlap: a paraphrase at 0.699 and preaching at 0.696 look
        // alike to the vector stage. That is what the call is for.
        assert_eq!(decide(&gate(), 0.699, None), Decision::Call);
        assert_eq!(decide(&gate(), 0.696, None), Decision::Call);
        // Plainly not scripture.
        assert_eq!(decide(&gate(), 0.313, None), Decision::Skip);
        // The known cost of the floor: the NIV-worded Jeremiah 29:11 at 0.339
        // is skipped. Pinned so raising the floor is a decision, not a drift.
        assert_eq!(decide(&gate(), 0.339, None), Decision::Skip);
    }

    #[test]
    fn the_cooldown_holds_inside_the_band_only() {
        let recent = Some(Duration::from_secs(4));
        let stale = Some(Duration::from_secs(11));
        assert_eq!(decide(&gate(), 0.70, recent), Decision::Cooldown);
        assert_eq!(decide(&gate(), 0.70, stale), Decision::Call);
        // Accept and skip are not delayed by a cooldown: neither costs a call.
        assert_eq!(decide(&gate(), 0.90, recent), Decision::AcceptVector);
        assert_eq!(decide(&gate(), 0.40, recent), Decision::Skip);
    }

    #[test]
    fn a_clean_reply_becomes_references() {
        let hits = parse_reply(
            r#"[{"reference":"Jeremiah 29:11","evidence":"plans to prosper you","confidence":0.9},
                {"reference":"1 Corinthians 13:4","evidence":"love is patient","confidence":0.7}]"#,
        );
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].reference.passage_id(), "JER.29.11");
        assert_eq!(hits[1].reference.passage_id(), "1CO.13.4");
        assert!((hits[0].confidence - 0.9).abs() < 1e-6);
    }

    #[test]
    fn a_verse_the_canon_cannot_hold_is_dropped_not_shown() {
        // A model can name "John 30:1" as fluently as "John 3:16". The
        // projector cannot tell them apart, so this stage has to.
        let hits = parse_reply(
            r#"[{"reference":"John 30:1","confidence":0.8},
                {"reference":"Hezekiah 2:3","confidence":0.8},
                {"reference":"John 3:16","confidence":0.8}]"#,
        );
        let ids: Vec<String> = hits.iter().map(|h| h.reference.passage_id()).collect();
        assert_eq!(ids, ["JHN.3.16"]);
    }

    #[test]
    fn prose_and_fences_around_the_array_are_tolerated() {
        let hits = parse_reply(
            "Here you go:\n```json\n[{\"reference\":\"Psalm 23:1\",\"confidence\":1.2}]\n```",
        );
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].reference.passage_id(), "PSA.23.1");
        // Confidence is clamped, not trusted.
        assert_eq!(hits[0].confidence, 1.0);
    }

    #[test]
    fn nothing_and_nonsense_both_yield_nothing_rather_than_a_panic() {
        assert!(parse_reply("[]").is_empty());
        assert!(parse_reply("").is_empty());
        assert!(parse_reply("I could not find any references.").is_empty());
        assert!(parse_reply("[{\"not\": \"the shape\"}]").is_empty());
        assert!(parse_reply("[{\"reference\": 42}]").is_empty());
    }

    #[test]
    fn cost_is_estimated_from_counted_tokens() {
        let usage = Usage {
            calls: 40,
            input_tokens: 40 * 600,
            cached_read_tokens: 40 * 380,
            output_tokens: 40 * 60,
            ..Default::default()
        };
        // Haiku: 24,000 in at $1, 15,200 cached at $0.10, 2,400 out at $5.
        let usd = usage.estimated_cost_usd("claude-haiku-4-5-20251001");
        assert!((usd - (0.024 + 0.00152 + 0.012)).abs() < 1e-9, "{usd}");
    }
}
