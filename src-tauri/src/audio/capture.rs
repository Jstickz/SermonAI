//! Opening a device and turning it into 250 ms chunks (FR-03, PRD §10.2).
//!
//! [`convert`](super::convert) does the format work; this module deals with
//! cpal: which config to ask for, which sample type the device speaks, and what
//! to do when it stops speaking mid-service.
//!
//! ## Two wrinkles worth knowing before reading
//!
//! **The config comes from a different side for loopback.** A WASAPI loopback
//! source is an *output* endpoint, and asking it for an input config returns
//! nothing — see [`devices`](super::devices). `open` takes the flag from
//! `find_device` and reads the matching side.
//!
//! **Devices do not speak f32.** A sample arrives as whatever the hardware
//! uses — `i16` from most interfaces, `f32` from shared-mode WASAPI, `u8` from
//! the occasional cheap capture card — so the stream is built per sample type
//! and converted on the way in. Assuming `f32` would work on the development
//! machine and produce silence or noise on somebody's desk.

use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{FromSample, SizedSample};

use super::convert::{CaptureConverter, CHUNK_SAMPLES};
use super::devices;
use super::meter::LevelMeter;
use crate::error::{Error, Result};

/// Where finished chunks go: 4,000 samples of 16 kHz mono PCM, 250 ms each.
///
/// Called on the audio thread, which must never block — the OS gives it a
/// deadline and missing it drops audio. Send the chunk somewhere and return.
pub type ChunkSink = Box<dyn FnMut(Vec<i16>) + Send + 'static>;

/// Called when the device fails mid-capture, most often by being unplugged.
///
/// A disconnect is an operator event, not a crash: the service is still
/// running, and the app has to say the device is gone and offer another
/// (PRD §10.6).
///
/// `Sync` as well as `Send` because two callers share one sink: cpal's error
/// callback, for a device that raises an error, and the watchdog, for one that
/// simply stops sending. A Bluetooth headset does the second.
pub type ErrorSink = Box<dyn Fn(Error) + Send + Sync + 'static>;

/// Where meter frames go: a level in dBFS, about thirty times a second.
///
/// Fed from the raw device buffers rather than the finished chunks, because
/// chunks arrive four times a second and the PRD asks for thirty — see
/// [`meter`](super::meter).
pub type LevelSink = Box<dyn FnMut(f32) + Send + 'static>;

/// The converter and the chunk sink, reachable from both the audio callback
/// and the stop path.
///
/// A mutex touched by an audio callback is normally a mistake: the OS gives
/// that thread a deadline, and blocking on a lock someone else holds misses it
/// and drops audio. It is safe here because **the two never run at once**.
/// Every lock but one is taken by the callback itself; the exception is the
/// final flush, and stop pauses the stream first, after which cpal makes no
/// further callbacks. So the lock is uncontended by construction rather than
/// by luck.
struct Shared {
    converter: CaptureConverter,
    sink: ChunkSink,
}

/// How long a running device may deliver nothing before it counts as lost.
///
/// **A silent device is not a quiet room.** cpal delivers buffers whether or
/// not anyone is speaking, so a gap in callbacks means the device stopped
/// producing, not that the preacher paused. That is what makes a watchdog
/// sound here rather than a guess.
///
/// It exists because a stream error is not reliable. A Bluetooth headset
/// disconnecting mid-service does not necessarily raise one: WASAPI can keep
/// the endpoint valid and simply stop delivering, so the app carries on
/// believing it is recording. That is exactly what happened in testing.
///
/// Two seconds is eight of our 250 ms chunks — long enough that a scheduling
/// hiccup on a busy machine does not trip it, short enough that an operator
/// finds out while the sermon is still recoverable.
const SILENCE_TIMEOUT: Duration = Duration::from_secs(2);

/// Grace after starting or resuming, before the watchdog applies.
///
/// A Bluetooth device can take a moment to deliver its first buffer, and
/// declaring it lost before it has spoken once would make it unusable.
const STARTUP_GRACE: Duration = Duration::from_secs(3);

/// How often the capture thread wakes to check the watchdog when no command
/// has arrived.
const WATCHDOG_TICK: Duration = Duration::from_millis(250);

