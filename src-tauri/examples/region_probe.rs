//! How long does a word take to come back, from *this* machine, per region?
//!
//! The DoD lag line is failing and the question is what share belongs to us.
//! A TCP handshake gives the round trip, but not what Deepgram does with the
//! audio once it arrives, and not whether a distant region is also a *less
//! stable* one. This streams real audio at real-time pace — exactly as capture
//! does, 250 ms at a time — and records, per interim result, how long after the
//! chunk was sent it came back.
//!
//! That is the same quantity `SessionTranscript` measures, so the two are
//! directly comparable.
//!
//! ```text
//! cargo run --example region_probe -- 60
//! ```
//!
//! Reads `DEEPGRAM_API_KEY` from the environment (the `.env` is loaded for
//! you). Costs one minute of streaming per region at Deepgram's usual rate.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use sermonai_lib::credentials::{Access, Secret};
use sermonai_lib::stt::deepgram::{DeepgramSession, TranscriptEvent};
use sermonai_lib::stt::vocabulary;

/// The endpoints to compare. `api.deepgram.com` resolves to `api.sac1` —
/// Sacramento — for everyone; the EU endpoint is Frankfurt and takes the same
/// API keys.
const REGIONS: &[(&str, &str)] = &[
    ("us (sac1)", "wss://api.deepgram.com/v1/listen"),
    ("eu (frankfurt)", "wss://api.eu.deepgram.com/v1/listen"),
];

const CHUNK_SAMPLES: usize = 4_000; // 250 ms at 16 kHz

#[tokio::main]
async fn main() {
    let _ = dotenvy::dotenv();
    let seconds: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);

    let Ok(key) = std::env::var("DEEPGRAM_API_KEY") else {
        eprintln!("DEEPGRAM_API_KEY is not set.");
        std::process::exit(2);
    };
    let access = Access::DirectKey(Secret::new(key.trim()));

    let wav = std::env::var("TEMP").unwrap_or_default() + "/sermonai-test-sermon.wav";
    let speech = match load_wav(&wav) {
        Ok(samples) => samples,
        Err(err) => {
            eprintln!("could not read {wav}: {err}");
            eprintln!("Generate it with: .\\scripts\\make-test-sermon.ps1 -Minutes 10");
            std::process::exit(2);
        }
    };

    println!("Streaming {seconds}s of real speech to each region.\n");

    for (label, endpoint) in REGIONS {
        match probe(endpoint, &access, seconds, &speech).await {
            Ok(lags) if lags.is_empty() => {
                println!("{label:<16} no interim results came back");
            }
            Ok(samples) => {
                let mut lags: Vec<f64> = samples.iter().map(|(_, lag)| *lag).collect();
                lags.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let at = |q: f64| {
                    let i = ((lags.len() as f64 * q).ceil() as usize).clamp(1, lags.len()) - 1;
                    lags[i] * 1000.0
                };
                println!(
                    "{label:<16} n={:<4} p50 {:4.0}  p95 {:4.0}  p99 {:4.0}  max {:4.0} ms",
                    lags.len(),
                    at(0.50),
                    at(0.95),
                    at(0.99),
                    at(1.0),
                );

                // Per minute of audio. A figure that climbs means something is
                // backing up; one that stays put means the network is simply
                // this far away.
                let minutes = samples.iter().map(|(at, _)| *at / 60.0).fold(0.0, f64::max);
                for m in 0..=(minutes as usize) {
                    let mut bucket: Vec<f64> = samples
                        .iter()
                        .filter(|(at, _)| (*at / 60.0) as usize == m)
                        .map(|(_, lag)| *lag)
                        .collect();
                    if bucket.len() < 3 {
                        continue;
                    }
                    bucket.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    let med = bucket[bucket.len() / 2] * 1000.0;
                    let hi = bucket[bucket.len() - 1] * 1000.0;
                    println!(
                        "    minute {m:<2}  n={:<3}  p50 {med:4.0}  max {hi:4.0} ms",
                        bucket.len()
                    );
                }
            }
            Err(err) => println!("{label:<16} FAILED: {err}"),
        }
    }

    println!("\nLag is measured from the moment each chunk was sent to the moment a");
    println!("result mentioning it came back, which is what the DoD line budgets");
    println!("(plus the fixed 250 ms a chunk spends accumulating).");
}

async fn probe(
    endpoint: &str,
    access: &Access,
    seconds: u64,
    speech: &[i16],
) -> Result<Vec<(f64, f64)>, Box<dyn std::error::Error>> {
    // (audio seconds sent, when it was sent)
    let marks: Arc<Mutex<Vec<(f64, Instant)>>> = Arc::new(Mutex::new(Vec::new()));
    // (audio position of the result, lag in seconds)
    let lags: Arc<Mutex<Vec<(f64, f64)>>> = Arc::new(Mutex::new(Vec::new()));

    let (m, l) = (Arc::clone(&marks), Arc::clone(&lags));
    let session = DeepgramSession::connect_to(
        endpoint,
        access,
        vocabulary::keyterms(),
        Box::new(move |event| {
            let TranscriptEvent::Interim { words, .. } = &event else {
                return;
            };
            let Some(end) = words.last().map(|w| w.end) else {
                return;
            };
            // Same lookup the app uses: the chunk that carried this word.
            let marks = m.lock().unwrap();
            let idx = marks.partition_point(|(audio_at, _)| *audio_at < end);
            if let Some((_, sent_at)) = marks.get(idx) {
                // Keyed by which minute of the run it landed in, so a lag that
                // grows can be told from one that is merely high. That is the
                // difference between something backing up in our pipeline and
                // a slow network, and it is the question the summary
                // percentiles cannot answer.
                l.lock()
                    .unwrap()
                    .push((end, sent_at.elapsed().as_secs_f64()));
            }
        }),
    )
    .await?;

    // Real speech. A synthetic tone produces no interim results at all —
    // Nova-3 returns nothing for something it does not hear as words — so the
    // first version of this probe reported "no results came back" for both
    // regions and proved nothing. This is the same file the DoD run plays.
    let mut audio_sent = 0.0_f64;
    let chunks = (seconds * 4) as usize;

    let started = Instant::now();
    for (i, chunk) in speech.chunks(CHUNK_SAMPLES).take(chunks).enumerate() {
        audio_sent += chunk.len() as f64 / 16_000.0;
        marks.lock().unwrap().push((audio_sent, Instant::now()));
        session.send_awaiting(chunk.to_vec()).await;

        // Real-time pace. Sending faster would let Deepgram answer for audio it
        // received early, which is exactly the effect that makes reconnect
        // catch-up unusable as a measurement.
        let due = started + Duration::from_millis((i as u64 + 1) * 250);
        tokio::time::sleep_until(tokio::time::Instant::from_std(due)).await;
    }

    session.finish().await;
    tokio::time::sleep(Duration::from_secs(2)).await;

    let out = lags.lock().unwrap().clone();
    Ok(out)
}

/// The test sermon: 16 kHz mono 16-bit PCM, exactly what capture produces, so
/// nothing is converted here and no conversion can skew the comparison.
///
/// The header is checked rather than assumed. A stereo or 44.1 kHz file would
/// be streamed at the wrong pace and would quietly halve the measured lag.
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
