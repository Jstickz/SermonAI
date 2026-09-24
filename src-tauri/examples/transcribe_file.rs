//! Stream a WAV to Deepgram and print what comes back, verbatim.
//!
//! The regex detection stage (FR-12, FR-13) has to match references the way
//! they *arrive*, not the way a preacher said them. Whether "Romans eight
//! twenty eight" reaches us as `Romans 8 28`, `Romans 828`, `Romans 8:28` or
//! the words spelled out decides what the scanner has to handle, and no amount
//! of reasoning about it substitutes for looking. The test sermon has spoken
//! references in it; this shows how Nova-3 renders them.
//!
//! ```text
//! cargo run --example transcribe_file -- 150
//! ```
//!
//! Streams the first N seconds of `%TEMP%\sermonai-test-sermon.wav` at
//! real-time pace and prints each settled utterance. Costs N seconds of
//! Deepgram at the usual rate.

use std::time::{Duration, Instant};

use sermonai_lib::credentials::{Access, Secret};
use sermonai_lib::stt::deepgram::{default_endpoint, DeepgramSession, TranscriptEvent};
use sermonai_lib::stt::vocabulary;

const CHUNK_SAMPLES: usize = 4_000;

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();
    let seconds: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(150);

    let Ok(key) = std::env::var("DEEPGRAM_API_KEY") else {
        eprintln!("DEEPGRAM_API_KEY is not set.");
        std::process::exit(2);
    };
    let access = Access::DirectKey(Secret::new(key.trim()));

    let wav = std::env::var("TEMP").unwrap_or_default() + "/sermonai-test-sermon.wav";
    let speech = match load_wav(&wav) {
        Ok(s) => s,
        Err(err) => {
            eprintln!("could not read {wav}: {err}");
            std::process::exit(2);
        }
    };

    let session = DeepgramSession::connect_to(
        default_endpoint(),
        &access,
        vocabulary::keyterms(),
        Box::new(|event| {
            // Finals only. Interims are revisions of the same words and would
            // print each sentence several times over.
            if let TranscriptEvent::Final { text, words, .. } = event {
                let at = words.first().map(|w| w.start).unwrap_or(0.0);
                println!("[{at:6.1}s] {text}");
            }
        }),
    )
    .await
    .unwrap_or_else(|err| {
        eprintln!("could not connect: {err}");
        std::process::exit(1);
    });

    eprintln!("streaming {seconds}s to {} ...", default_endpoint());
    let started = Instant::now();
    for (i, chunk) in speech
        .chunks(CHUNK_SAMPLES)
        .take((seconds * 4) as usize)
        .enumerate()
    {
        session.send_awaiting(chunk.to_vec()).await;
        let due = started + Duration::from_millis((i as u64 + 1) * 250);
        tokio::time::sleep_until(tokio::time::Instant::from_std(due)).await;
    }

    session.finish().await;
    tokio::time::sleep(Duration::from_secs(2)).await;
}

fn load_wav(path: &str) -> Result<Vec<i16>, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    if bytes.len() < 46 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("not a RIFF/WAVE file".into());
    }
    let channels = u16::from_le_bytes([bytes[22], bytes[23]]);
    let rate = u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]);
    let bits = u16::from_le_bytes([bytes[34], bytes[35]]);
    if (channels, rate, bits) != (1, 16_000, 16) {
        return Err(format!(
            "expected 16 kHz mono 16-bit, found {rate} Hz, {channels} ch, {bits}-bit"
        ));
    }
    Ok(bytes[46..]
        .chunks_exact(2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]))
        .collect())
}
