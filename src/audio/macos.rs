//! Audio recording for macOS using CPAL's CoreAudio backend.

use super::conversion::{downmix_frame, MonoPcm16Resampler};
use crate::{AppEvent, RecordCommand};
use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample, Stream, StreamConfig};
use crossbeam_channel::{Receiver, Sender};
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use tracing::{debug, error, info};

const OUTPUT_SAMPLE_RATE: u32 = 16_000;
const OUTPUT_CHANNELS: u16 = 1;

type WavWriter = hound::WavWriter<BufWriter<File>>;
type SharedWriter = Arc<Mutex<Option<WavWriter>>>;
type CaptureFailure = Arc<Mutex<Option<String>>>;
type SessionRunner = Arc<dyn Fn(Arc<AtomicBool>) -> Result<PathBuf> + Send + Sync>;

struct ActiveSession {
    recording: Arc<AtomicBool>,
    cancelled: Arc<AtomicBool>,
    handle: JoinHandle<()>,
}

/// Listen for start, stop and cancel commands from the application event loop.
pub fn audio_worker(
    record_rx: Receiver<RecordCommand>,
    audio_tx: Sender<Option<PathBuf>>,
    status_tx: Sender<AppEvent>,
) {
    let capture_status_tx = status_tx.clone();
    audio_worker_with_runner(
        record_rx,
        audio_tx,
        status_tx,
        Arc::new(move |recording| run_cpal_session(&recording, &capture_status_tx)),
    );
}

fn audio_worker_with_runner(
    record_rx: Receiver<RecordCommand>,
    audio_tx: Sender<Option<PathBuf>>,
    status_tx: Sender<AppEvent>,
    session_runner: SessionRunner,
) {
    info!("Audio worker thread started (macOS/CoreAudio backend)");
    let mut active_session: Option<ActiveSession> = None;

    for command in record_rx {
        reap_finished_session(&mut active_session, &audio_tx, &status_tx);

        match command {
            RecordCommand::Start => {
                if active_session.is_some() {
                    info!("Already recording, ignoring start signal");
                    continue;
                }

                let recording = Arc::new(AtomicBool::new(true));
                let cancelled = Arc::new(AtomicBool::new(false));
                let recording_clone = Arc::clone(&recording);
                let cancelled_clone = Arc::clone(&cancelled);
                let audio_tx_clone = audio_tx.clone();
                let status_tx_clone = status_tx.clone();
                let session_runner = Arc::clone(&session_runner);

                let handle = std::thread::spawn(move || {
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        session_runner(Arc::clone(&recording_clone))
                    }));
                    recording_clone.store(false, Ordering::SeqCst);

                    match result {
                        Ok(Ok(path)) if cancelled_clone.load(Ordering::SeqCst) => {
                            info!("Recording was cancelled, deleting file");
                            let _ = std::fs::remove_file(&path);
                            let _ = audio_tx_clone.send(None);
                        }
                        Ok(Ok(path)) => {
                            info!("Recording saved to {:?}", path);
                            let _ = audio_tx_clone.send(Some(path));
                        }
                        Ok(Err(capture_error)) => {
                            if !cancelled_clone.load(Ordering::SeqCst) {
                                error!("CoreAudio recording session failed: {}", capture_error);
                                let _ = status_tx_clone.send(AppEvent::Error(format!(
                                    "Audio recording failed: {}",
                                    capture_error
                                )));
                            }
                            let _ = audio_tx_clone.send(None);
                        }
                        Err(_) => {
                            error!("macOS audio recording worker panicked");
                            let _ = status_tx_clone.send(AppEvent::Error(
                                "Audio recording failed unexpectedly".to_string(),
                            ));
                            let _ = audio_tx_clone.send(None);
                        }
                    }
                });

                active_session = Some(ActiveSession {
                    recording,
                    cancelled,
                    handle,
                });
            }
            RecordCommand::Stop => {
                info!("Stop recording signal received");
                finish_active_session(&mut active_session, false, &audio_tx, &status_tx);
            }
            RecordCommand::Cancel => {
                info!("Cancel recording signal received");
                finish_active_session(&mut active_session, true, &audio_tx, &status_tx);
            }
        }
    }

    // Channel closure means the application is shutting down. Cancel and join
    // any in-flight recording so a private temporary WAV cannot survive exit.
    finish_active_session(&mut active_session, true, &audio_tx, &status_tx);
}

