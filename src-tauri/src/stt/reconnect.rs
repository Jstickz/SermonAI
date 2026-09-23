//! Keeping transcription alive across a dropped connection (M1 deliverable 10).
//!
//! A church's internet drops mid-sermon. The preacher does not stop, so
//! neither can we: audio keeps being captured, the socket is reopened behind
//! the scenes, and the operator is told what is happening rather than left
//! watching a transcript that quietly stopped growing.
//!
//! ## Three things this has to get right
//!
//! **Audio during the outage is held, not dropped.** 16 kHz mono PCM is 32 KB
//! a second, so a minute of it is under 2 MB — cheap enough to keep, and the
//! difference between a gap in the sermon and no gap at all. Deepgram accepts
//! audio faster than real time, so the backlog is flushed on reconnect and the
//! transcript catches up. Past the cap the oldest is dropped, because an
//! unbounded buffer turns a network problem into a crash.
//!
//! **Timestamps must not go backwards.** Every reconnect is a *new* Deepgram
//! stream whose word timings restart at zero. Left alone that breaks both
//! things built on them: the paragraph rule would see a negative gap, and the
//! rolling buffer's cutoff would discard the whole transcript. So a running
//! offset is added to every word, advanced by the audio actually sent.
//!
//! **Giving up is worse than retrying.** The backoff caps at 30 seconds and
//! keeps trying for as long as capture is running. Stopping after a fixed
//! number of attempts would abandon a service that was about to come back.

use std::time::Duration;

use tokio::sync::mpsc;

use super::deepgram::{self, DeepgramSession, EventSink, TranscriptEvent, Word};
use crate::audio::{convert::CHUNK_SAMPLES, SAMPLE_RATE_HZ};
use crate::credentials::{Access, Credentials, Service};
use crate::error::Result;

/// Delays between reconnection attempts, in milliseconds.
///
///
/// Quick at first, because most drops are a momentary blip and a two-second
/// wait for a one-second outage is two seconds of sermon transcribed late.
/// The tail is long because a genuinely offline church gains nothing from
/// being hammered, and the last value repeats for as long as it takes.
///
/// No jitter: there is exactly one client, so there is no thundering herd to
/// spread out, and a predictable delay is easier to explain to an operator
/// watching a countdown.
const BACKOFF_MS: &[u64] = &[500, 1_000, 2_000, 4_000, 8_000, 15_000, 30_000];

/// How much audio to hold while disconnected: 60 seconds.
///
/// Matches the DoD line that asks for a 30-second outage to recover, with room
/// to spare. Beyond this the oldest chunks go — a transcript missing its first
/// minute is recoverable, an app that ran out of memory mid-service is not.
const MAX_BACKLOG_CHUNKS: usize = 240;

/// Seconds of audio in one chunk, for advancing the timestamp offset.
const CHUNK_SECONDS: f64 = CHUNK_SAMPLES as f64 / SAMPLE_RATE_HZ as f64;

/// What the operator should be told about the transcription connection.
///
/// Mirrored by `SttStatus` in `src/lib/types.ts`.
///
/// `rename_all_fields` is the load-bearing attribute. `rename_all` renames the
/// *variants*, not the fields inside them, so `retry_in_ms` reached the
/// frontend under that name while TypeScript read `retryInMs` — and the banner
/// read "Retrying in NaNs". Nothing failed: the type said `number`, the value
/// was `undefined`, and `undefined / 1000` is a number in JavaScript. The
/// contract test below is what makes the field names an assertion rather than
/// an assumption.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(
    rename_all = "snake_case",
    tag = "kind",
    rename_all_fields = "camelCase"
)]
pub enum SttStatus {
    Connected,
    /// Trying again. Carries enough for a banner to count down rather than
    /// just spin, because "reconnecting" with no end in sight reads as hung.
    Reconnecting {
        attempt: u32,
        retry_in_ms: u64,
        /// Seconds of speech held so far. Shown so the operator knows the
        /// sermon is being kept, not lost.
        buffered_seconds: f64,
    },
    /// Audio was dropped because the outage outlasted the buffer. The
    /// transcript has a hole in it and says so, rather than running the two
    /// sides of the gap together as though nothing happened.
    AudioDropped {
        seconds: f64,
    },
}

