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

use std::collections::VecDeque;
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

/// Lag percentiles for one kind of result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Percentiles {
    pub samples: usize,
    pub p50_ms: u64,
    pub p95_ms: u64,
    pub p99_ms: u64,
    pub max_ms: u64,
}

/// End-to-end lag, for M1's Definition of Done.
///
/// **Two numbers, because they answer different questions**, and reporting
/// only one of them was a mistake.
///
/// `interim` is when words **first appear on screen**, which is what the DoD
/// line means by "transcript appears". `settled` is when Deepgram confirms an
/// utterance will not change, which is necessarily later because it waits for
/// the speaker to stop before deciding. A first run reported 2,797 ms p99
/// against a 700 ms budget while measuring only `settled` — comparing
/// Deepgram's endpointing delay against a budget written about visible text.
///
/// `settled` still matters and is not excused by that: M2's detection stages
/// run on settled text, so it bounds how long after a spoken reference a verse
/// can reach the projector.
///
/// Both are measured per result as **now, minus the moment the audio carrying
/// those words was handed to the transcriber**. That covers the socket,
/// Deepgram's own processing and the event arriving; the 250 ms the audio spent
/// accumulating into a chunk is fixed by FR-03 and sits on top.
///
/// ## Why it is anchored per chunk rather than once per run
///
/// It used to be `wall clock since the first chunk` minus `the word's audio
/// timestamp`. Those are two different clocks, and the gap between them only
/// ever widens:
///
/// - A **dropped chunk** removes 250 ms from Deepgram's timeline and nothing
///   from the wall clock. The difference is permanent.
/// - **Sound cards drift.** Measured on this machine's microphone array over
///   ten minutes: +370 ms per hour, at +/-120 ms resolution.
///
/// Neither is latency, both were counted as latency, and both accumulate — so
/// the figure grew with session length. A 38-second run reported 452/681/698 ms
/// and a 60-minute run of the same pipeline reported 1490/1926/2445 ms, with a
/// p50 sitting almost exactly at the midpoint of a straight line, which is the
/// signature of accumulation rather than of a slow pipeline.
///
/// Anchoring each result to the send time of its own audio measures one clock
/// against itself, so neither cause can reach it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LatencySummary {
    /// When words first appear. `None` before anything has been heard.
    pub interim: Option<Percentiles>,
    /// When an utterance is confirmed.
    pub settled: Option<Percentiles>,
    /// Reconnects during the run. **A non-zero count makes the tail suspect**:
    /// replayed audio is sent faster than real time, so results for it arrive
    /// late by construction and inflate the high percentiles.
    pub reconnects: usize,
    /// Samples discarded because they fell in a catch-up window after a
    /// reconnect. Reported rather than silently dropped, so the percentiles
    /// can be read as covering less than the whole run.
    pub excluded_catch_up: usize,
    /// Chunks the transcriber would not accept, each 250 ms of speech that no
    /// one will ever read.
    ///
    /// Surfaced rather than left to a log line, because this is lost sermon
    /// rather than a slow one: a transcript with holes in it reads as complete.
    pub dropped_chunks: usize,
}

impl LatencySummary {
    /// Seconds of speech lost to chunks the transcriber refused.
    pub fn dropped_seconds(&self) -> f64 {
        self.dropped_chunks as f64 * CHUNK_SECONDS
    }
}

/// Interim lag over one minute of audio.
///
/// Mirrored by `MinuteLatency` in `src/lib/types.ts`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MinuteLatency {
    pub minute: usize,
    pub samples: usize,
    pub p50_ms: u64,
    pub p95_ms: u64,
    pub max_ms: u64,
}

/// One chunk of audio, in seconds. Mirrors `CHUNK_SAMPLES` at 16 kHz (FR-03).
const CHUNK_SECONDS: f64 = 0.25;