fn reap_finished_session(
    active_session: &mut Option<ActiveSession>,
    audio_tx: &Sender<Option<PathBuf>>,
    status_tx: &Sender<AppEvent>,
) {
    let is_finished = active_session
        .as_ref()
        .map(|session| !session.recording.load(Ordering::SeqCst) || session.handle.is_finished())
        .unwrap_or(false);
    if is_finished {
        finish_active_session(active_session, false, audio_tx, status_tx);
    }
}

fn finish_active_session(
    active_session: &mut Option<ActiveSession>,
    cancel: bool,
    audio_tx: &Sender<Option<PathBuf>>,
    status_tx: &Sender<AppEvent>,
) {
    let Some(session) = active_session.take() else {
        return;
    };

    if cancel {
        session.cancelled.store(true, Ordering::SeqCst);
    }
    session.recording.store(false, Ordering::SeqCst);

    if session.handle.join().is_err() {
        error!("macOS audio recording thread terminated unexpectedly");
        let _ = status_tx.send(AppEvent::Error(
            "Audio recording failed unexpectedly".to_string(),
        ));
        let _ = audio_tx.send(None);
    }
}

/// Record the system default input in its native format, then convert it to the
/// mono 16 kHz PCM WAV contract consumed by the transcription engine.
fn run_cpal_session(recording: &Arc<AtomicBool>, status_tx: &Sender<AppEvent>) -> Result<PathBuf> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .context("No macOS input device is available")?;
    let supported = device
        .default_input_config()
        .context("Failed to read the default macOS input format")?;

    let sample_format = supported.sample_format();
    let config: StreamConfig = supported.into();
    let input_channels = usize::from(config.channels);
    let input_sample_rate = config.sample_rate.0;
    if input_channels == 0 || input_sample_rate == 0 {
        anyhow::bail!("The default macOS input format is invalid");
    }
    let device_name = device
        .name()
        .unwrap_or_else(|_| "System default microphone".to_string());
    info!(
        "Using macOS input device '{}' at {} Hz, {} channel(s), {}",
        device_name, input_sample_rate, input_channels, sample_format
    );

    let audio_temp = super::private_recording_temp_path()?;
    let audio_path = audio_temp.to_path_buf();
    let spec = hound::WavSpec {
        channels: OUTPUT_CHANNELS,
        sample_rate: OUTPUT_SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let writer = Arc::new(Mutex::new(Some(
        hound::WavWriter::create(&audio_path, spec).context("Failed to create WAV file")?,
    )));
    let capture_failure = Arc::new(Mutex::new(None));

    let stream = match sample_format {
        SampleFormat::I8 => build_input_stream::<i8>(
            &device,
            &config,
            Arc::clone(&writer),
            Arc::clone(&capture_failure),
        ),
        SampleFormat::I16 => build_input_stream::<i16>(
            &device,
            &config,
            Arc::clone(&writer),
            Arc::clone(&capture_failure),
        ),
        SampleFormat::I32 => build_input_stream::<i32>(
            &device,
            &config,
            Arc::clone(&writer),
            Arc::clone(&capture_failure),
        ),
        SampleFormat::I64 => build_input_stream::<i64>(
            &device,
            &config,
            Arc::clone(&writer),
            Arc::clone(&capture_failure),
        ),
        SampleFormat::U8 => build_input_stream::<u8>(
            &device,
            &config,
            Arc::clone(&writer),
            Arc::clone(&capture_failure),
        ),
        SampleFormat::U16 => build_input_stream::<u16>(
            &device,
            &config,
            Arc::clone(&writer),
            Arc::clone(&capture_failure),
        ),
        SampleFormat::U32 => build_input_stream::<u32>(
            &device,
            &config,
            Arc::clone(&writer),
            Arc::clone(&capture_failure),
        ),
        SampleFormat::U64 => build_input_stream::<u64>(
            &device,
            &config,
            Arc::clone(&writer),
            Arc::clone(&capture_failure),
        ),
        SampleFormat::F32 => build_input_stream::<f32>(
            &device,
            &config,
            Arc::clone(&writer),
            Arc::clone(&capture_failure),
        ),
        SampleFormat::F64 => build_input_stream::<f64>(
            &device,
            &config,
            Arc::clone(&writer),
            Arc::clone(&capture_failure),
        ),
        other => anyhow::bail!("Unsupported macOS input sample format: {}", other),
    }?;

    stream
        .play()
        .context("Failed to start the macOS input stream")?;
    let _ = status_tx.send(AppEvent::CaptureStarted { device_name });

    while recording.load(Ordering::SeqCst) && capture_failure_message(&capture_failure).is_none() {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    drop(stream);

    let failure = capture_failure_message(&capture_failure);
    let writer = writer
        .lock()
        .map_err(|_| anyhow::anyhow!("macOS audio writer lock was poisoned"))?
        .take();
    if let Some(writer) = writer {
        writer.finalize().context("Failed to finalise WAV file")?;
    }

    if let Some(failure) = failure {
        anyhow::bail!("The macOS input stream stopped unexpectedly: {}", failure);
    }

    let metadata = std::fs::metadata(&audio_path)?;
    if metadata.len() < 100 {
        anyhow::bail!("Audio file too small ({} bytes)", metadata.len());
    }

    debug!("macOS audio file ready: {} bytes", metadata.len());
    audio_temp
        .keep()
        .context("Failed to retain completed audio recording")
}

fn build_input_stream<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    writer: SharedWriter,
    capture_failure: CaptureFailure,
) -> Result<Stream>
where
    T: Sample + SizedSample,
    f32: FromSample<T>,
{
    let channels = usize::from(config.channels);
    let mut converter = MonoPcm16Resampler::new(config.sample_rate.0, OUTPUT_SAMPLE_RATE)
        .map_err(anyhow::Error::msg)?;
    let callback_failure = Arc::clone(&capture_failure);

    device
        .build_input_stream(
            config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                let Ok(mut guard) = writer.lock() else {
                    set_capture_failure(
                        &callback_failure,
                        "audio writer lock was poisoned".to_string(),
                    );
                    return;
                };
                let Some(writer) = guard.as_mut() else {
                    return;
                };

                for frame in data.chunks_exact(channels) {
                    let Some(mono) = downmix_frame(frame) else {
                        continue;
                    };
                    let mut write_error = None;
                    converter.push_sample(mono, |output| {
                        if write_error.is_none() {
                            if let Err(error) = writer.write_sample(output) {
                                write_error = Some(error.to_string());
                            }
                        }
                    });
                    if let Some(error) = write_error {
                        set_capture_failure(
                            &callback_failure,
                            format!("failed to write captured audio: {}", error),
                        );
                        return;
                    }
                }
            },
            move |stream_error| {
                error!("CoreAudio input stream error: {}", stream_error);
                set_capture_failure(&capture_failure, stream_error.to_string());
            },
            None,
        )
        .context("Failed to build the macOS input stream")
}

