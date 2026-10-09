//! The detection pipeline: three stages in order, one confidence per
//! candidate (FR-16), and a queue that never drops (FR-17). PRD §10.2 layer 3,
//! §13.5, §18.1.
//!
//! ## What it is, and is not
//!
//! Pure and synchronous. Text goes in, candidates come out, and the clock is a
//! parameter — so a full service can be replayed in a test in microseconds,
//! and so nothing here can block the transcript event path. The one network
//! stage, Claude, is not called from here: when the gate says the vector
//! stage is unsure, the pipeline hands back a [`ParaphraseRequest`] and the
//! caller makes the call and feeds the answer to [`Pipeline::on_paraphrase`].
//!
//! ## Two-stage detection (PRD §18.1, standing decision)
//!
//! The regex stage runs on **interim** text and raises **provisional**
//! candidates: shown in staging marked unconfirmed, never projectable. When the
//! utterance settles, the regex stage runs on the **final** text; a provisional
//! candidate the settled text still supports is **confirmed** under the same
//! id, and one it does not support is **withdrawn silently** — interim results
//! are revisions, not mistakes, and a panel that announced every retraction
//! would teach the operator to ignore it.
//!
//! The vector and Claude stages run on settled text only. They are the
//! semantic stages, and a semantic match on a half-sentence is a guess.
//!
//! ## The window
//!
//! A reference arrives split across settled utterances — `Jeremiah.` /
//! `Chapter 29.` / `Verse 11,` was three finals in the measured transcript —
//! so the regex stage is given the tail of the rolling buffer plus the current
//! text, not the current text alone. [`window`] builds it.
//!
//! ## Confidence (FR-16)
//!
//! One 0–1 figure per candidate, comparable across stages, so the card and the
//! auto-live threshold (§13.5, 90%) mean the same thing whichever stage spoke:
//!
//! - **regex** — the form's own confidence (`regex::Form::confidence`): a
//!   written colon form 0.97, a bare "Mark 3" 0.55.
//! - **vector** — the cosine score, plus 0.05 when the runner-up is more than
//!   0.15 behind (a clear winner, measured to separate real hits from flat
//!   top-5s), capped at 0.95 so no semantic match outranks a direct one.
//! - **llm** — Claude's self-reported confidence scaled by 0.9 and capped at
//!   0.90. Self-reports are not calibrated; the cap keeps a paraphrase below
//!   the auto-live line by default, so it always passes an operator.
//!
//! ## The queue (FR-17)
//!
//! Every candidate is pushed to an unbounded queue and stays there until the
//! caller drains it. Ten references in twenty seconds are ten candidates, in
//! order. Nothing is coalesced away; deduplication is by *passage within a
//! minute*, so the same verse said twice in ten seconds is one card and said
//! again five minutes later is a new one.

use std::collections::{HashMap, VecDeque};

use serde::{Deserialize, Serialize};

use super::llm::{decide, Decision, ParaphraseHit};
use super::regex::{self, Form};
use super::vector::{Match, SearchOptions, SemanticSearch, SynonymMode};
use crate::bible::reference::UsfmRef;
use crate::db::models::{DetectionSource, Verse};
use crate::llm::ParaphraseGate;

/// How many settled words precede the current text in the regex window.
/// Forty is about fifteen seconds of preaching — enough to hold "Jeremiah.
/// Chapter 29." while "Verse 11" arrives, not enough to re-find a reference
/// from a minute ago.
pub const WINDOW_WORDS: usize = 40;

/// A passage said again inside this many milliseconds is the same detection.
/// Beyond it, the preacher returning to a verse is a new card.
pub const DEDUPE_MS: u64 = 60_000;

/// What the pipeline hands the frontend and, in M4, the database.
///
/// Mirrored by `Detection` in `src/lib/types.ts`. `verse` is `None` until the
/// Bible cache (M2 deliverable 5) fills it; the card shows the reference
/// meanwhile. A **provisional** candidate has no verse either way — it is not
/// projectable, so fetching text for it would be spend without display.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub id: u64,
    /// USFM, e.g. `JER.29.11`. The key for deduplication and the cache.
    pub passage_id: String,
    /// Operator-facing, e.g. "Jeremiah 29:11".
    pub reference: String,
    pub source: DetectionSource,
    pub confidence: f32,
    /// Raised from interim text; never projectable. Cleared under the same id
    /// when the settled text confirms it.
    pub provisional: bool,
    /// The transcript words that suggested it, where a stage knows them.
    pub evidence: Option<String>,
    pub detected_at_ms: u64,
    pub verse: Option<Verse>,
}