/// One chunk, in milliseconds. Mirrors `CHUNK_SAMPLES` at 16 kHz (FR-03), and
/// deliberately equal to `WATCHDOG_TICK`: the watchdog then tops up at most one
/// chunk of silence per wake, so it paces itself instead of bursting.
const CHUNK_MS: u64 = 250;

/// How many chunks of silence are owed, given how long the timeline has been
/// running and how many chunks it has produced.
///
/// Pure so it can be tested without a sound card. The three ways to get it
/// wrong are injecting silence into continuous speech, never catching up at
/// all, and catching up so fast that a gap is filled in one burst.
fn silence_chunks_due(elapsed_ms: u64, produced: u64) -> u64 {
    let due = elapsed_ms / CHUNK_MS;
    // One chunk of slack. A device delivering normally is always a fraction of
    // a chunk behind the clock, and synthesising into that gap would inject
    // silence in the middle of speech.
    due.saturating_sub(produced + 1)
}

/// Whether a running device should be treated as lost.
///
/// Extracted so the decision can be tested without hardware: the three ways to
/// get this wrong are firing while paused, firing before a slow device has
/// spoken once, and never firing at all.
///
/// `running_for` is `None` while paused, which is the case that matters most —
/// a paused device is silent on purpose, and reporting every pause as a
/// disconnection would train the operator to ignore the warning.
fn device_lost(
    running_for: Option<Duration>,
    since_last_data: Duration,
    is_output_endpoint: bool,
) -> bool {
    // A loopback source is exempt, because silence there is normal rather than
    // evidence. Measured on this machine: of three loopback endpoints, two
    // deliver *nothing at all* while the system is playing nothing — including
    // "Speakers (Realtek)", which is what an operator picks to capture a desk
    // feed. The watchdog called both lost within two seconds of the music
    // stopping. A church between songs would be told its input had failed.
    //
    // Genuine removal of a loopback endpoint — a monitor unplugged — still
    // raises a cpal stream error, which is handled on its own path.
    if is_output_endpoint {
        return false;
    }

    match running_for {
        None => false,
        Some(elapsed) if elapsed < STARTUP_GRACE => false,
        Some(_) => since_last_data >= SILENCE_TIMEOUT,
    }
}

/// What the capture thread is asked to do. Sent rather than polled, so the
/// thread sleeps on `recv` instead of waking to check a flag.
enum Command {
    Pause,
    Resume,
    Stop,
}

/// Where capture is, for the operator UI (FR-06).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureState {
    Stopped,
    Running,
    Paused,
}

/// A running capture. Dropping it stops the device.
///
/// `cpal::Stream` is deliberately not `Send` on every platform, so this cannot
/// be parked in shared state as-is; the lifecycle work in FR-06 gives it a
/// thread of its own to live on. Until then a caller holds it directly.
pub struct CaptureStream {
    stream: cpal::Stream,
    shared: Arc<Mutex<Shared>>,
    /// Milliseconds since `epoch` at the last data callback, for the watchdog.
    last_data_ms: Arc<AtomicU64>,
    epoch: Instant,
    /// Chunks handed to the sink, real and synthesised.
    chunks_out: Arc<AtomicU64>,
    /// When the timeline started or was restarted, in ms since `epoch`, and
    /// the chunk count at that moment. Reset on the first chunk and on resume,
    /// so a deliberate pause is never backfilled with silence.
    ///
    /// `u64::MAX` means "not started": no audio has arrived yet, and there is
    /// no timeline to keep aligned.
    timeline_ms: Arc<AtomicU64>,
    chunks_at_timeline: Arc<AtomicU64>,
    /// Chunks of silence synthesised because a loopback device delivered
    /// nothing. Reported so a run can say how much of its audio was invented.
    silence_chunks: Arc<AtomicU64>,
    /// What the device actually gave us, which is rarely what was asked for.
    pub source_rate: u32,
    pub source_channels: u16,
}

impl CaptureStream {
    pub fn play(&self) -> Result<()> {
        self.stream
            .play()
            .map_err(|e| Error::Audio(format!("could not start capture: {e}")))
    }

    pub fn pause(&self) -> Result<()> {
        self.stream
            .pause()
            .map_err(|e| Error::Audio(format!("could not pause capture: {e}")))
    }