/// How many send marks to keep.
///
/// Deepgram can revise an utterance for a couple of seconds, and a reconnect
/// replays up to 60 s, so a result may refer to audio sent a minute ago. Five
/// minutes is generous cover at 4 marks a second — 1,200 entries, a few tens of
/// kilobytes — and bounded so an hour-long service cannot grow it without
/// limit.
const MAX_SEND_MARKS: usize = 1_200;

/// Percentiles over a set of lag samples, or `None` when there are none.
///
/// Nearest-rank on the sorted samples: with a few hundred utterances in a
/// service, interpolating between neighbours would be false precision. A
/// summary of nothing returns `None`, because zero would read as a
/// measurement rather than an absence.
fn percentiles(lags: &[f64]) -> Option<Percentiles> {
    if lags.is_empty() {
        return None;
    }

    let mut sorted = lags.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let at = |q: f64| -> u64 {
        let rank = ((sorted.len() as f64 * q).ceil() as usize).clamp(1, sorted.len());
        (sorted[rank - 1] * 1000.0).round().max(0.0) as u64
    };

    Some(Percentiles {
        samples: sorted.len(),
        p50_ms: at(0.50),
        p95_ms: at(0.95),
        p99_ms: at(0.99),
        max_ms: (sorted[sorted.len() - 1] * 1000.0).round().max(0.0) as u64,
    })
}

/// Everything transcribed since capture started.
#[derive(Debug, Default)]
pub struct SessionTranscript {
    finals: Vec<FinalSegment>,
    interim: String,
    word_count: usize,
    /// When each chunk of audio was handed to the transcriber, keyed by the
    /// cumulative audio seconds it completed.
    ///
    /// Only *accepted* chunks advance the key, because Deepgram's word
    /// timestamps count the audio it actually received — so this and its
    /// timeline stay in step even when we drop something.
    sent_marks: VecDeque<(f64, Instant)>,
    /// Cumulative seconds of audio accepted for sending.
    audio_sent: f64,
    /// The very first mark, kept after `sent_marks` has rolled past it, so the
    /// run's clock drift can still be worked out at the end.
    first_mark: Option<(f64, Instant)>,
    /// Chunks the transcriber refused. Lost speech, not just a lost sample.
    dropped_chunks: usize,
    /// Interim lags bucketed by which minute of audio they belong to.
    ///
    /// The summary percentiles cannot distinguish a lag that is *high* from one
    /// that is *growing*, and those have different causes: a growing figure
    /// means something in our pipeline is backing up, a flat one means the
    /// network is simply that far away. Answering that was costing a full
    /// 60-minute re-run each time.
    interim_by_minute: Vec<Vec<f64>>,
    /// The last minute reported to the log, so each is logged once as it closes
    /// rather than only at the end — a run that is killed still leaves its
    /// shape behind.
    logged_minute: usize,
    /// Lag per interim result: when words first appeared.
    interim_lags: Vec<f64>,
    /// Lag per settled utterance.
    settled_lags: Vec<f64>,
    /// Reconnects so far, which make the tail suspect.
    reconnects: usize,
    /// While set, results are catch-up from a replay rather than live, and are
    /// counted separately instead of poisoning the percentiles.
    catch_up_until: Option<Instant>,
    excluded_catch_up: usize,
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