/// The pipeline wants Claude asked about this buffer. The caller owns the
/// network and the deadline; the answer comes back through
/// [`Pipeline::on_paraphrase`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParaphraseRequest {
    pub rolling_buffer: String,
    pub requested_at_ms: u64,
}

/// How often each branch was taken. For the stop log, and for setting the
/// gate thresholds from real services rather than eight sentences.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub provisional_raised: u64,
    pub confirmed: u64,
    pub withdrawn: u64,
    pub regex_hits: u64,
    pub vector_accepted: u64,
    pub paraphrase_requested: u64,
    pub paraphrase_hits: u64,
    pub skipped: u64,
    pub cooled_down: u64,
    pub deduplicated: u64,
    /// Finals with no direct hit that arrived before the vector index had
    /// finished loading (it joins in the background after the window opens).
    /// Regex ran; the semantic stages did not. Says what the warm-up cost.
    pub finals_before_vector_ready: u64,
}

struct Recent {
    id: u64,
    provisional: bool,
    seen_at_ms: u64,
}

pub struct Pipeline {
    vector: Option<SemanticSearch>,
    gate: ParaphraseGate,
    next_id: u64,
    recent: HashMap<String, Recent>,
    queue: VecDeque<Candidate>,
    withdrawn: Vec<u64>,
    last_call_ms: Option<u64>,
    stats: Stats,
}

impl Pipeline {
    /// `vector` is optional so the regex and queue behaviour can be tested
    /// without the 15 MB of assets; the app always passes it.
    pub fn new(vector: Option<SemanticSearch>, gate: ParaphraseGate) -> Self {
        Self {
            vector,
            gate,
            next_id: 1,
            recent: HashMap::new(),
            queue: VecDeque::new(),
            withdrawn: Vec::new(),
            last_call_ms: None,
            stats: Stats::default(),
        }
    }

    /// Interim text arrived: raise provisional candidates for any reference
    /// the regex stage finds that is not already known.
    pub fn on_interim(&mut self, rolling_buffer: &str, interim: &str, now_ms: u64) {
        self.expire(now_ms);
        let text = window(rolling_buffer, interim);
        for hit in regex::scan(&text) {
            let passage = hit.reference.passage_id();
            if self.recent.contains_key(&passage) {
                self.stats.deduplicated += 1;
                continue;
            }
            self.stats.provisional_raised += 1;
            self.push(
                &hit.reference,
                DetectionSource::Regex,
                hit.form.confidence(),
                true,
                None,
                now_ms,
            );
        }
    }