    /// Start or restart the wall-clock timeline that silence is measured
    /// against. Called on the first chunk and on resume.
    fn reset_timeline(&self) {
        self.timeline_ms
            .store(self.epoch.elapsed().as_millis() as u64, Ordering::Relaxed);
        self.chunks_at_timeline
            .store(self.chunks_out.load(Ordering::Relaxed), Ordering::Relaxed);
    }

    /// Emit silence for audio a loopback device never delivered, so the
    /// timeline keeps pace with the clock. Returns how many chunks were added.
    ///
    /// **Windows loopback delivers nothing while nothing is playing.** Not
    /// silence — nothing at all. So every quiet moment in a service, a prayer,
    /// a pause between songs, simply vanishes from the audio timeline, and
    /// every timestamp after it is early by the length of the quiet.
    ///
    /// That is not only a measurement problem. Paragraph breaks are decided by
    /// a 2.5-second gap between utterances (`stt::transcript`), and a gap that
    /// never reaches the timeline can never open a paragraph — so the one
    /// feature that makes a transcript readable would fail exactly where the
    /// preacher paused for effect. M4's summary timestamps would drift the same
    /// way, a little further every time the room went quiet.
    ///
    /// A microphone has never had this problem: it delivers buffers of near
    /// silence continuously, and everything downstream is built for that. This
    /// makes loopback behave the same way rather than making everything
    /// downstream cope with two kinds of timeline.
    ///
    /// The silence is real audio and goes wherever real audio goes, Deepgram
    /// included. That is deliberate: our chunk counter and Deepgram's word
    /// timestamps have to count the same seconds, and they stopped doing so
    /// once before — see `stt::transcript`. It costs nothing extra against a
    /// microphone, which streams its own silence already.
    fn top_up_silence(&self) -> usize {
        let timeline = self.timeline_ms.load(Ordering::Relaxed);
        if timeline == u64::MAX {
            return 0;
        }

        let elapsed = (self.epoch.elapsed().as_millis() as u64).saturating_sub(timeline);
        let produced = self
            .chunks_out
            .load(Ordering::Relaxed)
            .saturating_sub(self.chunks_at_timeline.load(Ordering::Relaxed));

        let missing = silence_chunks_due(elapsed, produced);
        if missing == 0 {
            return 0;
        }

        let Ok(mut state) = self.shared.lock() else {
            return 0;
        };
        for _ in 0..missing {
            (state.sink)(vec![0i16; CHUNK_SAMPLES]);
        }
        drop(state);

        self.chunks_out.fetch_add(missing, Ordering::Relaxed);
        self.silence_chunks.fetch_add(missing, Ordering::Relaxed);
        missing as usize
    }

    /// Seconds of silence synthesised so far.
    pub fn synthesised_silence(&self) -> f64 {
        self.silence_chunks.load(Ordering::Relaxed) as f64 * CHUNK_MS as f64 / 1000.0
    }

    /// How long since the device last delivered audio.
    pub fn since_last_data(&self) -> Duration {
        let last = self.last_data_ms.load(Ordering::Relaxed);
        self.epoch
            .elapsed()
            .saturating_sub(Duration::from_millis(last))
    }

    /// Deliver whatever has not filled a chunk.
    ///
    /// Called once, after the stream is paused, when capture ends. Without it
    /// up to 250 ms of the final audio is dropped — harmless mid-service,
    /// wrong at End Service, where it is the last words of the sermon.
    fn flush_tail(&self) {
        let mut shared = match self.shared.lock() {
            Ok(shared) => shared,
            // A poisoned lock means the audio thread panicked mid-buffer.
            // The tail is not worth propagating that into the stop path.
            Err(err) => {
                tracing::error!("capture state was poisoned; dropping the final chunk");
                err.into_inner()
            }
        };

        let tail = shared.converter.flush();
        if !tail.is_empty() {
            (shared.sink)(tail);
        }
    }
}

