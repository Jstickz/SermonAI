//! Deepgram Nova-3 streaming over WebSocket (FR-07, PRD §11.3).
//!
//! Audio goes up as raw PCM frames; transcripts come back as JSON. The parts
//! worth knowing before reading:
//!
//! **Interim results are not drafts of separate sentences — they are revisions
//! of the same one.** Deepgram sends its current best guess for the utterance
//! in progress, repeatedly, each superseding the last, until one arrives with
//! `is_final`. A consumer that appends every result instead of replacing the
//! in-progress one prints the same half-sentence five times, growing. This is
//! the single easiest thing to get wrong here, so [`TranscriptEvent`] makes the
//! distinction impossible to ignore rather than leaving it to a bool.
//!
//! **Nothing downstream may act on interim text.** M2's detection stages run on
//! finals only: a scripture reference heard in an interim result can be revised
//! away a moment later, and a verse already on the projector cannot be.
//!
//! **A silent connection is closed by Deepgram.** It drops a stream after about
//! ten seconds with no audio, so a paused capture needs `KeepAlive` frames or
//! the socket dies during the pause rather than at the end of it.
//!
//! Reconnection with backoff is M1 deliverable 10, not here.

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;

use crate::audio::{CHANNELS, SAMPLE_RATE_HZ};
use crate::credentials::{Access, Credentials, Service};
use crate::error::{Error, Result};

const BASE_URL: &str = "wss://api.deepgram.com/v1/listen";

/// Deepgram closes a stream after roughly ten seconds of silence. Eight leaves
/// room for a late frame without being chatty.
const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(8);

/// Chunks held for the socket before audio is dropped.
///
/// 40 chunks is ten seconds. If the network stalls for longer than that the
/// backlog is already useless — a transcript ten seconds behind the preacher
/// helps nobody — so the oldest is dropped and the operator told, rather than
/// growing a buffer until the app runs out of memory. The audio thread must
/// never block, so this is a `try_send` and never a `send`.
const AUDIO_QUEUE_CHUNKS: usize = 40;

/// One word, with where it falls in the service.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Word {
    /// The punctuated, capitalised form where Deepgram supplies one. That is
    /// what a reader wants; the raw form differs only in casing and commas.
    pub word: String,
    /// Seconds from the start of the stream.
    pub start: f64,
    pub end: f64,
    pub confidence: f32,
}

/// What the stream produced.
///
/// Two variants rather than a struct with an `is_final` flag, because the
/// difference is not a detail of the same thing: one replaces what came before
/// it and must not be acted on, the other is settled and may be.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum TranscriptEvent {
    /// Deepgram's current guess at the utterance in progress. **Replaces** any
    /// previous interim; never append. Do not detect scripture in it.
    Interim { text: String, words: Vec<Word> },
    /// Settled. Append this, and let detection run on it.
    Final {
        text: String,
        words: Vec<Word>,
        /// Deepgram judged the speaker to have finished an utterance, not just
        /// this fragment. Useful for deciding when a thought is complete.
        speech_final: bool,
    },
    /// The socket closed. Carries a reason when there is one.
    Closed { reason: Option<String> },
}

impl TranscriptEvent {
    /// Text of a transcript event, empty for `Closed`.
    pub fn text(&self) -> &str {
        match self {
            TranscriptEvent::Interim { text, .. } | TranscriptEvent::Final { text, .. } => text,
            TranscriptEvent::Closed { .. } => "",
        }
    }

    /// Whether downstream stages may act on this (FR-12 onwards).
    pub fn is_actionable(&self) -> bool {
        matches!(self, TranscriptEvent::Final { .. })
    }
}

/// Where transcript events go. Called from the streaming task.
pub type EventSink = Box<dyn FnMut(TranscriptEvent) + Send + 'static>;

/// Query string for a stream.
///
/// Built as a function so it can be asserted in a test: a wrong `encoding` or
/// `sample_rate` here produces a connection that succeeds and transcribes
/// noise, which is far harder to diagnose than a refused connection.
pub fn stream_url(vocabulary: &[String]) -> String {
    let mut url = format!(
        "{BASE_URL}?model=nova-3&language=en&encoding=linear16\
         &sample_rate={SAMPLE_RATE_HZ}&channels={CHANNELS}\
         &interim_results=true&punctuate=true&smart_format=true"
    );

    // FR-09. Deepgram takes one `keyterm` per term, repeated.
    for term in vocabulary {
        let trimmed = term.trim();
        if trimmed.is_empty() {
            continue;
        }
        url.push_str("&keyterm=");
        url.push_str(&urlencode(trimmed));
    }

    url
}

