//! Turning whatever a device gives us into what the transcriber wants (FR-03).
//!
//! Deepgram and whisper.cpp both take **16 kHz mono 16-bit PCM**, so converting
//! once here means nothing downstream resamples (PRD §10.2). Devices rarely
//! offer that natively — the machines tested so far all report 48 kHz, and
//! 44.1 kHz is just as common — so nearly every service runs through this path.
//!
//! ## Why a resampler and not a divide
//!
//! 48 kHz to 16 kHz is exactly one sample in three, which makes dropping the
//! other two look reasonable. It is not. Dropping samples without first
//! removing everything above the new Nyquist frequency of 8 kHz **folds** that
//! content back into the speech band: a 12 kHz sibilant reappears as a 4 kHz
//! tone that was never spoken. Fricatives and cymbals carry real energy up
//! there, so the damage lands exactly where consonants are distinguished, and
//! it degrades recognition rather than announcing itself. It would also pass
//! every obvious test — the audio is the right length, the right rate, and
//! sounds roughly right.
//!
//! `rubato` does the band-limiting properly. `aliasing_is_removed_not_folded`
//! below is the test that holds this honest.
//!
//! 44.1 kHz settles the argument anyway: 441:160 is not an integer ratio, so
//! there is no sample-dropping shortcut to be tempted by.

use rubato::{FftFixedIn, Resampler};

use super::{CHANNELS, CHUNK_MS, SAMPLE_RATE_HZ};
use crate::error::{Error, Result};

/// Samples in one 250 ms chunk at 16 kHz mono: 4,000.
pub const CHUNK_SAMPLES: usize = (SAMPLE_RATE_HZ as usize * CHUNK_MS as usize) / 1000;

/// How much source audio to hand the resampler at a time.
///
/// 1,024 frames is about 21 ms at 48 kHz — small enough that a chunk is never
/// waiting long on the resampler, large enough that the FFT is not being set up
/// for a handful of samples.
const RESAMPLER_CHUNK_IN: usize = 1024;

/// Converts a device's stream into 250 ms chunks of 16 kHz mono PCM.
///
/// Feed it whatever the device produces with [`push`](Self::push); take back
/// complete chunks. Anything left over is held until the next call, so no
/// sample is dropped at a buffer boundary — [`flush`](Self::flush) at the end
/// of capture returns the remainder.
pub struct CaptureConverter {
    channels: usize,
    /// `None` when the device already runs at 16 kHz and there is nothing to do.
    resampler: Option<FftFixedIn<f32>>,
    /// Mono at the source rate, waiting for enough frames to resample.
    pending_source: Vec<f32>,
    /// Mono at 16 kHz, waiting to fill a chunk.
    pending_output: Vec<i16>,
}

impl CaptureConverter {
    pub fn new(source_rate: u32, channels: u16) -> Result<Self> {
        if channels == 0 {
            return Err(Error::Audio(
                "the device reports no audio channels. Choose another input in Settings."
                    .to_string(),
            ));
        }
        if source_rate == 0 {
            return Err(Error::Audio(
                "the device reports no sample rate. Choose another input in Settings.".to_string(),
            ));
        }

        let resampler = if source_rate == SAMPLE_RATE_HZ {
            None
        } else {
            Some(
                FftFixedIn::<f32>::new(
                    source_rate as usize,
                    SAMPLE_RATE_HZ as usize,
                    RESAMPLER_CHUNK_IN,
                    // Sub-chunks trade latency against FFT size. Two is
                    // rubato's own suggestion for streaming and keeps the
                    // worst-case delay well inside one 250 ms chunk.
                    2,
                    // Mono: channels are mixed down before resampling, which is
                    // both cheaper and what the transcriber wants anyway.
                    1,
                )
                .map_err(|e| Error::Audio(format!("could not set up resampling: {e}")))?,
            )
        };

        Ok(Self {
            channels: channels as usize,
            resampler,
            pending_source: Vec::new(),
            pending_output: Vec::new(),
        })
    }

    /// Feed interleaved samples from the device; take back any complete chunks.
    ///
    /// Returns however many 250 ms chunks the new audio completed — usually
    /// none or one, but a large buffer can complete several.
    pub fn push(&mut self, interleaved: &[f32]) -> Result<Vec<Vec<i16>>> {
        downmix_into(interleaved, self.channels, &mut self.pending_source);

        match &mut self.resampler {
            None => {
                // Already 16 kHz: straight to PCM.
                for sample in self.pending_source.drain(..) {
                    self.pending_output.push(to_pcm16(sample));
                }
            }
            Some(resampler) => loop {
                let needed = resampler.input_frames_next();
                if self.pending_source.len() < needed {
                    break;
                }

                let input: Vec<f32> = self.pending_source.drain(..needed).collect();
                let resampled = resampler
                    .process(&[input], None)
                    .map_err(|e| Error::Audio(format!("resampling failed: {e}")))?;

                for sample in &resampled[0] {
                    self.pending_output.push(to_pcm16(*sample));
                }
            },
        }

        let mut chunks = Vec::new();
        while self.pending_output.len() >= CHUNK_SAMPLES {
            chunks.push(self.pending_output.drain(..CHUNK_SAMPLES).collect());
        }
        Ok(chunks)
    }