/// Open a device and start converting it into chunks.
///
/// The stream is returned paused; call [`play`](CaptureStream::play).
///
/// The device's own default config is used rather than requesting 16 kHz mono
/// directly. Asking for a format the hardware does not have fails outright on
/// WASAPI in shared mode, and a church's interface is entitled to be 48 kHz
/// 4-channel. Taking what it offers and converting always works.
pub fn open(
    device: &cpal::Device,
    is_output_endpoint: bool,
    sink: ChunkSink,
    mut on_level: LevelSink,
    on_error: ErrorSink,
    // Shared with the caller so a running service can be asked how much of its
    // audio was invented, without reaching into the capture thread.
    silence_chunks: Arc<AtomicU64>,
) -> Result<CaptureStream> {
    let supported = if is_output_endpoint {
        device.default_output_config()
    } else {
        device.default_input_config()
    }
    .map_err(|e| {
        Error::Audio(format!(
            "could not read the device's audio format: {e}. Choose another input in Settings."
        ))
    })?;

    let sample_format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();
    let source_rate = config.sample_rate.0;
    let source_channels = config.channels;

    let shared = Arc::new(Mutex::new(Shared {
        converter: CaptureConverter::new(source_rate, source_channels)?,
        sink,
    }));

    let mut meter = LevelMeter::new();
    let callback_shared = Arc::clone(&shared);

    // The watchdog's heartbeat. An atomic store is a handful of nanoseconds,
    // which is what the audio thread can afford; anything needing a lock or an
    // allocation would not be.
    let epoch = Instant::now();
    let last_data_ms = Arc::new(AtomicU64::new(0));
    let heartbeat = Arc::clone(&last_data_ms);

    let chunks_out = Arc::new(AtomicU64::new(0));
    // u64::MAX is "no audio yet": there is no timeline to keep aligned until
    // the device has spoken once, and starting the clock at `open` would
    // synthesise silence to cover the device warming up.
    let timeline_ms = Arc::new(AtomicU64::new(u64::MAX));
    let chunks_at_timeline = Arc::new(AtomicU64::new(0));

    let callback_chunks = Arc::clone(&chunks_out);
    let callback_timeline = Arc::clone(&timeline_ms);

    let mut deliver = move |samples: &[f32]| {
        heartbeat.store(epoch.elapsed().as_millis() as u64, Ordering::Relaxed);

        // Metered before conversion, so the level reflects what the device
        // sent rather than what survived downmixing and resampling. Done
        // outside the lock, so a meter frame never waits on anything.
        if let Some(level) = meter.push(samples) {
            on_level(level);
        }

        let Ok(mut state) = callback_shared.lock() else {
            // Poisoned: an earlier callback panicked. Drop buffers and keep the
            // stream alive rather than ending the service.
            return;
        };

        // Conversion failure is reported and the stream left running:
        // resampling is stateful, and a single bad buffer is better lost than
        // treated as the end of the service.
        let Shared { converter, sink } = &mut *state;
        match converter.push(samples) {
            Ok(chunks) => {
                let n = chunks.len() as u64;
                for chunk in chunks {
                    sink(chunk);
                }
                if n > 0 {
                    let before = callback_chunks.fetch_add(n, Ordering::Relaxed);
                    // The timeline starts at the first chunk the device
                    // actually produced, not at `open`.
                    if before == 0 {
                        callback_timeline
                            .store(epoch.elapsed().as_millis() as u64, Ordering::Relaxed);
                    }
                }
            }
            Err(err) => tracing::error!(%err, "dropping an audio buffer"),
        }
    };

    let error_callback = move |err: cpal::StreamError| {
        let message = match err {
            cpal::StreamError::DeviceNotAvailable => {
                "The audio device was disconnected. Choose another input in Settings.".to_string()
            }
            other => format!("{other}. Choose another input in Settings."),
        };
        on_error(Error::Audio(message));
    };

    let stream = match sample_format {
        cpal::SampleFormat::I8 => build::<i8>(device, &config, deliver, error_callback),
        cpal::SampleFormat::I16 => build::<i16>(device, &config, deliver, error_callback),
        cpal::SampleFormat::I32 => build::<i32>(device, &config, deliver, error_callback),
        cpal::SampleFormat::U8 => build::<u8>(device, &config, deliver, error_callback),
        cpal::SampleFormat::U16 => build::<u16>(device, &config, deliver, error_callback),
        cpal::SampleFormat::U32 => build::<u32>(device, &config, deliver, error_callback),
        cpal::SampleFormat::F32 => build::<f32>(device, &config, deliver, error_callback),
        cpal::SampleFormat::F64 => build::<f64>(device, &config, deliver, error_callback),
        // Named rather than swallowed: an unsupported format is a device we
        // have never seen, and the operator needs to know to pick another.
        other => {
            let _ = &mut deliver;
            return Err(Error::Audio(format!(
                "This device uses an audio format SermonAI cannot read ({other}). \
                 Choose another input in Settings."
            )));
        }
    }?;

    Ok(CaptureStream {
        stream,
        shared,
        last_data_ms,
        epoch,
        chunks_out,
        timeline_ms,
        chunks_at_timeline,
        silence_chunks,
        source_rate,
        source_channels,
    })
}

