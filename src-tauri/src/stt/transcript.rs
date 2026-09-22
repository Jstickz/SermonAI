//! The transcript of the service in progress (FR-10, FR-11).
//!
//! **This lives in Rust because the operator window does not keep it.** Every
//! tab in the operator window unmounts when the operator switches away — to the
//! Library, to Settings — and component state goes with it. Capture deliberately
//! keeps running across that, so a transcript held in React would come back
//! empty while the recording was still going. An operator seeing a blank panel
//! mid-sermon assumes the app has crashed and restarts the service, which is the
//! one thing that actually loses the recording.
//!
//! The same reasoning already moved display assignments out of React in M0.
//!
//! ## Three retentions, one store
//!
//! | View | Keeps | For |
//! |---|---|---|
//! | [`snapshot`](SessionTranscript::snapshot) | everything | the operator to read |
//! | [`rolling`](SessionTranscript::rolling) | the last 60 s | M2's paraphrase stage (FR-11) |
//! | SQLite `transcript_segments` | everything, on disk | the Library and summary (FR-34, M4) |
//!
//! Only the first two are here. Persistence is M4 and attaches at
//! [`push_final`](SessionTranscript::push_final), which is the single point
//! every settled word passes through.

use std::time::Instant;

use serde::{Deserialize, Serialize};

use super::deepgram::Word;
use super::ROLLING_BUFFER_SECS;

/// One settled utterance.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinalSegment {
    pub text: String,
    /// Seconds from the start of the stream. Taken from the words rather than
    /// the clock, so it stays right if a chunk is delayed.
    pub start: f64,
    pub end: f64,
    /// Kept for M2's detection stages and M4's persistence, which need
    /// per-word timing. Not sent to the panel, which only renders text.
    #[serde(skip)]
    pub words: Vec<Word>,
}

/// How long a silence has to be before it reads as a new paragraph.
///
/// A preacher pauses for breath in well under a second and for effect in two
/// or three. 2.5 s catches the second without breaking on the first, and it
/// uses timing the transcript already has rather than guessing from sentence
/// length — a long verse read aloud is one thought, and three short sentences
/// in a row are usually one too.
///
/// Mirrored by `PARAGRAPH_GAP_SECONDS` in `LiveTranscript.tsx`, which applies
/// the same rule to a final as it arrives rather than waiting for a snapshot.
pub const PARAGRAPH_GAP_SECS: f64 = 2.5;

/// What the panel needs to render, and nothing more.
///
/// Words are deliberately excluded: a 45-minute sermon is several thousand of
/// them, and sending the lot across IPC every time a tab is opened would cost
/// far more than the text it renders.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptSnapshot {
    /// Settled utterances grouped into paragraphs by the pauses between them.
    /// A 45-minute sermon as one unbroken block is unreadable.
    pub paragraphs: Vec<String>,
    pub finals: Vec<String>,
    /// The utterance in progress, which the next interim replaces.
    pub interim: String,
    pub word_count: usize,
}

/// End-to-end lag, for M1's Definition of Done.
///
/// Measured per settled utterance as *now minus when those words were spoken*:
/// the wall clock since capture began, less the audio timestamp of the last
/// word. That covers the whole path — capture, conversion, the socket,
/// Deepgram's own processing and the event reaching us — rather than timing
/// one hop and calling it latency.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LatencySummary {
    pub samples: usize,
    pub p50_ms: u64,
    pub p95_ms: u64,
    /// The number M1's DoD budgets at 700 ms.
    pub p99_ms: u64,
    pub max_ms: u64,
}

/// Everything transcribed since capture started.
#[derive(Debug, Default)]
pub struct SessionTranscript {
    finals: Vec<FinalSegment>,
    interim: String,
    word_count: usize,
    /// When capture began, for measuring lag. `None` until the first start.
    started: Option<Instant>,
    /// Lag per settled utterance, in seconds.
    lags: Vec<f64>,
}

impl SessionTranscript {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a settled utterance.
    ///
    /// The one place every confirmed word passes through, which is why M4's
    /// SQLite write belongs here rather than in the command handler.
    pub fn push_final(&mut self, text: String, words: Vec<Word>) {
        if text.trim().is_empty() {
            return;
        }

        let start = words.first().map_or(0.0, |w| w.start);
        let end = words.last().map_or(start, |w| w.end);

        // Lag is recorded before the segment is stored, while `end` is the
        // newest thing we know about.
        if let Some(started) = self.started {
            let lag = started.elapsed().as_secs_f64() - end;
            // A negative lag means the clock and the audio timeline disagree,
            // which happened once already when a reconnect double-counted the
            // offset. Recording it would hide that; dropping it would too, so
            // it is clamped at zero and the max will show the disagreement.
            self.lags.push(lag.max(0.0));
        }

        self.word_count += text.split_whitespace().count();
        self.finals.push(FinalSegment {
            text,
            start,
            end,
            words,
        });
        // A final ends whatever was in progress. Leaving the interim would
        // show the same sentence twice: once settled, once as a ghost.
        self.interim.clear();
    }