    /// An utterance settled. Confirm or withdraw provisional candidates, run
    /// the regex stage on the settled text, and — when nothing direct was
    /// found — the vector stage, which may ask for Claude.
    ///
    /// `rolling_buffer` must already include `final_text`.
    pub fn on_final(
        &mut self,
        rolling_buffer: &str,
        final_text: &str,
        now_ms: u64,
    ) -> Option<ParaphraseRequest> {
        self.expire(now_ms);

        // Regex over the window, which holds this final and the tail before
        // it. Anything found is confirmed; a provisional candidate found again
        // firms up under its own id.
        let text = window(rolling_buffer, "");
        let mut found_now: Vec<String> = Vec::new();
        let mut direct_hit_in_final = false;

        for hit in regex::scan(&text) {
            let passage = hit.reference.passage_id();
            found_now.push(passage.clone());
            self.stats.regex_hits += 1;

            match self.recent.get_mut(&passage) {
                Some(recent) if recent.provisional => {
                    // Firm up: same id, provisional cleared. The card that was
                    // marked unconfirmed becomes projectable.
                    recent.provisional = false;
                    recent.seen_at_ms = now_ms;
                    let id = recent.id;
                    self.stats.confirmed += 1;
                    self.queue.push_back(candidate(
                        id,
                        &hit.reference,
                        DetectionSource::Regex,
                        hit.form.confidence(),
                        false,
                        None,
                        now_ms,
                    ));
                    direct_hit_in_final = true;
                }
                Some(recent) => {
                    recent.seen_at_ms = now_ms;
                    self.stats.deduplicated += 1;
                }
                None => {
                    self.stats.confirmed += 1;
                    self.push(
                        &hit.reference,
                        DetectionSource::Regex,
                        hit.form.confidence(),
                        false,
                        None,
                        now_ms,
                    );
                    direct_hit_in_final = true;
                }
            }
        }

        // Provisional candidates the settled text no longer supports are
        // withdrawn — silently, by id. Interim results are revisions.
        let stale: Vec<(String, u64)> = self
            .recent
            .iter()
            .filter(|(passage, r)| r.provisional && !found_now.contains(passage))
            .map(|(passage, r)| (passage.clone(), r.id))
            .collect();
        for (passage, id) in stale {
            self.recent.remove(&passage);
            self.withdrawn.push(id);
            self.stats.withdrawn += 1;
        }

        if direct_hit_in_final {
            return None;
        }

        // Nothing direct in this utterance: ask the semantic stages.
        let Some(vector) = self.vector.as_ref() else {
            self.stats.finals_before_vector_ready += 1;
            return None;
        };
        // Searched with the KJV-vocabulary variants blended into one query
        // (encoder upgrade Phase 1, measured 9 Oct 2026: paraphrase catch@3
        // 70% → 79% on the labelled set, preaching false accepts still 0, one
        // search either way). Neighbouring-verse passages were measured too
        // and not kept: +0.6% alone, −1.2% combined.
        let options = SearchOptions {
            synonyms: SynonymMode::Blend,
            neighbours: false,
        };
        let matches = vector.search_with(final_text, 5, &options).ok()?;
        let top1 = matches.first()?.score;

        let since_last = self
            .last_call_ms
            .map(|t| std::time::Duration::from_millis(now_ms.saturating_sub(t)));
        let decision = decide(&self.gate, top1, since_last);
        // One line per scored final so a real service yields the score
        // distribution the gate thresholds are tuned from (FR-14). Scores and
        // the decision only: the words spoken do not belong in a log.
        tracing::info!(
            top1 = format_args!("{top1:.3}"),
            top2 = format_args!("{:.3}", matches.get(1).map(|m| m.score).unwrap_or(0.0)),
            words = final_text.split_whitespace().count(),
            since_last_call_ms = since_last.map(|d| d.as_millis() as u64),
            decision = match decision {
                Decision::AcceptVector => "accept",
                Decision::Call => "call",
                Decision::Cooldown => "cooldown",
                Decision::Skip => "skip",
            },
            "vector stage scored a final"
        );
        match decision {
            Decision::AcceptVector => {
                self.stats.vector_accepted += 1;
                let reference = to_usfm(&matches[0]);
                let confidence = vector_confidence(&matches);
                if self.recent.contains_key(&reference.passage_id()) {
                    self.stats.deduplicated += 1;
                } else {
                    self.push(
                        &reference,
                        DetectionSource::Vector,
                        confidence,
                        false,
                        None,
                        now_ms,
                    );
                }
                None
            }
            Decision::Call => {
                self.stats.paraphrase_requested += 1;
                self.last_call_ms = Some(now_ms);
                Some(ParaphraseRequest {
                    rolling_buffer: rolling_buffer.to_string(),
                    requested_at_ms: now_ms,
                })
            }
            Decision::Cooldown => {
                self.stats.cooled_down += 1;
                None
            }
            Decision::Skip => {
                self.stats.skipped += 1;
                None
            }
        }
    }

    /// Claude answered a [`ParaphraseRequest`].
    pub fn on_paraphrase(&mut self, hits: Vec<ParaphraseHit>, now_ms: u64) {
        self.expire(now_ms);
        for hit in hits {
            if self.recent.contains_key(&hit.reference.passage_id()) {
                self.stats.deduplicated += 1;
                continue;
            }
            self.stats.paraphrase_hits += 1;
            let evidence = (!hit.evidence.is_empty()).then_some(hit.evidence);
            self.push(
                &hit.reference,
                DetectionSource::Llm,
                llm_confidence(hit.confidence),
                false,
                evidence,
                now_ms,
            );
        }
    }

    /// Everything raised, confirmed or found since the last drain, in order.
    /// Never truncated (FR-17).
    pub fn drain(&mut self) -> Vec<Candidate> {
        self.queue.drain(..).collect()
    }

    /// Ids of provisional candidates withdrawn since the last call.
    pub fn take_withdrawn(&mut self) -> Vec<u64> {
        std::mem::take(&mut self.withdrawn)
    }

    pub fn stats(&self) -> Stats {
        self.stats
    }

    pub fn pending(&self) -> usize {
        self.queue.len()
    }