/// Percent-encode a query value.
///
/// Hand-rolled rather than pulling in a crate for one use: the inputs are Bible
/// book names and preaching terms, so the alphabet is small and known.
fn urlencode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            b' ' => out.push_str("%20"),
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// 16-bit samples to the little-endian bytes Deepgram expects.
///
/// Explicitly little-endian, not a transmute of the host's layout. `linear16`
/// means LE on the wire; on a big-endian machine a memory reinterpretation
/// would send byte-swapped audio, which transcribes as silence or noise rather
/// than failing.
pub fn pcm_bytes(samples: &[i16]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(samples.len() * 2);
    for sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    bytes
}

/// A Deepgram response. Only the fields used are named; the API sends more.
#[derive(Debug, Deserialize)]
struct Response {
    #[serde(default)]
    r#type: String,
    #[serde(default)]
    is_final: bool,
    #[serde(default)]
    speech_final: bool,
    #[serde(default)]
    channel: Option<Channel>,
}

#[derive(Debug, Deserialize)]
struct Channel {
    #[serde(default)]
    alternatives: Vec<Alternative>,
}

#[derive(Debug, Deserialize)]
struct Alternative {
    #[serde(default)]
    transcript: String,
    #[serde(default)]
    words: Vec<RawWord>,
}

#[derive(Debug, Deserialize)]
struct RawWord {
    #[serde(default)]
    word: String,
    /// Present when `punctuate` or `smart_format` is on, which they are.
    #[serde(default)]
    punctuated_word: Option<String>,
    #[serde(default)]
    start: f64,
    #[serde(default)]
    end: f64,
    #[serde(default)]
    confidence: f32,
}

/// Turn one WebSocket text frame into an event, or `None` if it carries no
/// transcript.
///
/// Returning `None` for empty transcripts is deliberate. Deepgram sends plenty
/// of results with an empty string during silence, and forwarding them would
/// blank the transcript panel between every phrase.
pub fn parse_message(payload: &str) -> Option<TranscriptEvent> {
    let response: Response = serde_json::from_str(payload).ok()?;

    // Metadata, SpeechStarted and UtteranceEnd carry no transcript.
    if !response.r#type.is_empty() && response.r#type != "Results" {
        return None;
    }

    let alternative = response.channel?.alternatives.into_iter().next()?;
    if alternative.transcript.trim().is_empty() {
        return None;
    }

    let words: Vec<Word> = alternative
        .words
        .into_iter()
        .map(|raw| Word {
            // Prefer the punctuated form: it is what a reader sees, and the
            // plain form would show "lord" mid-sentence where the panel wants
            // "Lord,".
            word: raw.punctuated_word.unwrap_or(raw.word),
            start: raw.start,
            end: raw.end,
            confidence: raw.confidence,
        })
        .collect();

    Some(if response.is_final {
        TranscriptEvent::Final {
            text: alternative.transcript,
            words,
            speech_final: response.speech_final,
        }
    } else {
        TranscriptEvent::Interim {
            text: alternative.transcript,
            words,
        }
    })
}

/// A live transcription stream.
pub struct DeepgramSession {
    audio: mpsc::Sender<Vec<i16>>,
    task: tauri::async_runtime::JoinHandle<()>,
}