/// A capture running on its own thread.
///
/// The thread exists because `cpal::Stream` is not `Send` on every platform:
/// it cannot be created in a command and parked in shared state, so it is
/// created, played and dropped entirely on one thread that outlives neither.
/// Dropping the handle stops capture and waits for that thread to finish, so
/// the device is released before the next `start` opens it again.
pub struct CaptureHandle {
    commands: mpsc::Sender<Command>,
    /// Mirrors what the thread is doing, so the UI can be told without asking
    /// it. `AtomicU8` because `CaptureState` is three values and a lock here
    /// would be read far more often than written.
    state: Arc<AtomicU8>,
    /// Chunks of silence the capture thread had to invent because a loopback
    /// device delivered nothing. Shared so a running service can be asked.
    silence_chunks: Arc<AtomicU64>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl CaptureHandle {
    /// Seconds of silence synthesised so far because the device delivered
    /// nothing (see `CaptureStream::top_up_silence`).
    ///
    /// Worth reporting rather than hiding: it is audio SermonAI invented, and
    /// an operator diagnosing a transcript with gaps in it should be able to
    /// tell "the room was quiet" from "the capture path was quiet".
    pub fn synthesised_silence(&self) -> f64 {
        self.silence_chunks.load(Ordering::Relaxed) as f64 * CHUNK_MS as f64 / 1000.0
    }

    /// Stop delivering audio without releasing the device (FR-06).
    ///
    /// The device stays open deliberately. Closing and reopening risks the OS
    /// handing the input to something else in the gap, and some interfaces
    /// allow only one capture client — a pause that loses the microphone is
    /// not a pause.
    pub fn pause(&self) -> Result<()> {
        self.send(Command::Pause, CaptureState::Paused)
    }

    pub fn resume(&self) -> Result<()> {
        self.send(Command::Resume, CaptureState::Running)
    }

    pub fn state(&self) -> CaptureState {
        match self.state.load(Ordering::Relaxed) {
            1 => CaptureState::Running,
            2 => CaptureState::Paused,
            _ => CaptureState::Stopped,
        }
    }

    fn send(&self, command: Command, next: CaptureState) -> Result<()> {
        self.commands.send(command).map_err(|_| {
            Error::Audio("Capture has already stopped. Start it again in Settings.".to_string())
        })?;
        self.state.store(next as u8, Ordering::Relaxed);
        Ok(())
    }
}

impl Drop for CaptureHandle {
    fn drop(&mut self) {
        // Ignored: a disconnected receiver means the thread has already gone,
        // which is the state this is trying to reach.
        let _ = self.commands.send(Command::Stop);
        self.state
            .store(CaptureState::Stopped as u8, Ordering::Relaxed);

        if let Some(thread) = self.thread.take() {
            // A panicked capture thread is logged, not propagated: unwinding
            // out of a Drop during teardown would abort the process, and
            // losing the meter is not worth ending a service over.
            if thread.join().is_err() {
                tracing::error!("the capture thread panicked");
            }
        }
    }
}

/// Start capturing from a device by name, on a thread of its own.
///
/// Returns once the device is open and running, so a failure to open surfaces
/// as an error the operator sees immediately rather than as a meter that never
/// moves.
pub fn spawn(
    device_name: String,
    sink: ChunkSink,
    on_level: LevelSink,
    on_error: ErrorSink,
) -> Result<CaptureHandle> {
    let state = Arc::new(AtomicU8::new(CaptureState::Stopped as u8));
    let silence_chunks = Arc::new(AtomicU64::new(0));
    let thread_silence = Arc::clone(&silence_chunks);
    let (command_tx, command_rx) = mpsc::channel::<Command>();
    let (ready_tx, ready_rx) = mpsc::channel::<Result<()>>();

    let thread = std::thread::Builder::new()
        .name("sermonai-capture".to_string())
        .spawn(move || {
            // Shared, because two things report a lost device: cpal's own
            // error callback for a device that raises one, and the watchdog
            // for a device that simply goes quiet. Bluetooth does the second.
            let report: Arc<ErrorSink> = Arc::new(on_error);
            let stream_report = Arc::clone(&report);

            let mut watch_for_silence = false;
            let opened = devices::find_device(&device_name).and_then(|(device, is_output)| {
                watch_for_silence = !is_output;
                open(
                    &device,
                    is_output,
                    sink,
                    on_level,
                    Box::new(move |err| stream_report(err)),
                    thread_silence,
                )
            });

            let stream = match opened.and_then(|stream| stream.play().map(|()| stream)) {
                Ok(stream) => stream,
                Err(err) => {
                    let _ = ready_tx.send(Err(err));
                    return;
                }
            };

            let _ = ready_tx.send(Ok(()));

            // Woken by a command, or by the watchdog tick. A disconnected
            // sender means the handle was dropped, which is a stop.
            let mut running_since = Some(Instant::now());

            loop {
                match command_rx.recv_timeout(WATCHDOG_TICK) {
                    Ok(command) => {
                        let outcome = match command {
                            Command::Pause => {
                                // A paused device is silent on purpose, so the
                                // watchdog has to stand down or it would report
                                // every pause as a lost device.
                                running_since = None;
                                stream.pause()
                            }
                            Command::Resume => {
                                running_since = Some(Instant::now());
                                // Before playing, so the pause is not
                                // backfilled with silence on the next tick. A
                                // pause is deliberate: those seconds did not
                                // happen as far as the service is concerned.
                                stream.reset_timeline();
                                stream.play()
                            }
                            Command::Stop => break,
                        };

                        if let Err(err) = outcome {
                            tracing::error!(%err, "the device refused a transport command");
                        }
                    }

                    Err(mpsc::RecvTimeoutError::Timeout) => {
                        // Loopback only, and only while running. An input
                        // device delivers continuously, so a gap there is a
                        // real fault and the watchdog below must see it —
                        // filling it with silence would hide a lost
                        // microphone for the rest of the service.
                        if !watch_for_silence && running_since.is_some() {
                            let added = stream.top_up_silence();
                            if added > 0 {
                                tracing::debug!(
                                    chunks = added,
                                    "loopback delivered nothing; synthesising silence to keep the timeline on the clock"
                                );
                            }
                        }

                        if !device_lost(
                            running_since.map(|since| since.elapsed()),
                            stream.since_last_data(),
                            !watch_for_silence,
                        ) {
                            continue;
                        }

                        // Reported once, then the watchdog stands down: the
                        // device is not coming back on its own, and an error a
                        // second for the rest of the service would bury it.
                        running_since = None;
                        tracing::warn!(
                            device = %device_name,
                            "no audio for {}s; treating the device as lost",
                            SILENCE_TIMEOUT.as_secs()
                        );
                        report(Error::Audio(format!(
                            "\"{device_name}\" stopped sending audio and appears to be disconnected.                              Choose another input to carry on."
                        )));
                    }

                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }

            // Paused before flushing, so no callback can be part-way through a
            // buffer while the tail is taken. After this cpal makes no further
            // calls, which is what makes the shared lock uncontended.
            if let Err(err) = stream.pause() {
                tracing::warn!(%err, "could not pause the device before stopping");
            }
            stream.flush_tail();

            // Dropped here, on the thread that created it.
            drop(stream);
        })
        .map_err(|e| Error::Audio(format!("could not start the capture thread: {e}")))?;

    match ready_rx.recv() {
        Ok(Ok(())) => {
            state.store(CaptureState::Running as u8, Ordering::Relaxed);
            Ok(CaptureHandle {
                commands: command_tx,
                state,
                silence_chunks,
                thread: Some(thread),
            })
        }
        Ok(Err(err)) => Err(err),
        // The thread ended without reporting, which means it panicked before
        // it could. Naming it beats a channel error the operator cannot read.
        Err(_) => Err(Error::Audio(
            "Capture stopped unexpectedly. Choose another input in Settings.".to_string(),
        )),
    }
}

fn build<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut deliver: impl FnMut(&[f32]) + Send + 'static,
    on_error: impl Fn(cpal::StreamError) + Send + 'static,
) -> Result<cpal::Stream>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    // Reused across callbacks so the audio thread does not allocate per buffer.
    let mut scratch: Vec<f32> = Vec::new();