    fn push(
        &mut self,
        reference: &UsfmRef,
        source: DetectionSource,
        confidence: f32,
        provisional: bool,
        evidence: Option<String>,
        now_ms: u64,
    ) {
        let id = self.next_id;
        self.next_id += 1;
        self.recent.insert(
            reference.passage_id(),
            Recent {
                id,
                provisional,
                seen_at_ms: now_ms,
            },
        );
        self.queue.push_back(candidate(
            id,
            reference,
            source,
            confidence,
            provisional,
            evidence,
            now_ms,
        ));
    }

    /// Forget passages not seen for a minute, so a verse returned to later in
    /// the sermon is a new card rather than a suppressed duplicate.
    fn expire(&mut self, now_ms: u64) {
        // A provisional entry that ages out is withdrawn, not forgotten. The
        // first run in front of an operator left a dimmed card on screen for
        // good: its interim never got a final, `retain` dropped the entry, and
        // nothing told the window.
        let mut expired = Vec::new();
        self.recent.retain(|_, r| {
            let keep = now_ms.saturating_sub(r.seen_at_ms) < DEDUPE_MS;
            if !keep && r.provisional {
                expired.push(r.id);
            }
            keep
        });
        for id in expired {
            self.withdrawn.push(id);
            self.stats.withdrawn += 1;
        }
    }

    /// The transcript stream closed, or capture stopped. Every provisional
    /// candidate is withdrawn: no final is coming to confirm it.
    pub fn on_closed(&mut self) {
        let orphaned: Vec<(String, u64)> = self
            .recent
            .iter()
            .filter(|(_, r)| r.provisional)
            .map(|(p, r)| (p.clone(), r.id))
            .collect();
        for (passage, id) in orphaned {
            self.recent.remove(&passage);
            self.withdrawn.push(id);
            self.stats.withdrawn += 1;
        }
    }

    /// The vector index finished loading. It is loaded off the startup path
    /// so the window opens first; until this is called the regex stage runs
    /// alone and `finals_before_vector_ready` counts what that cost.
    pub fn attach_vector(&mut self, search: SemanticSearch) {
        self.vector = Some(search);
    }

    pub fn vector_ready(&self) -> bool {
        self.vector.is_some()
    }
}

fn candidate(
    id: u64,
    reference: &UsfmRef,
    source: DetectionSource,
    confidence: f32,
    provisional: bool,
    evidence: Option<String>,
    now_ms: u64,
) -> Candidate {
    Candidate {
        id,
        passage_id: reference.passage_id(),
        reference: reference.display(),
        source,
        confidence: confidence.clamp(0.0, 1.0),
        provisional,
        evidence,
        detected_at_ms: now_ms,
        verse: None,
    }
}

/// The regex window: the last [`WINDOW_WORDS`] settled words, then the text
/// in hand.
pub fn window(rolling_buffer: &str, current: &str) -> String {
    let words: Vec<&str> = rolling_buffer.split_whitespace().collect();
    let start = words.len().saturating_sub(WINDOW_WORDS);
    let mut out = words[start..].join(" ");
    if !current.is_empty() {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(current);
    }
    out
}

/// Cosine score, with a bonus for a clear winner, capped below any direct
/// reference. See the module note.
fn vector_confidence(matches: &[Match]) -> f32 {
    let top1 = matches.first().map(|m| m.score).unwrap_or(0.0);
    let top2 = matches.get(1).map(|m| m.score).unwrap_or(0.0);
    let bonus = if top1 - top2 > 0.15 { 0.05 } else { 0.0 };
    (top1 + bonus).min(0.95)
}

/// Claude's self-report, discounted and capped below the auto-live line.
fn llm_confidence(reported: f32) -> f32 {
    (reported.clamp(0.0, 1.0) * 0.9).min(0.90)
}

fn to_usfm(m: &Match) -> UsfmRef {
    UsfmRef {
        book: m.verse.book,
        chapter: u16::from(m.verse.chapter),
        verse: u16::from(m.verse.verse),
        end_verse: None,
    }
}

// Kept in scope for the regex confidence table in the module note.
#[allow(dead_code)]
const _: fn(Form) -> f32 = Form::confidence;

#[cfg(test)]
mod tests {
    use super::*;

    fn pipeline() -> Pipeline {
        Pipeline::new(None, ParaphraseGate::default())
    }

