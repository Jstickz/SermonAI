//! Does the capture pipeline produce exactly one second of audio per second?
//!
//! A 60-minute DoD run reported lag of 1490/1926/2445 ms where a 38-second run
//! had reported 452/681/698. Lag is measured as *wall clock since the first
//! chunk was sent, minus Deepgram's audio timestamp* — so anything that makes
//! our audio timeline fall behind the wall clock is indistinguishable from real
//! latency, and, unlike latency, it **accumulates**. Two seconds of drift over
//! an hour would produce exactly the numbers observed.
//!
//! There are two candidate causes and they need different fixes:
//!
//! 1. **Dropped chunks.** `AudioFeed::send` uses `try_send` and logs
//!    "transcription is not keeping up". Each drop removes 250 ms from the
//!    audio timeline permanently — and loses words from the sermon, which
//!    matters more than the metric.
//! 2. **Clock drift.** A sound card that reports 48000 Hz and actually runs at
//!    47 974 delivers fewer samples than real time. Only ~9 samples per second
//!    short of 16 000 — 0.055% — accounts for the whole two seconds.
//!
//! This harness distinguishes them without needing Deepgram, a network or an
//! hour: it counts what the converter emits and compares it against the clock.
//!
//! ```text
//! cargo run --example capture_rate -- "Speakers (Realtek(R) Audio)" 180
//! ```
//!
//! Play audio through the device while it runs — a silent loopback endpoint
//! delivers nothing on Windows and the run will say so rather than report 100%
//! drift.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use sermonai_lib::audio::capture;
use sermonai_lib::audio::convert::CHUNK_SAMPLES;

fn main() {
    let mut args = std::env::args().skip(1);
    let device = args.next().unwrap_or_else(|| {
        eprintln!("usage: capture_rate <device name> [seconds]");
        eprintln!("\nAvailable devices:");
        for d in sermonai_lib::audio::devices::list_devices().unwrap_or_default() {
            // The native rate matters here. 48 kHz resamples to 16 by an exact
            // 3:1; 44.1 kHz is 441:160, and a resampler that handles the ratio
            // imprecisely at chunk boundaries loses a little audio on every
            // one — which would show up as "drift" and accumulate.
            eprintln!(
                "  {:?}  {}  ({} Hz, {} ch)",
                d.kind, d.name, d.default_sample_rate, d.channels
            );
        }
        std::process::exit(2);
    });
    let seconds: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(180);

    let samples = Arc::new(AtomicU64::new(0));
    let chunks = Arc::new(AtomicU64::new(0));

    // Measured between the FIRST and LAST chunk boundary, not from start to
    // stop. A run that ends mid-chunk leaves up to 250 ms unaccounted for, and
    // over three minutes that is 0.14% — coarser than the 0.055% effect being
    // looked for, so the first version of this harness could not have answered
    // its own question. Between boundaries the audio interval is exactly
    // (n - 1) x 250 ms and the only error left is callback jitter.
    //
    // Anchoring on the first chunk rather than on `spawn` matters for the same
    // reason it mattered in the lag metric: opening a device takes time that is
    // not drift, and charging it here would manufacture the effect.
    let marks: Arc<std::sync::Mutex<Option<(Instant, Instant)>>> =
        Arc::new(std::sync::Mutex::new(None));

    let (s, c, m) = (
        Arc::clone(&samples),
        Arc::clone(&chunks),
        Arc::clone(&marks),
    );

    let handle = capture::spawn(
        device.clone(),
        Box::new(move |chunk: Vec<i16>| {
            let now = Instant::now();
            let mut marks = m.lock().unwrap();
            match marks.as_mut() {
                None => *marks = Some((now, now)),
                Some((_, last)) => *last = now,
            }
            s.fetch_add(chunk.len() as u64, Ordering::Relaxed);
            c.fetch_add(1, Ordering::Relaxed);
        }),
        Box::new(|_level| {}),
        Box::new(|err| eprintln!("capture error: {err}")),
    )
    .unwrap_or_else(|err| {
        eprintln!("could not open {device}: {err}");
        std::process::exit(1);
    });

    println!("Capturing from {device} for {seconds}s. Play audio through it now.");
    std::thread::sleep(Duration::from_secs(seconds));

    let span = *marks.lock().unwrap();
    let total_samples = samples.load(Ordering::Relaxed);
    let total_chunks = chunks.load(Ordering::Relaxed);
    drop(handle);

    let Some((first, last)) = span else {
        eprintln!("\nNo audio arrived at all. On Windows a loopback endpoint delivers");
        eprintln!("nothing while it is idle — play something through it and retry.");
        std::process::exit(1);
    };
    if total_chunks < 2 {
        eprintln!("\nToo few chunks to measure a rate. Run for longer.");
        std::process::exit(1);
    }

    let wall = last.duration_since(first).as_secs_f64();
    // Intervals, not chunks: n chunks bound n-1 intervals of 250 ms each, and
    // `wall` spans exactly those.
    let audio = (total_chunks - 1) as f64 * CHUNK_SAMPLES as f64 / 16_000.0;
    let ratio = audio / wall;
    let drift_ms_per_hour = (1.0 - ratio) * 3600.0 * 1000.0;

    // Callback jitter on the first and last chunk is the remaining error, and
    // it is bounded by roughly one device buffer at each end.
    let resolution_ms_per_hour = 0.020 / wall * 3600.0 * 1000.0;

    println!("\n  chunks            {total_chunks} ({CHUNK_SAMPLES} samples each)");
    println!("  samples           {total_samples}");
    println!("  audio spanned     {audio:.3} s   (between first and last chunk)");
    println!("  wall clock        {wall:.3} s");
    println!("  ratio             {ratio:.6}  (1.000000 is perfect)");
    println!("  implied drift     {drift_ms_per_hour:+.0} ms per hour");
    println!("  resolution        +/-{resolution_ms_per_hour:.0} ms per hour");

    // The 60-minute run needs about +2000 ms/hour explained. Sign matters:
    // audio running BEHIND the clock inflates the measured lag, audio running
    // ahead would deflate it.
    println!();
    if resolution_ms_per_hour > 500.0 {
        println!("VERDICT: inconclusive — this run is too short to resolve the effect.");
        println!("  Run for longer; resolution improves in proportion to the duration.");
    } else if drift_ms_per_hour > 500.0 {
        println!(
            "VERDICT: the audio timeline falls BEHIND the clock by {drift_ms_per_hour:.0} ms/hour."
        );
        println!("  That is measured as lag and accumulates, so the DoD figure grows with");
        println!("  session length rather than describing the pipeline.");
    } else {
        println!("VERDICT: the audio timeline keeps up with the clock.");
        println!("  Drift does not explain the 60-minute lag figures. Look at dropped");
        println!("  chunks instead: grep the run's log for \"not keeping up\".");
    }
}