    /// Replace the utterance in progress.
    ///
    /// Replace, never append: Deepgram revises its guess as it hears more, so
    /// each interim supersedes the last.
    pub fn set_interim(&mut self, text: String) {
        self.interim = text;
    }

    /// Clear the in-progress text without settling it, when a stream closes.
    pub fn clear_interim(&mut self) {
        self.interim.clear();
    }

    /// Start a new service. Everything already transcribed is discarded.
    ///
    /// The clock starts here rather than at the first word, so the wait before
    /// anyone speaks is not counted as lag.
    pub fn reset(&mut self) {
        *self = Self {
            started: Some(Instant::now()),
            ..Self::default()
        };
    }

    /// Lag percentiles so far, or `None` before anything has settled.
    ///
    /// Nearest-rank percentiles on the sorted samples: with a few hundred
    /// utterances in a service, interpolating between neighbours would be
    /// false precision.
    pub fn latency(&self) -> Option<LatencySummary> {
        if self.lags.is_empty() {
            return None;
        }

        let mut sorted = self.lags.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let at = |q: f64| -> u64 {
            let rank = ((sorted.len() as f64 * q).ceil() as usize).clamp(1, sorted.len());
            (sorted[rank - 1] * 1000.0).round().max(0.0) as u64
        };

        Some(LatencySummary {
            samples: sorted.len(),
            p50_ms: at(0.50),
            p95_ms: at(0.95),
            p99_ms: at(0.99),
            max_ms: (sorted[sorted.len() - 1] * 1000.0).round().max(0.0) as u64,
        })
    }

    /// Group settled utterances into paragraphs by the pauses between them.
    fn paragraphs(&self) -> Vec<String> {
        let mut paragraphs: Vec<String> = Vec::new();
        let mut previous_end: Option<f64> = None;

        for segment in &self.finals {
            let breaks = previous_end.is_some_and(|end| segment.start - end >= PARAGRAPH_GAP_SECS);

            match paragraphs.last_mut() {
                Some(current) if !breaks => {
                    current.push(' ');
                    current.push_str(&segment.text);
                }
                _ => paragraphs.push(segment.text.clone()),
            }
            previous_end = Some(segment.end);
        }

        paragraphs
    }

    pub fn snapshot(&self) -> TranscriptSnapshot {
        TranscriptSnapshot {
            paragraphs: self.paragraphs(),
            finals: self.finals.iter().map(|f| f.text.clone()).collect(),
            interim: self.interim.clone(),
            word_count: self.word_count,
        }
    }

    /// The last [`ROLLING_BUFFER_SECS`] of settled speech (FR-11).
    ///
    /// For M2's paraphrase stage, which sends recent context to Claude when the
    /// regex stage has found nothing. Measured against the end of the newest
    /// segment rather than wall-clock time, so a pause in the service does not
    /// silently empty the window — the preacher stopping for ten seconds should
    /// not erase what they said before it.
    ///
    /// Interim text is excluded: acting on words that may be revised is exactly
    /// what [`crate::stt::deepgram::TranscriptEvent`] exists to prevent.
    pub fn rolling(&self) -> String {
        let Some(newest) = self.finals.last() else {
            return String::new();
        };

        let cutoff = newest.end - ROLLING_BUFFER_SECS as f64;
        let kept: Vec<&str> = self
            .finals
            .iter()
            // By segment end, not start: an utterance that began before the
            // window but finished inside it is still recent speech.
            .filter(|segment| segment.end >= cutoff)
            .map(|segment| segment.text.as_str())
            .collect();

        kept.join(" ")
    }