    /// Whatever has not filled a chunk, at the end of capture.
    ///
    /// A partial chunk still matters: it is the last word of the sermon, and
    /// silently discarding up to 250 ms would clip it.
    pub fn flush(&mut self) -> Vec<i16> {
        std::mem::take(&mut self.pending_output)
    }
}

/// Mix interleaved channels down to mono by averaging, appending to `out`.
///
/// Averaging rather than taking the first channel: a sound desk often feeds a
/// signal to one side only, and picking a channel blind would give silence
/// roughly half the time.
fn downmix_into(interleaved: &[f32], channels: usize, out: &mut Vec<f32>) {
    if channels == 1 {
        out.extend_from_slice(interleaved);
        return;
    }

    // A trailing partial frame is dropped: it is not a whole sample across all
    // channels yet, and cpal delivers the remainder at the start of the next
    // buffer.
    for frame in interleaved.chunks_exact(channels) {
        let sum: f32 = frame.iter().sum();
        out.push(sum / channels as f32);
    }
}

/// Float sample to 16-bit PCM.
///
/// Clamped before scaling: a hot sound desk feed can exceed ±1.0, and letting
/// that wrap would turn a loud passage into a burst of noise rather than
/// something merely loud.
fn to_pcm16(sample: f32) -> i16 {
    (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
}

/// The format every chunk is in, asserted once so the constants cannot drift.
const _: () = {
    assert!(CHUNK_SAMPLES == 4000);
    assert!(CHANNELS == 1);
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    /// A sine at `freq` sampled at `rate`, `seconds` long.
    fn tone(freq: f32, rate: u32, seconds: f32) -> Vec<f32> {
        let count = (rate as f32 * seconds) as usize;
        (0..count)
            .map(|i| (TAU * freq * i as f32 / rate as f32).sin())
            .collect()
    }

    /// Energy at `freq` in `samples`, by correlating against that frequency.
    /// Enough to tell "this tone is present" from "this tone is not".
    fn energy_at(samples: &[i16], freq: f32, rate: u32) -> f32 {
        let (mut re, mut im) = (0.0f64, 0.0f64);
        for (i, s) in samples.iter().enumerate() {
            let phase = TAU as f64 * freq as f64 * i as f64 / rate as f64;
            let v = *s as f64 / i16::MAX as f64;
            re += v * phase.cos();
            im += v * phase.sin();
        }
        ((re * re + im * im).sqrt() / samples.len() as f64) as f32
    }

    fn drain_all(conv: &mut CaptureConverter, input: &[f32]) -> Vec<i16> {
        let mut out: Vec<i16> = conv
            .push(input)
            .expect("conversion should succeed")
            .into_iter()
            .flatten()
            .collect();
        out.extend(conv.flush());
        out
    }

    #[test]
    fn chunks_are_exactly_250_ms_of_16_khz_mono() {
        assert_eq!(CHUNK_SAMPLES, 4000);

        let mut conv = CaptureConverter::new(48_000, 1).unwrap();
        // One second in: three seconds' worth of source frames at 48 kHz.
        let chunks = conv.push(&tone(440.0, 48_000, 1.0)).unwrap();

        assert!(!chunks.is_empty(), "a second of audio should fill chunks");
        for chunk in &chunks {
            assert_eq!(chunk.len(), CHUNK_SAMPLES);
        }
    }

    #[test]
    fn a_second_in_is_a_second_out_whatever_the_source_rate() {
        // Within one chunk: the resampler holds a little audio internally, and
        // the exact amount depends on its FFT size.
        for rate in [44_100, 48_000, 96_000, 16_000] {
            let mut conv = CaptureConverter::new(rate, 1).unwrap();
            let out = drain_all(&mut conv, &tone(440.0, rate, 1.0));

            let expected = SAMPLE_RATE_HZ as i64;
            let drift = (out.len() as i64 - expected).abs();
            assert!(
                drift < CHUNK_SAMPLES as i64,
                "{rate} Hz produced {} samples, expected about {expected}",
                out.len()
            );
        }
    }

    #[test]
    fn stereo_is_averaged_not_half_ignored() {
        // Left carries the signal, right is silent — a sound desk feeding one
        // side, which is common. Averaging halves it; picking a channel blind
        // would give either full scale or nothing.
        let mut interleaved = Vec::new();
        for s in tone(440.0, 16_000, 0.5) {
            interleaved.push(s);
            interleaved.push(0.0);
        }

        let mut conv = CaptureConverter::new(16_000, 2).unwrap();
        let out = drain_all(&mut conv, &interleaved);

        let peak = out.iter().map(|s| s.unsigned_abs()).max().unwrap();
        let half = i16::MAX as u32 / 2;
        assert!(
            (peak as u32).abs_diff(half) < half / 5,
            "expected about half scale, got {peak}"
        );
    }

    /// The test that justifies the dependency.
    ///
    /// A 12 kHz tone cannot exist at 16 kHz — it is above the 8 kHz Nyquist
    /// limit. Decimating without band-limiting first would fold it to
    /// |16000 - 12000| = 4 kHz, putting a loud tone in the middle of the
    /// speech band that nobody produced. Proper resampling removes it instead.
    #[test]
    fn aliasing_is_removed_not_folded() {
        let mut conv = CaptureConverter::new(48_000, 1).unwrap();
        let out = drain_all(&mut conv, &tone(12_000.0, 48_000, 0.5));

        let folded = energy_at(&out, 4_000.0, SAMPLE_RATE_HZ);

        // For scale: the same measurement on a tone that really is at 4 kHz.
        let mut reference = CaptureConverter::new(48_000, 1).unwrap();
        let real = drain_all(&mut reference, &tone(4_000.0, 48_000, 0.5));
        let genuine = energy_at(&real, 4_000.0, SAMPLE_RATE_HZ);

        assert!(
            folded < genuine / 10.0,
            "12 kHz folded back to 4 kHz at {folded} against {genuine} for a real 4 kHz tone; \
             the resampler is not band-limiting"
        );
    }

    /// A 1 kHz tone is comfortably inside the speech band and must survive.
    /// The counterpart to the test above: band-limiting that removed
    /// everything would pass that one and fail this.
    #[test]
    fn speech_band_content_survives_resampling() {
        let mut conv = CaptureConverter::new(48_000, 1).unwrap();
        let out = drain_all(&mut conv, &tone(1_000.0, 48_000, 0.5));

        let kept = energy_at(&out, 1_000.0, SAMPLE_RATE_HZ);
        assert!(kept > 0.2, "a 1 kHz tone should come through, got {kept}");

        let peak = out.iter().map(|s| s.unsigned_abs()).max().unwrap();
        assert!(peak > i16::MAX as u16 / 2, "amplitude lost: peak {peak}");
    }

    #[test]
    fn samples_are_not_lost_across_push_boundaries() {
        // The same audio in one push and in many awkward-sized ones should
        // produce the same output, or a buffer boundary is eating samples.
        let source = tone(440.0, 48_000, 1.0);

        let mut whole = CaptureConverter::new(48_000, 1).unwrap();
        let in_one_go = drain_all(&mut whole, &source);

        let mut split = CaptureConverter::new(48_000, 1).unwrap();
        let mut piecemeal = Vec::new();
        // 577 is deliberately coprime with the resampler's chunk size, so
        // pushes never line up with its internal boundaries.
        for part in source.chunks(577) {
            piecemeal.extend(split.push(part).unwrap().into_iter().flatten());
        }
        piecemeal.extend(split.flush());

        assert_eq!(in_one_go, piecemeal);
    }

    #[test]
    fn loud_input_clips_rather_than_wrapping() {
        // A hot desk feed beyond full scale. Wrapping would turn the loudest
        // moment of a service into noise; clamping keeps it merely loud.
        let hot: Vec<f32> = (0..16_000)
            .map(|i| if i % 2 == 0 { 4.0 } else { -4.0 })
            .collect();

        let mut conv = CaptureConverter::new(16_000, 1).unwrap();
        let out = drain_all(&mut conv, &hot);

        for sample in out {
            assert!(
                sample == i16::MAX || sample == -i16::MAX,
                "expected clipping to the rails, got {sample}"
            );
        }
    }

    #[test]
    fn a_device_reporting_nonsense_is_refused_with_a_way_out() {
        for err in [
            CaptureConverter::new(48_000, 0).err(),
            CaptureConverter::new(0, 2).err(),
        ] {
            let message = err.expect("should be refused").to_string();
            assert!(message.contains("Settings"), "{message}");
        }
    }
}