    device
        .build_input_stream(
            config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                scratch.clear();
                scratch.extend(data.iter().map(|s| f32::from_sample_(*s)));
                deliver(&scratch);
            },
            on_error,
            None,
        )
        .map_err(|e| {
            Error::Audio(format!(
                "could not open the audio device: {e}. Choose another input in Settings."
            ))
        })
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_device_keeping_up_is_left_alone() {
        // The failure that would matter most: injecting silence into the
        // middle of continuous speech. A device delivering normally is always
        // a fraction of a chunk behind the clock, so this must stay at zero
        // across the whole range of normal jitter.
        for produced in 0..200u64 {
            let exactly = produced * CHUNK_MS;
            assert_eq!(silence_chunks_due(exactly, produced), 0);
            // A little ahead of the clock, and a little behind.
            assert_eq!(silence_chunks_due(exactly + CHUNK_MS - 1, produced), 0);
            assert_eq!(silence_chunks_due(exactly.saturating_sub(10), produced), 0);
        }
    }

    #[test]
    fn a_silent_loopback_is_topped_up_to_the_clock() {
        // Windows loopback delivers nothing — not silence, nothing — while
        // nothing is playing. Thirty seconds of quiet must come back as 30
        // seconds of timeline, or the pause never reaches the transcript and
        // no paragraph can break on it.
        let thirty_seconds = 30_000;
        let owed = silence_chunks_due(thirty_seconds, 0);
        // 120 chunks in 30 s, less the one chunk of slack.
        assert_eq!(owed, 119);
        assert!((owed as f64 * CHUNK_MS as f64 / 1000.0 - 29.75).abs() < 1e-9);
    }

    #[test]
    fn the_top_up_paces_itself_rather_than_bursting() {
        // The watchdog ticks every 250 ms, which is exactly one chunk, so a
        // device that goes quiet is topped up one chunk at a time. A burst
        // would send a block of audio to Deepgram faster than real time —
        // the same effect that makes reconnect catch-up unusable as a
        // measurement.
        assert_eq!(WATCHDOG_TICK.as_millis() as u64, CHUNK_MS);

        // The clock advances one tick per iteration regardless of what the
        // device does; only `produced` responds. Deriving elapsed from
        // `produced` instead — as the first version of this test did — models
        // a clock that stops whenever the device does, which is the opposite
        // of the situation being tested.
        const BASE: u64 = 40;
        let mut produced = BASE;
        for tick in 1..=20u64 {
            let elapsed = (BASE + tick) * CHUNK_MS;
            let owed = silence_chunks_due(elapsed, produced);
            assert!(owed <= 1, "tick {tick} wanted {owed} chunks at once");
            produced += owed;
        }

        // And it did keep pace: the timeline is level with the clock, not
        // lagging a chunk further behind on every tick.
        assert_eq!(produced, BASE + 19);
    }

    #[test]
    fn a_long_gap_is_filled_completely_rather_than_partly() {
        // If the watchdog missed its ticks — a busy machine, a suspended
        // laptop — the whole gap is still owed. Filling only part of it would
        // leave the timeline permanently short, which is the bug being fixed.
        let mut produced = 0u64;
        let elapsed = 60_000;
        produced += silence_chunks_due(elapsed, produced);
        assert_eq!(produced, 239);
        // And once filled, nothing further is owed.
        assert_eq!(silence_chunks_due(elapsed, produced), 0);
    }

    use super::*;
    use crate::audio::convert::CHUNK_SAMPLES;
    use std::sync::mpsc;

    #[test]
    fn a_paused_device_is_never_reported_lost() {
        // The case that matters most. A paused device is silent on purpose,
        // and reporting every pause as a disconnection would teach the
        // operator to ignore the one warning that matters.
        assert!(!device_lost(None, Duration::from_secs(60), false));
    }

    #[test]
    fn a_slow_device_gets_time_to_speak_once() {
        // Bluetooth can take a moment to deliver its first buffer. Declaring
        // it lost before it ever has would make it unusable.
        assert!(!device_lost(
            Some(Duration::from_millis(500)),
            Duration::from_secs(10),
            false
        ));
        assert!(!device_lost(
            Some(STARTUP_GRACE - Duration::from_millis(1)),
            Duration::from_secs(10),
            false
        ));
    }

    #[test]
    fn a_running_device_that_stops_sending_is_reported() {
        // The Bluetooth case from testing: WASAPI kept the endpoint valid and
        // simply stopped delivering, so no stream error was ever raised and
        // the app carried on believing it was recording.
        assert!(device_lost(
            Some(Duration::from_secs(30)),
            SILENCE_TIMEOUT,
            false
        ));
        assert!(device_lost(
            Some(Duration::from_secs(30)),
            Duration::from_secs(10),
            false
        ));
    }

    #[test]
    fn a_brief_gap_is_not_a_disconnection() {
        // A busy machine can miss a buffer or two without the device having
        // gone anywhere, and a false alarm mid-sermon is its own failure.
        assert!(!device_lost(
            Some(Duration::from_secs(30)),
            SILENCE_TIMEOUT - Duration::from_millis(1),
            false
        ));
        assert!(!device_lost(
            Some(Duration::from_secs(30)),
            Duration::from_millis(250),
            false
        ));
    }

    #[test]
    fn an_idle_loopback_is_not_a_lost_device() {
        // Measured, not assumed: two of this machine's three loopback
        // endpoints deliver nothing while the system plays nothing, so the
        // watchdog reported a healthy sound-desk feed as failed within two
        // seconds of the music stopping.
        assert!(!device_lost(
            Some(Duration::from_secs(30)),
            Duration::from_secs(600),
            true
        ));
    }

    /// Opening a device that is not there must name it and offer a way out,
    /// rather than panicking on the audio thread where nothing can catch it.
    #[test]
    fn opening_a_missing_device_fails_with_an_operator_facing_message() {
        let err = crate::audio::devices::find_device("No Such Interface")
            .err()
            .expect("a device that does not exist should not resolve");
        assert!(err.to_string().contains("Settings"), "{err}");
    }

    /// The end-to-end shape of a capture, without a sound card: drive the
    /// converter the way the stream callback does and confirm what comes out
    /// the far side is what the transcriber expects.
    #[test]
    fn a_capture_produces_250_ms_chunks_of_16_khz_mono() {
        let (tx, rx) = mpsc::channel();
        let mut sink: ChunkSink = Box::new(move |chunk| {
            tx.send(chunk).expect("receiver should outlive the sink");
        });

        // A 48 kHz stereo device, delivered in the uneven buffers cpal gives.
        let mut converter = CaptureConverter::new(48_000, 2).unwrap();
        let frames = 48_000; // one second
        let interleaved: Vec<f32> = (0..frames * 2)
            .map(|i| ((i / 2) as f32 * 0.001).sin())
            .collect();

        for buffer in interleaved.chunks(1022) {
            for chunk in converter.push(buffer).unwrap() {
                sink(chunk);
            }
        }
        drop(sink);

        let chunks: Vec<Vec<i16>> = rx.into_iter().collect();
        assert!(
            !chunks.is_empty(),
            "a second of audio should produce chunks"
        );
        for chunk in &chunks {
            assert_eq!(chunk.len(), CHUNK_SAMPLES, "every chunk is exactly 250 ms");
        }
        // One second in, four chunks out, give or take the resampler's latency.
        assert!(
            (3..=4).contains(&chunks.len()),
            "expected about four chunks, got {}",
            chunks.len()
        );
    }
}