    pub fn is_empty(&self) -> bool {
        self.finals.is_empty() && self.interim.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(pairs: &[(&str, f64, f64)]) -> Vec<Word> {
        pairs
            .iter()
            .map(|(w, start, end)| Word {
                word: (*w).to_string(),
                start: *start,
                end: *end,
                confidence: 0.95,
            })
            .collect()
    }

    #[test]
    fn a_final_ends_the_utterance_in_progress() {
        let mut transcript = SessionTranscript::new();

        transcript.set_interim("turn with me to".to_string());
        transcript.push_final(
            "Turn with me to John.".to_string(),
            words(&[("Turn", 0.0, 0.3), ("John.", 1.0, 1.4)]),
        );

        let snapshot = transcript.snapshot();
        assert_eq!(snapshot.finals, vec!["Turn with me to John."]);
        // Left behind, the interim would render the same sentence twice: once
        // settled and once as a ghost below it.
        assert_eq!(snapshot.interim, "");
    }

    #[test]
    fn an_interim_replaces_rather_than_accumulates() {
        let mut transcript = SessionTranscript::new();

        transcript.set_interim("turn with".to_string());
        transcript.set_interim("turn with me to join".to_string());
        transcript.set_interim("turn with me to John chapter three".to_string());

        // The revision Deepgram actually made in testing. Appending would show
        // all three, each containing the last.
        assert_eq!(
            transcript.snapshot().interim,
            "turn with me to John chapter three"
        );
    }

    #[test]
    fn the_snapshot_survives_being_taken_repeatedly() {
        // The panel takes one on every mount, which happens every time the
        // operator switches back to the Live tab.
        let mut transcript = SessionTranscript::new();
        transcript.push_final("First.".to_string(), words(&[("First.", 0.0, 0.5)]));

        let first = transcript.snapshot();
        let second = transcript.snapshot();
        assert_eq!(first.finals, second.finals);
        assert_eq!(first.word_count, 1);
    }

    #[test]
    fn empty_and_whitespace_finals_are_not_recorded() {
        // Deepgram sends these during silence. A blank segment would add a
        // stray space to the transcript and inflate the word count.
        let mut transcript = SessionTranscript::new();
        transcript.push_final(String::new(), vec![]);
        transcript.push_final("   ".to_string(), vec![]);

        assert!(transcript.is_empty());
        assert_eq!(transcript.snapshot().word_count, 0);
    }

    #[test]
    fn latency_percentiles_use_nearest_rank() {
        let mut transcript = SessionTranscript::new();
        transcript.reset();
        // Injected directly: measuring real lag would mean sleeping through it.
        transcript.lags = (1..=100).map(|ms| ms as f64 / 1000.0).collect();

        let summary = transcript.latency().expect("samples exist");
        assert_eq!(summary.samples, 100);
        assert_eq!(summary.p50_ms, 50);
        assert_eq!(summary.p95_ms, 95);
        assert_eq!(summary.p99_ms, 99);
        assert_eq!(summary.max_ms, 100);
    }

    #[test]
    fn latency_is_none_before_anything_settles() {
        // A summary of nothing would read as a measurement of zero.
        let mut transcript = SessionTranscript::new();
        transcript.reset();
        assert!(transcript.latency().is_none());
    }

    #[test]
    fn a_pause_starts_a_new_paragraph() {
        let mut transcript = SessionTranscript::new();

        // Two sentences in quick succession, then a three-second pause.
        transcript.push_final("First thought.".to_string(), words(&[("First.", 0.0, 1.0)]));
        transcript.push_final(
            "Still the same.".to_string(),
            words(&[("Still.", 1.4, 2.0)]),
        );
        transcript.push_final("New thought.".to_string(), words(&[("New.", 5.0, 6.0)]));

        let paragraphs = transcript.snapshot().paragraphs;
        assert_eq!(
            paragraphs,
            vec!["First thought. Still the same.", "New thought."]
        );
    }

    #[test]
    fn a_breath_does_not_start_a_new_paragraph() {
        // Well under the threshold. Breaking here would give a paragraph per
        // sentence, which is not what a sermon reads like.
        let mut transcript = SessionTranscript::new();
        transcript.push_final("One.".to_string(), words(&[("One.", 0.0, 1.0)]));
        transcript.push_final("Two.".to_string(), words(&[("Two.", 1.6, 2.0)]));

        assert_eq!(transcript.snapshot().paragraphs, vec!["One. Two."]);
    }

    #[test]
    fn the_rolling_window_keeps_the_last_minute_and_drops_the_rest() {
        let mut transcript = SessionTranscript::new();

        // Three utterances across four minutes of service.
        transcript.push_final("Long ago.".to_string(), words(&[("ago.", 10.0, 11.0)]));
        transcript.push_final(
            "A while back.".to_string(),
            words(&[("back.", 120.0, 121.0)]),
        );
        transcript.push_final("Just now.".to_string(), words(&[("now.", 170.0, 171.0)]));

        let recent = transcript.rolling();
        assert!(recent.contains("Just now."));
        // 121 s is within 60 s of 171 s, so it stays.
        assert!(recent.contains("A while back."));
        // 11 s is not.
        assert!(!recent.contains("Long ago."), "{recent}");
    }

    #[test]
    fn a_pause_does_not_empty_the_rolling_window() {
        // Measured against the newest segment, not the clock. If the preacher
        // stops for two minutes, what they said before the pause is still the
        // most recent thing they said, and the paraphrase stage needs it.
        let mut transcript = SessionTranscript::new();
        transcript.push_final(
            "Before the pause.".to_string(),
            words(&[("pause.", 30.0, 31.0)]),
        );

        std::thread::sleep(std::time::Duration::from_millis(20));

        assert_eq!(transcript.rolling(), "Before the pause.");
    }

    #[test]
    fn the_rolling_window_excludes_unsettled_text() {
        // Acting on words that may be revised is exactly what the interim and
        // final split exists to prevent, and the paraphrase stage acts.
        let mut transcript = SessionTranscript::new();
        transcript.push_final("Settled.".to_string(), words(&[("Settled.", 1.0, 2.0)]));
        transcript.set_interim("not settled yet".to_string());

        assert_eq!(transcript.rolling(), "Settled.");
    }

    #[test]
    fn a_new_service_starts_clean() {
        let mut transcript = SessionTranscript::new();
        transcript.push_final("Last week.".to_string(), words(&[("week.", 0.0, 1.0)]));
        transcript.set_interim("still going".to_string());

        transcript.reset();

        assert!(transcript.is_empty());
        assert_eq!(transcript.snapshot().word_count, 0);
        assert_eq!(transcript.rolling(), "");
    }
}