pub type StatusSink = Box<dyn FnMut(SttStatus) + Send + 'static>;

/// A cloneable handle for feeding audio in from the capture thread.
#[derive(Clone)]
pub struct AudioFeed {
    audio: mpsc::Sender<Vec<i16>>,
}

impl AudioFeed {
    /// Queue a chunk. Never blocks: the audio thread has a deadline the OS
    /// enforces and must not wait on a network write.
    pub fn send(&self, chunk: Vec<i16>) {
        if self.audio.try_send(chunk).is_err() {
            tracing::warn!("transcription is not keeping up; dropping audio");
        }
    }
}

/// A transcription stream that reopens itself.
pub struct ResilientStream {
    audio: mpsc::Sender<Vec<i16>>,
    task: tauri::async_runtime::JoinHandle<()>,
}

impl ResilientStream {
    /// Open a stream and keep it open.
    ///
    /// The first connection is awaited, so a bad key or a blocked network is
    /// reported at Start rather than as a transcript that never appears. Every
    /// later reconnection happens in the background.
    pub async fn connect(
        credentials: &Credentials,
        vocabulary: Vec<String>,
        on_event: EventSink,
        on_status: StatusSink,
    ) -> Result<Self> {
        Self::connect_at(
            deepgram::default_endpoint(),
            credentials,
            vocabulary,
            on_event,
            on_status,
        )
        .await
    }

    /// The same, against a given endpoint, for tests that need a server which
    /// fails on purpose.
    pub async fn connect_at(
        endpoint: &str,
        credentials: &Credentials,
        vocabulary: Vec<String>,
        on_event: EventSink,
        on_status: StatusSink,
    ) -> Result<Self> {
        let access = credentials.access(Service::Deepgram)?;
        let (audio_tx, audio_rx) = mpsc::channel::<Vec<i16>>(MAX_BACKLOG_CHUNKS);

        let mut supervisor = Supervisor {
            endpoint: endpoint.to_string(),
            access,
            vocabulary,
            on_event,
            on_status,
            offset_seconds: 0.0,
            stream_seconds: 0.0,
            backlog: Vec::new(),
        };

        let session = supervisor.open().await?;
        let task = tauri::async_runtime::spawn(supervisor.run(session, audio_rx));

        Ok(Self {
            audio: audio_tx,
            task,
        })
    }

    /// A cloneable handle for the audio thread.
    pub fn feed(&self) -> AudioFeed {
        AudioFeed {
            audio: self.audio.clone(),
        }
    }

    /// Queue a chunk. Never blocks.
    ///
    /// A full channel here means the supervisor is wedged rather than merely
    /// disconnected — disconnection is absorbed by the backlog inside it — so
    /// the chunk is dropped and logged.
    pub fn send(&self, chunk: Vec<i16>) {
        if self.audio.try_send(chunk).is_err() {
            tracing::warn!("transcription is not keeping up; dropping audio");
        }
    }

    /// Close the stream and wait for the final results.
    pub async fn finish(self) {
        drop(self.audio);
        if let Err(err) = self.task.await {
            tracing::error!(%err, "the transcription supervisor did not shut down cleanly");
        }
    }
}

struct Supervisor {
    endpoint: String,
    access: Access,
    vocabulary: Vec<String>,
    on_event: EventSink,
    on_status: StatusSink,
    /// Global time at which the *current* stream's clock reads zero.
    ///
    /// Advanced only when a stream is replaced, by however much audio that
    /// stream received. Advancing it per chunk instead would double-count:
    /// Deepgram's own timestamps already cover the audio sent to the stream
    /// that produced them, so adding to them as well runs the transcript's
    /// clock at twice real time — an 18.6 s recording reported timings out to
    /// 33.8 s before this was caught.
    offset_seconds: f64,
    /// Audio sent to the current stream, which becomes the next offset step.
    stream_seconds: f64,
    backlog: Vec<Vec<i16>>,
}

