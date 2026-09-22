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

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{mpsc, Arc, Mutex};

use cpal::traits::{DeviceTrait, StreamTrait};
use cpal::{FromSample, SizedSample};

use super::convert::CaptureConverter;
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
pub type ErrorSink = Box<dyn Fn(Error) + Send + 'static>;

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

    let mut deliver = move |samples: &[f32]| {
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
                for chunk in chunks {
                    sink(chunk);
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
    thread: Option<std::thread::JoinHandle<()>>,
}

impl CaptureHandle {
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
    let (command_tx, command_rx) = mpsc::channel::<Command>();
    let (ready_tx, ready_rx) = mpsc::channel::<Result<()>>();

    let thread = std::thread::Builder::new()
        .name("sermonai-capture".to_string())
        .spawn(move || {
            let opened = devices::find_device(&device_name)
                .and_then(|(device, is_output)| open(&device, is_output, sink, on_level, on_error));

            let stream = match opened.and_then(|stream| stream.play().map(|()| stream)) {
                Ok(stream) => stream,
                Err(err) => {
                    let _ = ready_tx.send(Err(err));
                    return;
                }
            };

            let _ = ready_tx.send(Ok(()));

            // Blocks rather than polls. A disconnected sender means the handle
            // was dropped, which is a stop.
            while let Ok(command) = command_rx.recv() {
                let outcome = match command {
                    Command::Pause => stream.pause(),
                    Command::Resume => stream.play(),
                    Command::Stop => break,
                };

                if let Err(err) = outcome {
                    tracing::error!(%err, "the device refused a transport command");
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
    use super::*;
    use crate::audio::convert::CHUNK_SAMPLES;
    use std::sync::mpsc;

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