    #[test]
    fn ten_references_in_twenty_seconds_are_ten_cards_in_order() {
        // The M2 DoD line, as a unit: nothing lost, order kept.
        let refs = [
            "John 3:16",
            "Romans 8:28",
            "Jeremiah 29:11",
            "Psalm 23:1",
            "Genesis 1:1",
            "Isaiah 53:5",
            "Matthew 28:19",
            "Philippians 4:13",
            "Hebrews 11:1",
            "Revelation 21:4",
        ];
        let mut p = pipeline();
        let mut rolling = String::new();
        for (i, r) in refs.iter().enumerate() {
            let text = format!("turn with me to {r} church");
            rolling.push(' ');
            rolling.push_str(&text);
            p.on_final(&rolling, &text, (i as u64) * 2_000);
        }
        let cards = p.drain();
        let got: Vec<&str> = cards.iter().map(|c| c.reference.as_str()).collect();
        assert_eq!(got.len(), 10);
        assert_eq!(got[0], "John 3:16");
        assert_eq!(got[9], "Revelation 21:4");
        assert!(cards.iter().all(|c| !c.provisional));
        assert!(
            cards.windows(2).all(|w| w[0].id < w[1].id),
            "ids must rise with order"
        );
        assert_eq!(
            p.pending(),
            0,
            "drain empties the queue and nothing was dropped"
        );
    }

    #[test]
    fn a_provisional_candidate_firms_up_under_the_same_id() {
        // The standing decision. Interim text raises it unconfirmed; the
        // settled text confirms it; the operator sees one card change state,
        // not two cards.
        let mut p = pipeline();
        p.on_interim("", "turn to John 3:16", 1_000);
        let first = p.drain();
        assert_eq!(first.len(), 1);
        assert!(first[0].provisional);
        let id = first[0].id;

        let rolling = "turn to John 3:16 for a moment";
        let request = p.on_final(rolling, rolling, 2_500);
        assert!(request.is_none(), "a direct hit does not ask Claude");
        let confirmed = p.drain();
        assert_eq!(confirmed.len(), 1);
        assert_eq!(confirmed[0].id, id);
        assert!(!confirmed[0].provisional);
        assert!(p.take_withdrawn().is_empty());
        assert_eq!(p.stats().confirmed, 1);
    }

    #[test]
    fn a_provisional_candidate_the_settled_text_drops_is_withdrawn_silently() {
        // "...if you will to join" became "...to John chapter three" in
        // testing — and the reverse happens too. The interim hit must go
        // without a card announcing that it went.
        let mut p = pipeline();
        p.on_interim("", "he said in John 3", 1_000);
        let raised = p.drain();
        assert_eq!(raised.len(), 1);
        let id = raised[0].id;

        // Deepgram revised the utterance; no reference in it after all.
        let rolling = "he said join me in prayer";
        p.on_final(rolling, rolling, 2_500);

        assert!(p.drain().is_empty(), "a withdrawal emits no card");
        assert_eq!(p.take_withdrawn(), vec![id]);
        assert_eq!(p.stats().withdrawn, 1);
    }

    #[test]
    fn a_provisional_card_whose_final_never_comes_is_still_withdrawn() {
        // What the operator saw: a dimmed card that stayed dimmed. The last
        // interim of a run has no final after it. Two things now catch it —
        // the stream closing, and the entry ageing out — and both must tell
        // the window.
        let mut p = pipeline();
        p.on_interim("", "turn to John 3:16", 1_000);
        let id = p.drain()[0].id;

        // Stream closes with nothing settled after it.
        p.on_closed();
        assert_eq!(p.take_withdrawn(), vec![id]);
        assert!(p.drain().is_empty(), "withdrawal is silent: no card");

        // And the other path: nothing closes, but a minute passes with finals
        // that never mention it. (A final that does not re-find it withdraws
        // it sooner; this is the backstop for a final that never arrives at
        // all, e.g. a provisional raised in the last second before a pause.)
        let mut p = pipeline();
        p.on_interim("", "turn to John 3:16", 1_000);
        let id = p.drain()[0].id;
        p.on_interim("unrelated words", "more unrelated words", 62_000);
        assert_eq!(p.take_withdrawn(), vec![id]);
        assert_eq!(p.stats().withdrawn, 1);
    }

    #[test]
    fn finals_before_the_vector_index_is_ready_are_counted() {
        // The index loads after the window opens. Until it joins, a final with
        // no direct reference gets regex only, and the stop log must be able
        // to say how many that was.
        let mut p = pipeline();
        assert!(!p.vector_ready());
        p.on_final("just preaching here", "just preaching here", 1_000);
        p.on_final("and more of it", "and more of it", 2_000);
        p.on_final("John 3:16", "John 3:16", 3_000); // direct hit: not counted
        assert_eq!(p.stats().finals_before_vector_ready, 2);
    }