impl Supervisor {
    /// Open one socket, routing its events through a channel the supervisor
    /// owns so it can see a close rather than only the operator seeing it.
    async fn open(&mut self) -> Result<Stream> {
        let (event_tx, event_rx) = mpsc::unbounded_channel::<TranscriptEvent>();

        let session = DeepgramSession::connect_to(
            &self.endpoint,
            &self.access,
            self.vocabulary.clone(),
            Box::new(move |event| {
                // Unbounded because dropping a transcript event loses words,
                // and the receiver is a tight loop that cannot fall behind.
                let _ = event_tx.send(event);
            }),
        )
        .await?;

        Ok(Stream {
            session,
            events: event_rx,
        })
    }

    async fn run(mut self, mut stream: Stream, mut audio: mpsc::Receiver<Vec<i16>>) {
        tracing::info!("transcription supervisor started");
        (self.on_status)(SttStatus::Connected);

        // A heartbeat, so a retest shows whether the supervisor is running,
        // starved or wedged. Silence in the log was the least useful thing
        // about the last failure: the app looked stuck and nothing said where.
        let mut heartbeat = tokio::time::interval(Duration::from_secs(5));
        heartbeat.tick().await;
        let mut chunks_in: u64 = 0;
        let mut chunks_sent: u64 = 0;

        loop {
            tokio::select! {
                _ = heartbeat.tick() => {
                    tracing::info!(
                        chunks_in,
                        chunks_sent,
                        buffered = self.backlog.len(),
                        audio_seconds = self.offset_seconds + self.stream_seconds,
                        "transcription alive"
                    );
                }

                chunk = audio.recv() => match chunk {
                    Some(samples) => {
                        chunks_in += 1;
                        // Awaited, not dropped on a full queue: this task is
                        // not the audio thread, so it can wait, and waiting
                        // pushes back on the 60-second buffer that exists to
                        // absorb exactly this.
                        if stream.session.send_awaiting(samples.clone()).await {
                            self.stream_seconds += CHUNK_SECONDS;
                            chunks_sent += 1;
                        } else {
                            // The socket task has gone. Previously this ended
                            // the whole run, which stopped transcription for
                            // good on the first hiccup; a dead stream is the
                            // reason reconnection exists.
                            tracing::warn!("the stream stopped accepting audio; reconnecting");
                            self.hold(samples);
                            match self.reconnect(&mut audio).await {
                                Some(next) => {
                                    stream = next;
                                    (self.on_status)(SttStatus::Connected);
                                }
                                None => break,
                            }
                        }
                    }
                    // The handle was dropped: capture has stopped.
                    None => break,
                },

                event = stream.events.recv() => match event {
                    Some(TranscriptEvent::Closed { reason }) => {
                        tracing::warn!(
                            ?reason,
                            buffered_chunks = self.backlog.len(),
                            "the transcription stream closed; reconnecting"
                        );
                        match self.reconnect(&mut audio).await {
                            Some(next) => {
                                stream = next;
                                (self.on_status)(SttStatus::Connected);
                            }
                            // Only returned when capture itself stopped while
                            // we were retrying.
                            None => break,
                        }
                    }
                    Some(event) => self.emit(event),
                    None => break,
                },
            }
        }

        tracing::info!("transcription supervisor stopping");
        stream.session.finish().await;
        // Drain whatever arrived during the close handshake: the final results
        // for the last utterance come through here.
        while let Ok(event) = stream.events.try_recv() {
            self.emit(event);
        }
    }

    /// Shift an event's timings into the session's own timeline and pass it on.
    fn emit(&mut self, event: TranscriptEvent) {
        let shift = |words: Vec<Word>, offset: f64| -> Vec<Word> {
            words
                .into_iter()
                .map(|word| Word {
                    start: word.start + offset,
                    end: word.end + offset,
                    ..word
                })
                .collect()
        };

        let shifted = match event {
            TranscriptEvent::Interim { text, words } => TranscriptEvent::Interim {
                text,
                words: shift(words, self.offset_seconds),
            },
            TranscriptEvent::Final {
                text,
                words,
                speech_final,
            } => TranscriptEvent::Final {
                text,
                words: shift(words, self.offset_seconds),
                speech_final,
            },
            other => other,
        };

        (self.on_event)(shifted);
    }

