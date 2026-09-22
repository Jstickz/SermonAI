//! Input level metering at 30 fps in dBFS (FR-04, PRD §13.2, §18.4).
//!
//! ## Why this taps the device and not the chunk stream
//!
//! The obvious place to measure level is where the finished 250 ms chunks come
//! out — the audio is already mono, already 16 kHz, already converted. That
//! would give **four updates a second**. The PRD asks for thirty, and it is
//! right to: a meter that moves four times a second cannot tell an operator
//! whether the preacher's mic is live, because it lags the room badly enough
//! to feel broken.
//!
//! So metering happens on the raw device buffers instead, which arrive every
//! few milliseconds, before downmixing and resampling. That is also more
//! truthful: it shows what the device is actually sending, so a clipping input
//! reads as clipping rather than as whatever survived conversion.
//!
//! ## Peak, and why nothing is missed between frames
//!
//! Buffers arrive faster than 30 fps, so most are not emitted. Their peaks are
//! still **held** and folded into the next emission, rather than sampling
//! whichever buffer happens to land on the tick. A transient — a dropped mic, a
//! snare — lasts a few milliseconds, so sampling would miss it most of the
//! time, and a meter that misses clipping is worse than none: it says the input
//! is safe while the recording distorts.

use std::time::{Duration, Instant};

/// 30 fps (PRD §13.2). Roughly 33 ms between emissions.
const FRAME_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / 30);

/// Quietest level the meter shows. Below this it reads as silence.
///
/// -60 dBFS is far below anything a service produces and comfortably under
/// room tone, so the bar sits at the floor when nothing is happening instead
/// of twitching at noise.
pub const FLOOR_DBFS: f32 = -60.0;

/// Accumulates peaks and releases a level 30 times a second.
pub struct LevelMeter {
    /// Linear peak since the last emission, held so transients survive.
    peak: f32,
    last_emit: Instant,
    interval: Duration,
}

impl Default for LevelMeter {
    fn default() -> Self {
        Self::new()
    }
}

impl LevelMeter {
    pub fn new() -> Self {
        Self::with_interval(FRAME_INTERVAL)
    }

    /// Used by tests to avoid sleeping through real frame intervals.
    pub fn with_interval(interval: Duration) -> Self {
        Self {
            peak: 0.0,
            // A frame behind, so the first buffer emits immediately and the
            // meter comes alive the moment capture starts rather than a frame
            // later.
            last_emit: Instant::now() - interval,
            interval,
        }
    }

    /// Feed raw device samples. Returns a level in dBFS when a frame is due.
    ///
    /// Called on the audio thread, so it allocates nothing and does no I/O.
    pub fn push(&mut self, samples: &[f32]) -> Option<f32> {
        for sample in samples {
            let magnitude = sample.abs();
            if magnitude > self.peak {
                self.peak = magnitude;
            }
        }

        self.emit_if_due(Instant::now())
    }

    fn emit_if_due(&mut self, now: Instant) -> Option<f32> {
        if now.duration_since(self.last_emit) < self.interval {
            return None;
        }

        let level = to_dbfs(self.peak);
        self.peak = 0.0;
        self.last_emit = now;
        Some(level)
    }
}

/// Linear amplitude to dBFS, floored and clamped.
///
/// Full scale is 0 dBFS. Above it is clipping, which is reported as 0 rather
/// than as a positive number: the meter's job is to show the input has hit the
/// ceiling, and how far past it went is not actionable.
pub fn to_dbfs(peak: f32) -> f32 {
    if peak <= 0.0 {
        return FLOOR_DBFS;
    }
    (20.0 * peak.log10()).clamp(FLOOR_DBFS, 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Emit on every push, so the tests measure levels and not the clock.
    fn instant_meter() -> LevelMeter {
        LevelMeter::with_interval(Duration::ZERO)
    }

    #[test]
    fn full_scale_reads_zero_and_silence_reads_the_floor() {
        assert_eq!(to_dbfs(1.0), 0.0);
        assert_eq!(to_dbfs(0.0), FLOOR_DBFS);
        // Well below the floor, not a large negative number.
        assert_eq!(to_dbfs(0.000_001), FLOOR_DBFS);
    }

    #[test]
    fn halving_the_amplitude_drops_about_six_db() {
        // The rule of thumb every audio operator knows; if this is wrong the
        // meter will not match the desk they are reading it against.
        assert!((to_dbfs(0.5) - -6.02).abs() < 0.01, "{}", to_dbfs(0.5));
        assert!((to_dbfs(0.25) - -12.04).abs() < 0.01, "{}", to_dbfs(0.25));
    }

    #[test]
    fn clipping_reports_the_ceiling_rather_than_a_positive_level() {
        // A hot desk feed beyond full scale. The meter should peg, not invent
        // headroom above 0 dBFS that no display would know what to do with.
        assert_eq!(to_dbfs(4.0), 0.0);
    }

    #[test]
    fn a_negative_swing_counts_as_much_as_a_positive_one() {
        // Audio is symmetric around zero, and a waveform can clip on the way
        // down first. Measuring only positive samples would under-read it.
        let mut meter = instant_meter();
        let level = meter.push(&[-1.0, 0.1]).expect("should emit");
        assert_eq!(level, 0.0);
    }

    #[test]
    fn a_transient_between_frames_is_held_rather_than_missed() {
        // The reason peaks accumulate instead of being sampled. A loud buffer
        // arrives, then several quiet ones, and only then does a frame fall
        // due. The loud moment must still be what the operator sees.
        let mut meter = LevelMeter::with_interval(Duration::from_millis(50));

        // Consume the initial due frame so the interval is genuinely running.
        meter.push(&[0.0]);

        assert_eq!(meter.push(&[1.0]), None, "not due yet");
        for _ in 0..5 {
            assert_eq!(meter.push(&[0.001]), None, "still not due");
        }

        std::thread::sleep(Duration::from_millis(55));
        let level = meter
            .push(&[0.001])
            .expect("should emit after the interval");
        assert_eq!(level, 0.0, "the transient should survive, not the quiet");
    }

    #[test]
    fn the_peak_resets_after_each_frame_so_the_meter_can_fall() {
        // Without the reset the bar would only ever rise, which reads as a
        // permanently hot input and hides a mic that has actually gone dead.
        let mut meter = instant_meter();

        assert_eq!(meter.push(&[1.0]).expect("emits"), 0.0);
        let quiet = meter.push(&[0.01]).expect("emits");
        assert!(quiet < -30.0, "expected the meter to fall, got {quiet}");
    }

    #[test]
    fn the_first_buffer_emits_immediately() {
        // The meter should come alive the moment capture starts. Waiting a
        // frame is only 33 ms, but it is 33 ms of a dead-looking meter right
        // when the operator is checking whether the input works.
        let mut meter = LevelMeter::new();
        assert!(meter.push(&[0.5]).is_some());
    }

    #[test]
    fn frames_are_not_emitted_faster_than_thirty_a_second() {
        // Each emission crosses the IPC boundary and re-renders the top bar.
        // Emitting per buffer would be hundreds a second for no visible gain.
        let mut meter = LevelMeter::new();
        meter.push(&[0.5]).expect("first buffer emits");

        let mut emitted = 0;
        let start = Instant::now();
        while start.elapsed() < Duration::from_millis(100) {
            if meter.push(&[0.5]).is_some() {
                emitted += 1;
            }
        }

        // 100 ms at 30 fps is three frames, allowing one either side for
        // scheduling jitter on a busy machine.
        assert!(
            (2..=4).contains(&emitted),
            "expected about 3 frames in 100 ms, got {emitted}"
        );
    }
}