impl DeepgramSession {
    /// Open a stream and start transcribing.
    ///
    /// Returns once the socket is connected, so a bad key or a blocked network
    /// surfaces as an error the operator sees at Start rather than as a
    /// transcript that never appears.
    pub async fn connect(
        credentials: &Credentials,
        vocabulary: Vec<String>,
        mut on_event: EventSink,
    ) -> Result<Self> {
        let key = match credentials.access(Service::Deepgram)? {
            Access::DirectKey(secret) => secret,
            // Managed mode proxies the socket through the gateway, which does
            // not exist yet (Phase 3/4 of the strategy doc).
            Access::Gateway { .. } => {
                return Err(Error::Config(
                    "Live transcription through the SermonAI Gateway is not available yet."
                        .to_string(),
                ))
            }
        };

        let mut request = stream_url(&vocabulary)
            .into_client_request()
            .map_err(|e| Error::Stt(format!("could not build the Deepgram request: {e}")))?;
        request.headers_mut().insert(
            "Authorization",
            format!("Token {}", key.expose())
                .parse()
                .map_err(|_| Error::Stt("the Deepgram key is not a valid header".to_string()))?,
        );

        let (socket, _) = tokio_tungstenite::connect_async(request)
            .await
            .map_err(|e| Error::Stt(format!("could not reach Deepgram: {e}")))?;

        let (mut write, mut read) = socket.split();
        let (audio_tx, mut audio_rx) = mpsc::channel::<Vec<i16>>(AUDIO_QUEUE_CHUNKS);

        let task = tauri::async_runtime::spawn(async move {
            let mut keepalive = tokio::time::interval(KEEPALIVE_INTERVAL);
            // The first tick fires immediately, which would send a keepalive
            // before any audio.
            keepalive.tick().await;

            loop {
                tokio::select! {
                    chunk = audio_rx.recv() => match chunk {
                        Some(samples) => {
                            if write.send(Message::Binary(pcm_bytes(&samples))).await.is_err() {
                                break;
                            }
                            // Audio counts as activity, so the idle timer only
                            // matters while capture is paused.
                            keepalive.reset();
                        }
                        // The session was dropped or finished: tell Deepgram to
                        // return whatever it is still holding rather than
                        // cutting the socket and losing the last words.
                        None => {
                            let _ = write
                                .send(Message::Text(r#"{"type":"CloseStream"}"#.to_string()))
                                .await;
                            break;
                        }
                    },

                    message = read.next() => match message {
                        Some(Ok(Message::Text(payload))) => {
                            if let Some(event) = parse_message(&payload) {
                                on_event(event);
                            }
                        }
                        Some(Ok(Message::Close(frame))) => {
                            on_event(TranscriptEvent::Closed {
                                reason: frame.map(|f| f.reason.to_string()),
                            });
                            break;
                        }
                        Some(Ok(_)) => {}
                        Some(Err(err)) => {
                            on_event(TranscriptEvent::Closed {
                                reason: Some(err.to_string()),
                            });
                            break;
                        }
                        None => break,
                    },

                    _ = keepalive.tick() => {
                        if write
                            .send(Message::Text(r#"{"type":"KeepAlive"}"#.to_string()))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                }
            }

            // Drain anything Deepgram sends after CloseStream: the final
            // results for the last utterance arrive here.
            while let Some(Ok(Message::Text(payload))) = read.next().await {
                if let Some(event) = parse_message(&payload) {
                    on_event(event);
                }
            }
        });

        Ok(Self {
            audio: audio_tx,
            task,
        })
    }

    /// Queue a chunk. Never blocks.
    ///
    /// Called from the audio thread, which has a deadline the OS enforces, so a
    /// full queue drops the chunk rather than waiting. Ten seconds of backlog
    /// already means the transcript is useless; the operator needs to know the
    /// connection is failing, not to have the audio held for them.
    pub fn send(&self, chunk: Vec<i16>) {
        if self.audio.try_send(chunk).is_err() {
            tracing::warn!("the Deepgram connection is not keeping up; dropping audio");
        }
    }

    /// Close the stream and wait for the final results.
    pub async fn finish(self) {
        // Dropping the sender is what tells the task to send CloseStream.
        drop(self.audio);
        if let Err(err) = self.task.await {
            tracing::error!(%err, "the Deepgram task did not shut down cleanly");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stream_url_describes_the_audio_we_actually_send() {
        let url = stream_url(&[]);

        // These four have to match audio::convert exactly. A mismatch connects
        // happily and transcribes noise, which is much harder to diagnose than
        // a refused connection.
        assert!(url.contains("encoding=linear16"), "{url}");
        assert!(url.contains("sample_rate=16000"), "{url}");
        assert!(url.contains("channels=1"), "{url}");
        assert!(url.contains("model=nova-3"), "{url}");

        // Without this there are no interim results at all (FR-07).
        assert!(url.contains("interim_results=true"), "{url}");
    }

    #[test]
    fn vocabulary_terms_are_encoded_not_pasted() {
        let url = stream_url(&[
            "1 Thessalonians".to_string(),
            "Song of Solomon".to_string(),
            "  ".to_string(),
        ]);

        assert!(url.contains("keyterm=1%20Thessalonians"), "{url}");
        assert!(url.contains("keyterm=Song%20of%20Solomon"), "{url}");
        // A blank term would send `keyterm=`, which is a malformed request for
        // no benefit.
        assert!(!url.contains("keyterm=&"), "{url}");
        assert!(!url.ends_with("keyterm="), "{url}");
    }

    #[test]
    fn samples_are_little_endian_whatever_the_host_is() {
        // 0x0102 must go out as 02 01. A transmute of host memory would be
        // right on x86 and silently wrong elsewhere, producing audio that
        // transcribes as noise rather than failing.
        assert_eq!(pcm_bytes(&[0x0102]), vec![0x02, 0x01]);
        assert_eq!(pcm_bytes(&[-2]), vec![0xFE, 0xFF]);
        assert_eq!(pcm_bytes(&[0, 1]), vec![0x00, 0x00, 0x01, 0x00]);
    }

    #[test]
    fn an_interim_result_is_parsed_as_revisable() {
        let payload = r#"{
            "type": "Results",
            "is_final": false,
            "speech_final": false,
            "channel": { "alternatives": [{
                "transcript": "turn with me to john",
                "words": [
                    {"word":"turn","punctuated_word":"Turn","start":0.1,"end":0.4,"confidence":0.99},
                    {"word":"john","punctuated_word":"John","start":1.0,"end":1.3,"confidence":0.91}
                ]
            }]}
        }"#;

        let event = parse_message(payload).expect("a transcript should parse");

        match &event {
            TranscriptEvent::Interim { text, words } => {
                assert_eq!(text, "turn with me to john");
                // The punctuated form is preferred: the panel wants "Turn",
                // not "turn", mid-sentence.
                assert_eq!(words[0].word, "Turn");
                assert_eq!(words[1].word, "John");
                assert!((words[1].start - 1.0).abs() < f64::EPSILON);
            }
            other => panic!("expected an interim result, got {other:?}"),
        }

        // The property the rest of the app depends on.
        assert!(!event.is_actionable(), "interim text must not be acted on");
    }

    #[test]
    fn a_final_result_is_parsed_as_settled() {
        let payload = r#"{
            "type": "Results",
            "is_final": true,
            "speech_final": true,
            "channel": { "alternatives": [{
                "transcript": "Turn with me to John 3:16.",
                "words": [{"word":"john","punctuated_word":"John","start":1.0,"end":1.3,"confidence":0.98}]
            }]}
        }"#;

        let event = parse_message(payload).expect("a transcript should parse");

        match &event {
            TranscriptEvent::Final {
                text, speech_final, ..
            } => {
                assert_eq!(text, "Turn with me to John 3:16.");
                assert!(*speech_final);
            }
            other => panic!("expected a final result, got {other:?}"),
        }

        assert!(event.is_actionable(), "final text drives detection");
    }

    #[test]
    fn silence_produces_nothing_rather_than_a_blank_transcript() {
        // Deepgram sends these constantly between phrases. Forwarding them
        // would blank the transcript panel after every sentence.
        let empty = r#"{"type":"Results","is_final":false,
            "channel":{"alternatives":[{"transcript":"","words":[]}]}}"#;
        assert!(parse_message(empty).is_none());

        let whitespace = r#"{"type":"Results","is_final":true,
            "channel":{"alternatives":[{"transcript":"   ","words":[]}]}}"#;
        assert!(parse_message(whitespace).is_none());
    }

    #[test]
    fn non_transcript_messages_are_ignored_without_failing() {
        // The API sends all of these on a normal stream. None is an error, and
        // none carries words.
        for payload in [
            r#"{"type":"Metadata","request_id":"abc","duration":12.5}"#,
            r#"{"type":"SpeechStarted","timestamp":1.25}"#,
            r#"{"type":"UtteranceEnd","last_word_end":3.5}"#,
        ] {
            assert!(parse_message(payload).is_none(), "{payload}");
        }

        // And malformed input is a dropped message, not a panic: a socket is
        // not a place to trust the shape of what arrives.
        assert!(parse_message("not json").is_none());
        assert!(parse_message("{}").is_none());
    }

    #[test]
    fn a_word_missing_its_punctuated_form_still_appears() {
        // punctuated_word is absent if smart_format is ever turned off, and a
        // word vanishing from the transcript would be worse than an unpunctuated
        // one appearing.
        let payload = r#"{"type":"Results","is_final":true,
            "channel":{"alternatives":[{"transcript":"amen",
            "words":[{"word":"amen","start":0.0,"end":0.5,"confidence":0.9}]}]}}"#;

        let event = parse_message(payload).expect("should parse");
        match event {
            TranscriptEvent::Final { words, .. } => assert_eq!(words[0].word, "amen"),
            other => panic!("expected final, got {other:?}"),
        }
    }
}