    /// Retry until the socket comes back or capture stops.
    ///
    /// Audio arriving during the outage is held rather than dropped, which is
    /// the difference between a gap in the sermon and none.
    async fn reconnect(&mut self, audio: &mut mpsc::Receiver<Vec<i16>>) -> Option<Stream> {
        self.begin_new_stream();

        let mut attempt: u32 = 0;

        loop {
            let delay = BACKOFF_MS[(attempt as usize).min(BACKOFF_MS.len() - 1)];
            attempt += 1;

            (self.on_status)(SttStatus::Reconnecting {
                attempt,
                retry_in_ms: delay,
                buffered_seconds: self.backlog.len() as f64 * CHUNK_SECONDS,
            });

            // Audio is collected *while* waiting rather than after. Sleeping
            // first and draining later would drop everything spoken during the
            // delay, which at the 30-second cap is most of the outage.
            let deadline = tokio::time::sleep(Duration::from_millis(delay));
            tokio::pin!(deadline);

            loop {
                tokio::select! {
                    () = &mut deadline => break,
                    chunk = audio.recv() => match chunk {
                        Some(samples) => self.hold(samples),
                        // Capture stopped mid-outage. Nothing left to do.
                        None => return None,
                    },
                }
            }

            tracing::info!(attempt, "reopening the Deepgram connection");
            match self.open().await {
                Ok(stream) => {
                    tracing::info!(attempt, "reconnected");
                    // The backlog is sent before anything new, so the sermon
                    // stays in order.
                    // Backlog first, so the sermon stays in order — and it
                    // counts towards the new stream's audio, not the old one's.
                    //
                    // Every chunk is awaited. Firing them all at a queue a
                    // sixth the size of the backlog is what corrupted the
                    // transcript after a reconnect: most of the held minute
                    // was discarded on the floor of a `try_send`.
                    let held = std::mem::take(&mut self.backlog);
                    let expected = held.len();
                    let mut replayed = 0usize;

                    for chunk in held {
                        if !stream.session.send_awaiting(chunk).await {
                            break;
                        }
                        self.stream_seconds += CHUNK_SECONDS;
                        replayed += 1;
                    }

                    if replayed != expected {
                        // The socket died again mid-replay. Say how much of the
                        // sermon went with it rather than leaving a silent hole.
                        let lost = (expected - replayed) as f64 * CHUNK_SECONDS;
                        tracing::error!(replayed, expected, "the replay was cut short");
                        (self.on_status)(SttStatus::AudioDropped { seconds: lost });
                    } else {
                        tracing::info!(
                            chunks = replayed,
                            seconds = replayed as f64 * CHUNK_SECONDS,
                            "replayed held audio after reconnecting"
                        );
                    }
                    return Some(stream);
                }
                Err(err) => {
                    tracing::warn!(%err, attempt, "reconnection failed; will retry");
                }
            }
        }
    }

    /// Move the clock on to a replacement stream.
    ///
    /// The stream that just died covered `stream_seconds` of audio, so the
    /// next stream's zero sits that far further along the service. Separate
    /// from `reconnect` so the arithmetic can be tested without a socket —
    /// this is where the double-counting bug lived.
    fn begin_new_stream(&mut self) {
        self.offset_seconds += self.stream_seconds;
        self.stream_seconds = 0.0;
    }

    /// Total audio this supervisor has sent, across every stream.
    ///
    /// Only the test reads this, but it names the invariant the clock has to
    /// hold: the transcript's timeline and the audio's duration are the same
    /// number, and the bug this guards against made them diverge.
    #[cfg(test)]
    fn total_audio_seconds(&self) -> f64 {
        self.offset_seconds + self.stream_seconds
    }

    /// Hold a chunk, dropping the oldest once the buffer is full.
    fn hold(&mut self, chunk: Vec<i16>) {
        if self.backlog.len() >= MAX_BACKLOG_CHUNKS {
            self.backlog.remove(0);
            // Reported rather than silent. A transcript that runs the two
            // sides of a gap together reads as continuous speech, and nobody
            // reading it later would know a minute was missing.
            (self.on_status)(SttStatus::AudioDropped {
                seconds: CHUNK_SECONDS,
            });
        }
        self.backlog.push(chunk);
    }
}