fn set_capture_failure(capture_failure: &CaptureFailure, message: String) {
    if let Ok(mut failure) = capture_failure.lock() {
        if failure.is_none() {
            *failure = Some(message);
        }
    }
}

fn capture_failure_message(capture_failure: &CaptureFailure) -> Option<String> {
    capture_failure
        .lock()
        .ok()
        .and_then(|failure| failure.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    use std::time::Duration;

    fn spawn_worker(
        runner: SessionRunner,
    ) -> (
        Sender<RecordCommand>,
        Receiver<Option<PathBuf>>,
        Receiver<AppEvent>,
        JoinHandle<()>,
    ) {
        let (record_tx, record_rx) = crossbeam_channel::unbounded();
        let (audio_tx, audio_rx) = crossbeam_channel::unbounded();
        let (status_tx, status_rx) = crossbeam_channel::unbounded();
        let handle = std::thread::spawn(move || {
            audio_worker_with_runner(record_rx, audio_tx, status_tx, runner)
        });
        (record_tx, audio_rx, status_rx, handle)
    }

    fn wait_until(predicate: impl Fn() -> bool) {
        for _ in 0..100 {
            if predicate() {
                return;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        panic!("timed out waiting for test condition");
    }

    fn temporary_output() -> PathBuf {
        let file = tempfile::NamedTempFile::new().unwrap();
        let (_file, path) = file.keep().unwrap();
        path
    }

    #[test]
    fn output_format_matches_whisper_input_contract() {
        assert_eq!(OUTPUT_SAMPLE_RATE, 16_000);
        assert_eq!(OUTPUT_CHANNELS, 1);
    }

    #[test]
    fn stop_joins_before_an_immediate_restart() {
        let running = Arc::new(AtomicUsize::new(0));
        let max_running = Arc::new(AtomicUsize::new(0));
        let starts = Arc::new(AtomicUsize::new(0));
        let runner: SessionRunner = {
            let running = Arc::clone(&running);
            let max_running = Arc::clone(&max_running);
            let starts = Arc::clone(&starts);
            Arc::new(move |recording| {
                starts.fetch_add(1, AtomicOrdering::SeqCst);
                let now_running = running.fetch_add(1, AtomicOrdering::SeqCst) + 1;
                max_running.fetch_max(now_running, AtomicOrdering::SeqCst);
                while recording.load(Ordering::SeqCst) {
                    std::thread::sleep(Duration::from_millis(2));
                }
                std::thread::sleep(Duration::from_millis(20));
                running.fetch_sub(1, AtomicOrdering::SeqCst);
                Ok(temporary_output())
            })
        };
        let (record_tx, audio_rx, _status_rx, worker) = spawn_worker(runner);

        record_tx.send(RecordCommand::Start).unwrap();
        wait_until(|| starts.load(AtomicOrdering::SeqCst) == 1);
        record_tx.send(RecordCommand::Stop).unwrap();
        record_tx.send(RecordCommand::Start).unwrap();
        wait_until(|| starts.load(AtomicOrdering::SeqCst) == 2);
        record_tx.send(RecordCommand::Stop).unwrap();
        drop(record_tx);
        worker.join().unwrap();

        assert_eq!(max_running.load(AtomicOrdering::SeqCst), 1);
        for path in audio_rx.try_iter().flatten() {
            let _ = std::fs::remove_file(path);
        }
    }

    #[test]
    fn repeated_start_does_not_replace_the_active_session() {
        let starts = Arc::new(AtomicUsize::new(0));
        let runner: SessionRunner = {
            let starts = Arc::clone(&starts);
            Arc::new(move |recording| {
                starts.fetch_add(1, AtomicOrdering::SeqCst);
                while recording.load(Ordering::SeqCst) {
                    std::thread::sleep(Duration::from_millis(2));
                }
                Ok(temporary_output())
            })
        };
        let (record_tx, audio_rx, _status_rx, worker) = spawn_worker(runner);

        record_tx.send(RecordCommand::Start).unwrap();
        wait_until(|| starts.load(AtomicOrdering::SeqCst) == 1);
        record_tx.send(RecordCommand::Start).unwrap();
        std::thread::sleep(Duration::from_millis(20));
        record_tx.send(RecordCommand::Stop).unwrap();
        drop(record_tx);
        worker.join().unwrap();

        assert_eq!(starts.load(AtomicOrdering::SeqCst), 1);
        for path in audio_rx.try_iter().flatten() {
            let _ = std::fs::remove_file(path);
        }
    }

    #[test]
    fn old_session_completion_cannot_stop_the_new_session() {
        let starts = Arc::new(AtomicUsize::new(0));
        let (started_tx, started_rx) = crossbeam_channel::unbounded();
        let runner: SessionRunner = {
            let starts = Arc::clone(&starts);
            Arc::new(move |recording| {
                let call = starts.fetch_add(1, AtomicOrdering::SeqCst) + 1;
                started_tx.send((call, Arc::clone(&recording))).unwrap();
                while recording.load(Ordering::SeqCst) {
                    std::thread::sleep(Duration::from_millis(2));
                }
                Ok(temporary_output())
            })
        };
        let (record_tx, audio_rx, _status_rx, worker) = spawn_worker(runner);

        record_tx.send(RecordCommand::Start).unwrap();
        let (first_call, first_token) = started_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert_eq!(first_call, 1);

        record_tx.send(RecordCommand::Stop).unwrap();
        record_tx.send(RecordCommand::Start).unwrap();
        let (second_call, second_token) = started_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert_eq!(second_call, 2);
        assert!(!first_token.load(Ordering::SeqCst));
        assert!(second_token.load(Ordering::SeqCst));
        std::thread::sleep(Duration::from_millis(20));
        assert!(second_token.load(Ordering::SeqCst));

        record_tx.send(RecordCommand::Stop).unwrap();
        drop(record_tx);
        worker.join().unwrap();
        for path in audio_rx.try_iter().flatten() {
            let _ = std::fs::remove_file(path);
        }
    }

    #[test]
    fn cancel_deletes_the_completed_output() {
        let output = temporary_output();
        let output_for_runner = output.clone();
        let runner: SessionRunner = Arc::new(move |recording| {
            while recording.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(2));
            }
            Ok(output_for_runner.clone())
        });
        let (record_tx, audio_rx, _status_rx, worker) = spawn_worker(runner);

        record_tx.send(RecordCommand::Start).unwrap();
        record_tx.send(RecordCommand::Cancel).unwrap();
        drop(record_tx);
        worker.join().unwrap();

        assert_eq!(audio_rx.recv_timeout(Duration::from_secs(1)).unwrap(), None);
        assert!(!output.exists());
    }

    #[test]
    fn failed_session_is_reaped_and_next_start_recovers() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let runner: SessionRunner = {
            let attempts = Arc::clone(&attempts);
            Arc::new(move |recording| {
                let attempt = attempts.fetch_add(1, AtomicOrdering::SeqCst);
                if attempt == 0 {
                    anyhow::bail!("synthetic capture failure");
                }
                while recording.load(Ordering::SeqCst) {
                    std::thread::sleep(Duration::from_millis(2));
                }
                Ok(temporary_output())
            })
        };
        let (record_tx, audio_rx, status_rx, worker) = spawn_worker(runner);

        record_tx.send(RecordCommand::Start).unwrap();
        assert_eq!(audio_rx.recv_timeout(Duration::from_secs(1)).unwrap(), None);
        record_tx.send(RecordCommand::Start).unwrap();
        wait_until(|| attempts.load(AtomicOrdering::SeqCst) == 2);
        record_tx.send(RecordCommand::Stop).unwrap();
        drop(record_tx);
        worker.join().unwrap();

        assert!(status_rx
            .try_iter()
            .any(|event| matches!(event, AppEvent::Error(message) if message.contains("synthetic capture failure"))));
        assert_eq!(attempts.load(AtomicOrdering::SeqCst), 2);
        for path in audio_rx.try_iter().flatten() {
            let _ = std::fs::remove_file(path);
        }
    }

    #[test]
    fn panicked_session_is_contained_and_next_start_recovers() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let runner: SessionRunner = {
            let attempts = Arc::clone(&attempts);
            Arc::new(move |recording| {
                let attempt = attempts.fetch_add(1, AtomicOrdering::SeqCst);
                if attempt == 0 {
                    panic!("synthetic capture panic");
                }
                while recording.load(Ordering::SeqCst) {
                    std::thread::sleep(Duration::from_millis(2));
                }
                Ok(temporary_output())
            })
        };
        let (record_tx, audio_rx, status_rx, worker) = spawn_worker(runner);

        record_tx.send(RecordCommand::Start).unwrap();
        assert_eq!(audio_rx.recv_timeout(Duration::from_secs(1)).unwrap(), None);
        record_tx.send(RecordCommand::Start).unwrap();
        wait_until(|| attempts.load(AtomicOrdering::SeqCst) == 2);
        record_tx.send(RecordCommand::Stop).unwrap();
        drop(record_tx);
        worker.join().unwrap();

        assert!(status_rx.try_iter().any(|event| {
            matches!(event, AppEvent::Error(message) if message == "Audio recording failed unexpectedly")
        }));
        assert_eq!(attempts.load(AtomicOrdering::SeqCst), 2);
        for path in audio_rx.try_iter().flatten() {
            let _ = std::fs::remove_file(path);
        }
    }
}