    #[test]
    fn the_same_verse_within_a_minute_is_one_card_and_after_a_minute_a_new_one() {
        let mut p = pipeline();
        let text = "John 3:16 says";
        p.on_final(text, text, 0);
        p.on_final(text, text, 10_000);
        assert_eq!(p.drain().len(), 1);
        assert_eq!(p.stats().deduplicated, 1);

        // The minute slides from the LAST mention, not the first: a preacher
        // circling a verse every thirty seconds gets one card, not one every
        // minute. Last said at 10 s, so a minute later is 70 s. A probe inside
        // the window would itself refresh it, so there is none.
        p.on_final(text, text, 71_000);
        let again = p.drain();
        assert_eq!(
            again.len(),
            1,
            "a minute after the last mention it is a new card"
        );
    }

    #[test]
    fn a_reference_split_across_utterances_is_found_from_the_window() {
        // The measured transcript: "Jeremiah." then "Chapter 29." then
        // "Verse 11,". No single final contains the reference.
        let mut p = pipeline();
        let mut rolling = String::from("open our Bibles to the book of Jeremiah.");
        assert!(p.on_final(&rolling, "Jeremiah.", 1_000).is_none());
        rolling.push_str(" Chapter 29.");
        p.on_final(&rolling, "Chapter 29.", 3_000);
        rolling.push_str(" Verse 11,");
        p.on_final(&rolling, "Verse 11,", 5_000);

        let cards = p.drain();
        let refs: Vec<&str> = cards.iter().map(|c| c.reference.as_str()).collect();
        assert!(refs.contains(&"Jeremiah 29:11"), "{refs:?}");
    }

    #[test]
    fn claude_hits_become_cards_with_capped_confidence_and_evidence() {
        let mut p = pipeline();
        let hits = vec![ParaphraseHit {
            reference: crate::bible::reference::parse("Jeremiah 29:11").unwrap(),
            evidence: "plans to prosper you".into(),
            confidence: 1.0,
        }];
        p.on_paraphrase(hits, 5_000);
        let cards = p.drain();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].source, DetectionSource::Llm);
        assert_eq!(cards[0].evidence.as_deref(), Some("plans to prosper you"));
        // 1.0 self-reported becomes 0.90: below the auto-live line, so a
        // paraphrase always passes an operator by default.
        assert!((cards[0].confidence - 0.90).abs() < 1e-6);
        assert!(!cards[0].provisional);
    }

    #[test]
    fn confidence_is_comparable_across_stages() {
        // A direct written reference outranks the best semantic match, which
        // outranks the best paraphrase. The card's percentage means one thing.
        assert!(Form::Standard.confidence() > 0.95);
        assert!(vector_confidence(&[]) <= 0.95);
        assert!(llm_confidence(1.0) <= 0.90);
        assert!(llm_confidence(0.5) < llm_confidence(0.9));
    }

    #[test]
    fn the_window_keeps_the_tail_and_the_current_text() {
        let rolling: String = (1..=100)
            .map(|i| format!("w{i}"))
            .collect::<Vec<_>>()
            .join(" ");
        let w = window(&rolling, "now");
        let words: Vec<&str> = w.split_whitespace().collect();
        assert_eq!(words.len(), WINDOW_WORDS + 1);
        assert_eq!(words[0], "w61");
        assert_eq!(*words.last().unwrap(), "now");
        assert_eq!(window("", "only"), "only");
    }

    #[test]
    fn the_wire_shape_matches_the_frontend_type() {
        // Pinned JSON, because the last field-name mismatch across this
        // boundary rendered as "Retrying in NaNs" with every type satisfied.
        let c = candidate(
            7,
            &crate::bible::reference::parse("John 3:16").unwrap(),
            DetectionSource::Regex,
            0.97,
            true,
            None,
            1234,
        );
        let json = serde_json::to_string(&c).unwrap();
        for key in [
            r#""id":7"#,
            r#""passageId":"JHN.3.16""#,
            r#""reference":"John 3:16""#,
            r#""source":"regex""#,
            r#""provisional":true"#,
            r#""detectedAtMs":1234"#,
            r#""verse":null"#,
        ] {
            assert!(json.contains(key), "{key} missing from {json}");
        }
    }
}