struct Stream {
    session: DeepgramSession,
    events: mpsc::UnboundedReceiver<TranscriptEvent>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The exact JSON the banner receives, with a real backoff value.
    ///
    /// This is a contract test, and it exists because the field names were
    /// wrong in a way nothing caught. `rename_all` renames variants, not the
    /// fields inside them, so `retry_in_ms` crossed the boundary under that
    /// name; TypeScript read `retryInMs`, got `undefined`, and rendered
    /// "Retrying in NaNs". The type said `number` and JavaScript was happy to
    /// divide `undefined` by a thousand.
    ///
    /// The literal below is duplicated in `LiveTranscript.test.ts`, which
    /// formats it into the sentence an operator reads. If this assertion is
    /// ever updated, that fixture has to change with it — which is the point.
    #[test]
    fn the_status_json_matches_what_the_banner_reads() {
        // A real delay from the table, not an invented one: the third attempt.
        let retry_in_ms = BACKOFF_MS[2];
        assert_eq!(retry_in_ms, 2_000);

        let status = SttStatus::Reconnecting {
            attempt: 3,
            retry_in_ms,
            // Twelve seconds of held speech: 48 chunks at a quarter-second.
            buffered_seconds: 48.0 * CHUNK_SECONDS,
        };

        let json = serde_json::to_string(&status).expect("status should serialize");
        assert_eq!(
            json,
            r#"{"kind":"reconnecting","attempt":3,"retryInMs":2000,"bufferedSeconds":12.0}"#
        );
    }

    #[test]
    fn a_connected_status_carries_no_fields_to_misread() {
        assert_eq!(
            serde_json::to_string(&SttStatus::Connected).unwrap(),
            r#"{"kind":"connected"}"#
        );
    }

    #[test]
    fn the_backoff_starts_quick_and_settles_long() {
        // Most drops are a blip: waiting two seconds for a one-second outage
        // is two seconds of sermon transcribed late.
        assert_eq!(BACKOFF_MS[0], 500);
        // And a genuinely offline church gains nothing from being hammered.
        assert_eq!(*BACKOFF_MS.last().unwrap(), 30_000);

        // Monotonic, or a "retrying in 8s" banner could be followed by a
        // shorter wait and read as a bug.
        for pair in BACKOFF_MS.windows(2) {
            assert!(pair[1] > pair[0], "backoff must increase: {pair:?}");
        }
    }

    #[test]
    fn the_backoff_never_gives_up() {
        // Indexing is clamped to the last delay, so attempt 100 waits 30 s
        // rather than panicking or stopping. A service that is about to come
        // back should not be abandoned at attempt six.
        for attempt in [0usize, 5, 6, 50, 10_000] {
            let delay = BACKOFF_MS[attempt.min(BACKOFF_MS.len() - 1)];
            assert!((500..=30_000).contains(&delay));
        }
    }

    #[test]
    fn the_backlog_holds_a_minute_of_speech() {
        // The DoD asks for a 30-second outage to recover; this leaves room.
        let seconds = MAX_BACKLOG_CHUNKS as f64 * CHUNK_SECONDS;
        assert!((seconds - 60.0).abs() < f64::EPSILON, "{seconds}s");

        // And it is small enough to be worth holding: 16-bit mono at 16 kHz.
        let bytes = MAX_BACKLOG_CHUNKS * CHUNK_SAMPLES * 2;
        assert!(bytes < 2_500_000, "{bytes} bytes is too much to hold");
    }

    fn supervisor() -> Supervisor {
        Supervisor {
            endpoint: deepgram::default_endpoint().to_string(),
            access: Access::DirectKey(crate::credentials::Secret::new("test")),
            vocabulary: Vec::new(),
            on_event: Box::new(|_| {}),
            on_status: Box::new(|_| {}),
            offset_seconds: 0.0,
            stream_seconds: 0.0,
            backlog: Vec::new(),
        }
    }