        // Recorded before the segment is stored, while `end` is the newest
        // thing we know about.
        if let Some(lag) = self.lag_for_at(end, Instant::now()) {
            self.settled_lags.push(lag);
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

    /// Record how long it took for these words to appear on screen.
    ///
    /// Separate from `set_interim` because the panel takes only the text,
    /// while the measurement needs the timing the words carry.
    pub fn record_interim_timing(&mut self, words: &[Word]) {
        self.record_interim_timing_at(words, Instant::now());
    }

    fn record_interim_timing_at(&mut self, words: &[Word], now: Instant) {
        let Some(end) = words.last().map(|w| w.end) else {
            return;
        };
        if let Some(lag) = self.lag_for_at(end, now) {
            self.interim_lags.push(lag);
            self.bucket_interim(end, lag);
        }
    }

    /// Lag for a result whose newest word ends at `end`, or `None` when it
    /// should not be counted.
    ///
    /// `end` is on Deepgram's timeline, which counts the audio it actually
    /// received — the same quantity `audio_sent` counts. So the mark for the
    /// chunk that carried this word is the first one at or past `end`, and the
    /// lag is how long ago we handed that chunk over.
    /// `now` is a parameter rather than read inside, so a test can simulate an
    /// hour-long service without taking an hour. The bug this replaced only
    /// appears once wall clock and audio time have both advanced a long way,
    /// which no test that runs in milliseconds can otherwise reach — and the
    /// first attempt at testing it passed against the broken code for exactly
    /// that reason.
    fn lag_for_at(&mut self, end: f64, now: Instant) -> Option<f64> {
        // Catch-up after a reconnect is late by construction: the backlog is
        // sent faster than real time, so Deepgram returns results for audio
        // that is already old. Counting those would let a network outage read
        // as a slow pipeline.
        if self.catch_up_until.is_some_and(|until| now < until) {
            self.excluded_catch_up += 1;
            return None;
        }

        // Marks are appended in order, so this is sorted by construction.
        let idx = self
            .sent_marks
            .partition_point(|(audio_at, _)| *audio_at < end);
        let (_, sent_at) = self.sent_marks.get(idx)?;

        Some(now.saturating_duration_since(*sent_at).as_secs_f64())
    }

    /// File an interim sample under the minute of audio it belongs to, and log
    /// a minute as soon as it is complete.
    fn bucket_interim(&mut self, audio_at: f64, lag: f64) {
        let minute = (audio_at / 60.0) as usize;
        // Capped so a three-hour service cannot grow this without bound. The
        // shape is visible long before 600 samples in a minute.
        const PER_MINUTE_CAP: usize = 600;

        if self.interim_by_minute.len() <= minute {
            self.interim_by_minute.resize(minute + 1, Vec::new());
        }
        let bucket = &mut self.interim_by_minute[minute];
        if bucket.len() < PER_MINUTE_CAP {
            bucket.push(lag);
        }

        while self.logged_minute < minute {
            let done = self.logged_minute;
            if let Some(stats) = self.minute_stats(done) {
                tracing::info!(
                    minute = done,
                    samples = stats.samples,
                    p50_ms = stats.p50_ms,
                    p95_ms = stats.p95_ms,
                    max_ms = stats.max_ms,
                    dropped_chunks = self.dropped_chunks,
                    "lag by minute — a figure that climbs means something is backing up; a flat one means the network is simply this far away"
                );
            }
            self.logged_minute += 1;
        }
    }

    fn minute_stats(&self, minute: usize) -> Option<MinuteLatency> {
        let bucket = self.interim_by_minute.get(minute)?;
        if bucket.len() < 3 {
            return None;
        }
        let p = percentiles(bucket)?;
        Some(MinuteLatency {
            minute,
            samples: p.samples,
            p50_ms: p.p50_ms,
            p95_ms: p.p95_ms,
            max_ms: p.max_ms,
        })
    }

    /// How far the audio timeline has fallen behind the wall clock, in
    /// milliseconds per hour. `None` until there is enough of a run to tell.
    ///
    /// Positive means we are producing less than a second of audio per second —
    /// a sound card running slow, or chunks refused. This used to be measured
    /// *as latency*, which is why an hour-long run read three times a
    /// 38-second one. It is reported separately now so it can never be
    /// mistaken for the pipeline being slow again.
    pub fn clock_drift_ms_per_hour(&self) -> Option<f64> {
        let (first_audio, first_at) = self.first_mark?;
        let (last_audio, last_at) = *self.sent_marks.back()?;

        let wall = last_at.saturating_duration_since(first_at).as_secs_f64();
        let audio = last_audio - first_audio;
        // Under a minute the answer is noise, and a confident wrong number is
        // worse than none.
        if wall < 60.0 {
            return None;
        }
        Some((1.0 - audio / wall) * 3_600_000.0)
    }

    /// Interim lag per minute of audio, for the log and for the record.
    pub fn latency_timeline(&self) -> Vec<MinuteLatency> {
        (0..self.interim_by_minute.len())
            .filter_map(|m| self.minute_stats(m))
            .collect()
    }

    /// Note that the connection dropped, and that results for the next
    /// `catch_up_seconds` are replayed audio rather than live speech.
    pub fn note_reconnect(&mut self, catch_up_seconds: f64) {
        self.reconnects += 1;
        // A little longer than the backlog itself, because Deepgram still has
        // to process what was replayed before its results come back.
        let window = std::time::Duration::from_secs_f64(catch_up_seconds.max(1.0) + 5.0);
        self.catch_up_until = Some(Instant::now() + window);
    }

    /// Clear the in-progress text without settling it, when a stream closes.
    pub fn clear_interim(&mut self) {
        self.interim.clear();
    }

    /// Start a new service. Everything already transcribed is discarded.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Record that a chunk was offered to the transcriber.
    ///
    /// `accepted` is what `AudioFeed::send` returned. A refused chunk advances
    /// the loss count and **not** the audio clock, because Deepgram never hears
    /// it and its word timestamps will not count it either — which is exactly
    /// what keeps the two timelines from drifting apart.
    ///
    /// Called for every chunk; the caller does not have to know which is first.
    pub fn note_chunk_sent(&mut self, accepted: bool) {
        self.note_chunk_sent_at(accepted, Instant::now());
    }

    fn note_chunk_sent_at(&mut self, accepted: bool, now: Instant) {
        if !accepted {
            self.dropped_chunks += 1;
            return;
        }

        self.audio_sent += CHUNK_SECONDS;
        self.first_mark.get_or_insert((self.audio_sent, now));
        self.sent_marks.push_back((self.audio_sent, now));
        if self.sent_marks.len() > MAX_SEND_MARKS {
            self.sent_marks.pop_front();
        }
    }

    /// Lag percentiles so far, for both kinds of result.
    pub fn latency(&self) -> LatencySummary {
        LatencySummary {
            interim: percentiles(&self.interim_lags),
            settled: percentiles(&self.settled_lags),
            reconnects: self.reconnects,
            excluded_catch_up: self.excluded_catch_up,
            dropped_chunks: self.dropped_chunks,
        }
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
    use std::time::Duration;

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
        transcript.settled_lags = (1..=100).map(|ms| ms as f64 / 1000.0).collect();

        let settled = transcript.latency().settled.expect("samples exist");
        assert_eq!(settled.samples, 100);
        assert_eq!(settled.p50_ms, 50);
        assert_eq!(settled.p95_ms, 95);
        assert_eq!(settled.p99_ms, 99);
        assert_eq!(settled.max_ms, 100);
    }

    #[test]
    fn latency_is_none_before_anything_settles() {
        // A summary of nothing would read as a measurement of zero.
        let mut transcript = SessionTranscript::new();
        transcript.reset();
        let latency = transcript.latency();
        assert!(latency.interim.is_none());
        assert!(latency.settled.is_none());
    }

    /// A simulated service: audio is fed in real time, and each result comes
    /// back `response` later.
    ///
    /// Drives the clock explicitly so a 60-minute run takes microseconds. The
    /// point is that wall clock and audio time advance *together*, which is
    /// what a real service does and what the broken measurement depended on —
    /// a test that advances only one of them passes against either version.
    struct Service {
        transcript: SessionTranscript,
        base: Instant,
        chunk: u32,
    }

    impl Service {
        fn new() -> Self {
            Self {
                transcript: SessionTranscript::new(),
                base: Instant::now(),
                chunk: 0,
            }
        }

        fn at(&self, chunk: u32) -> Instant {
            self.base + Duration::from_millis(u64::from(chunk) * 250)
        }

        /// Run for `chunks` chunks, dropping one every `drop_every` (0 = none),
        /// and taking a lag sample every 20 chunks.
        fn run(&mut self, chunks: u32, drop_every: u32, response: Duration) {
            for _ in 0..chunks {
                let accepted = drop_every == 0 || self.chunk % drop_every != 0;
                // The one instant this chunk exists at: when its 250 ms
                // finished accumulating and it was handed over. The result it
                // produces is timed from here, so the harness measures exactly
                // `response` and nothing else.
                let sent_at = self.at(self.chunk);
                self.transcript.note_chunk_sent_at(accepted, sent_at);
                self.chunk += 1;

                if self.chunk % 20 == 0 {
                    // Deepgram timestamps the audio it actually received.
                    let heard = self.transcript.audio_sent;
                    self.transcript.record_interim_timing_at(
                        &words(&[("word", heard - 0.2, heard)]),
                        sent_at + response,
                    );
                }
            }
        }
    }

    #[test]
    fn lag_does_not_grow_with_the_length_of_the_service() {
        // The regression. Lag was `wall clock since the first chunk` minus
        // `the word's audio timestamp` — two different clocks whose gap only
        // widens. A 38-second run measured 452/681/698 ms; an hour of the same
        // pipeline measured 1490/1926/2445 ms, with p50 almost exactly at the
        // midpoint of a straight line. That is the signature of accumulation,
        // not of a slow pipeline.
        //
        // Here one chunk in 500 is refused — about 7 over the hour, which is
        // the order the DoD figures implied.
        let response = Duration::from_millis(400);
        let mut service = Service::new();
        service.run(4 * 60 * 60, 500, response); // 60 minutes at 4 chunks/s

        let interim = service.transcript.latency().interim.expect("samples");
        assert!(interim.samples > 500, "only {} samples", interim.samples);

        // Every sample is the same real 400 ms, so the spread must be flat.
        // Under the old formula the tail carried every dropped chunk.
        assert!(
            interim.p50_ms.abs_diff(400) <= 10,
            "p50 was {} ms, not the 400 ms actually taken",
            interim.p50_ms
        );
        assert!(
            interim.max_ms <= 450,
            "lag grew through the service: max {} ms against a flat 400",
            interim.max_ms
        );
    }

    #[test]
    fn dropped_audio_is_counted_as_lost_speech_rather_than_as_slowness() {
        // A refused chunk is 250 ms nobody will ever read. Reporting it as
        // latency describes a slow pipeline; reporting it as loss describes a
        // transcript with holes, which is the true and more serious thing.
        let mut service = Service::new();
        service.run(400, 50, Duration::from_millis(300)); // 100 s, 8 refused

        let latency = service.transcript.latency();
        assert_eq!(latency.dropped_chunks, 8);
        assert!((latency.dropped_seconds() - 2.0).abs() < 1e-9);

        let interim = latency.interim.expect("samples");
        assert!(
            interim.max_ms <= 350,
            "the 2 s of dropped audio leaked into the lag: {} ms",
            interim.max_ms
        );
    }

    #[test]
    fn a_result_for_audio_we_never_sent_is_not_counted() {
        // Better to measure nothing than to invent a number. A timestamp past
        // what we have sent means the two timelines disagree, and any lag
        // derived from them is fiction.
        let mut transcript = SessionTranscript::new();
        for _ in 0..4 {
            transcript.note_chunk_sent(true); // 1 s
        }

        transcript.record_interim_timing(&words(&[("impossible", 59.0, 60.0)]));

        assert!(
            transcript.latency().interim.is_none(),
            "a timestamp beyond the audio sent must not produce a sample"
        );
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