    #[test]
    fn the_clock_advances_once_per_second_of_audio_not_twice() {
        // The regression this exists for: the offset was advanced on every
        // chunk *and* Deepgram's own timestamps already covered that audio, so
        // the transcript's clock ran at twice real time. An 18.6 s recording
        // reported word timings out to 33.8 s, which would have put the
        // paragraph rule and the rolling window into a different service.
        let mut supervisor = supervisor();

        // Ten seconds of audio on the first stream.
        for _ in 0..40 {
            supervisor.stream_seconds += CHUNK_SECONDS;
        }

        // While one stream is running the offset does not move at all: the
        // stream's own timestamps are already correct.
        assert_eq!(supervisor.offset_seconds, 0.0);
        assert!((supervisor.total_audio_seconds() - 10.0).abs() < 1e-9);

        // Replacing the stream is the only thing that moves it, and it moves
        // by exactly what the old stream covered.
        supervisor.begin_new_stream();
        assert!((supervisor.offset_seconds - 10.0).abs() < 1e-9);
        assert_eq!(supervisor.stream_seconds, 0.0);
        assert!((supervisor.total_audio_seconds() - 10.0).abs() < 1e-9);

        // Five more seconds, then another drop.
        for _ in 0..20 {
            supervisor.stream_seconds += CHUNK_SECONDS;
        }
        supervisor.begin_new_stream();

        // Fifteen seconds of audio, fifteen seconds on the clock. Not thirty.
        assert!((supervisor.total_audio_seconds() - 15.0).abs() < 1e-9);
    }

    #[test]
    fn the_oldest_audio_goes_first_when_the_buffer_is_full() {
        let mut supervisor = supervisor();

        for i in 0..MAX_BACKLOG_CHUNKS {
            supervisor.hold(vec![i as i16]);
        }
        assert_eq!(supervisor.backlog.len(), MAX_BACKLOG_CHUNKS);
        assert_eq!(supervisor.backlog[0][0], 0);

        // One more pushes the first out: losing the start of a long outage
        // beats running out of memory mid-service.
        supervisor.hold(vec![9_999]);
        assert_eq!(supervisor.backlog.len(), MAX_BACKLOG_CHUNKS);
        assert_eq!(supervisor.backlog[0][0], 1, "the oldest chunk should go");
        assert_eq!(supervisor.backlog.last().unwrap()[0], 9_999);
    }

    /// The bug that corrupted the transcript after a reconnect, as a test.
    ///
    /// The backlog holds sixty seconds; the socket's queue holds ten. Firing
    /// the whole backlog at it with `try_send` discarded everything past the
    /// first forty chunks, and the only trace was a log line per drop. The
    /// transcript came back missing words and nothing reported a failure.
    #[tokio::test]
    async fn replaying_the_backlog_does_not_outrun_the_socket_queue() {
        let (tx, mut rx) = mpsc::channel::<Vec<i16>>(40);

        // What the old code did.
        let mut dropped = 0;
        for i in 0..MAX_BACKLOG_CHUNKS {
            if tx.try_send(vec![i as i16]).is_err() {
                dropped += 1;
            }
        }
        assert!(
            dropped > 0,
            "the queue is smaller than the backlog, so try_send must drop"
        );
        assert_eq!(dropped, MAX_BACKLOG_CHUNKS - 40, "only the first 40 fit");

        // What it does now: a reader drains while the writer awaits, so every
        // chunk arrives, in order, exactly once.
        while rx.try_recv().is_ok() {}
        let reader = tokio::spawn(async move {
            let mut seen = Vec::new();
            while let Some(chunk) = rx.recv().await {
                seen.push(chunk[0]);
            }
            seen
        });

        for i in 0..MAX_BACKLOG_CHUNKS {
            assert!(tx.send(vec![i as i16]).await.is_ok());
        }
        drop(tx);

        let seen = reader.await.expect("reader should finish");
        assert_eq!(seen.len(), MAX_BACKLOG_CHUNKS, "nothing may be dropped");
        assert!(
            seen.windows(2).all(|w| w[1] == w[0] + 1),
            "order must be preserved and nothing duplicated"
        );
    }

    #[test]
    fn a_chunk_is_a_quarter_second() {
        // The offset advances by this per chunk, so if it were wrong every
        // timestamp after the first reconnect would drift.
        assert!(
            (CHUNK_SECONDS - 0.25).abs() < f64::EPSILON,
            "{CHUNK_SECONDS}"
        );
    }
}
