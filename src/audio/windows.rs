//! Audio recording for Windows using CPAL's WASAPI backend.

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
    audio_worker_with_runner(
        record_rx,
        audio_tx,
        status_tx,
        Arc::new(|recording| run_wasapi_session(&recording)),
    );
}

fn audio_worker_with_runner(
    record_rx: Receiver<RecordCommand>,
    audio_tx: Sender<Option<PathBuf>>,
    status_tx: Sender<AppEvent>,
    session_runner: SessionRunner,
) {
    info!("Audio worker thread started (Windows/WASAPI backend)");
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
                        Ok(Err(error)) => {
                            if !cancelled_clone.load(Ordering::SeqCst) {
                                error!("WASAPI recording session failed: {}", error);
                                let _ = status_tx_clone.send(AppEvent::Error(format!(
                                    "Audio recording failed: {}",
                                    error
                                )));
                            }
                            let _ = audio_tx_clone.send(None);
                        }
                        Err(_) => {
                            error!("Windows audio recording worker panicked");
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

    // Channel closure means the application is shutting down. Cancel any
    // in-flight recording so a private temporary WAV cannot survive shutdown.
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
        error!("Windows audio recording thread terminated unexpectedly");
        let _ = status_tx.send(AppEvent::Error(
            "Audio recording failed unexpectedly".to_string(),
        ));
        let _ = audio_tx.send(None);
    }
}

fn run_wasapi_session(recording: &Arc<AtomicBool>) -> Result<PathBuf> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .context("No Windows input device is available")?;
    let supported = device
        .default_input_config()
        .context("Failed to read the default Windows input format")?;

    let sample_format = supported.sample_format();
    let config: StreamConfig = supported.into();
    let input_channels = usize::from(config.channels);
    let input_sample_rate = config.sample_rate.0;

    info!(
        "Using Windows input device '{}' at {} Hz, {} channel(s), {}",
        device.name().unwrap_or_else(|_| "unknown".to_string()),
        input_sample_rate,
        input_channels,
        sample_format
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
    let stream_failed = Arc::new(AtomicBool::new(false));

    let stream = match sample_format {
        SampleFormat::I8 => build_input_stream::<i8>(
            &device,
            &config,
            Arc::clone(&writer),
            Arc::clone(&stream_failed),
        ),
        SampleFormat::I16 => build_input_stream::<i16>(
            &device,
            &config,
            Arc::clone(&writer),
            Arc::clone(&stream_failed),
        ),
        SampleFormat::I32 => build_input_stream::<i32>(
            &device,
            &config,
            Arc::clone(&writer),
            Arc::clone(&stream_failed),
        ),
        SampleFormat::I64 => build_input_stream::<i64>(
            &device,
            &config,
            Arc::clone(&writer),
            Arc::clone(&stream_failed),
        ),
        SampleFormat::U8 => build_input_stream::<u8>(
            &device,
            &config,
            Arc::clone(&writer),
            Arc::clone(&stream_failed),
        ),
        SampleFormat::U16 => build_input_stream::<u16>(
            &device,
            &config,
            Arc::clone(&writer),
            Arc::clone(&stream_failed),
        ),
        SampleFormat::U32 => build_input_stream::<u32>(
            &device,
            &config,
            Arc::clone(&writer),
            Arc::clone(&stream_failed),
        ),
        SampleFormat::U64 => build_input_stream::<u64>(
            &device,
            &config,
            Arc::clone(&writer),
            Arc::clone(&stream_failed),
        ),
        SampleFormat::F32 => build_input_stream::<f32>(
            &device,
            &config,
            Arc::clone(&writer),
            Arc::clone(&stream_failed),
        ),
        SampleFormat::F64 => build_input_stream::<f64>(
            &device,
            &config,
            Arc::clone(&writer),
            Arc::clone(&stream_failed),
        ),
        other => anyhow::bail!("Unsupported Windows input sample format: {}", other),
    }?;

    stream
        .play()
        .context("Failed to start the Windows input stream")?;
    while recording.load(Ordering::SeqCst) && !stream_failed.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    drop(stream);

    if stream_failed.load(Ordering::SeqCst) {
        anyhow::bail!("The Windows input stream stopped unexpectedly");
    }

    if let Ok(mut guard) = writer.lock() {
        if let Some(writer) = guard.take() {
            writer.finalize().context("Failed to finalise WAV file")?;
        }
    }

    let metadata = std::fs::metadata(&audio_path)?;
    if metadata.len() < 100 {
        let _ = std::fs::remove_file(&audio_path);
        anyhow::bail!("Audio file too small ({} bytes)", metadata.len());
    }

    debug!("Windows audio file ready: {} bytes", metadata.len());
    audio_temp
        .keep()
        .context("Failed to retain completed audio recording")
}

fn build_input_stream<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    writer: SharedWriter,
    stream_failed: Arc<AtomicBool>,
) -> Result<Stream>
where
    T: Sample + SizedSample,
    f32: FromSample<T>,
{
    let channels = usize::from(config.channels);
    let input_rate = u64::from(config.sample_rate.0);
    let mut resample_phase = 0_u64;

    device
        .build_input_stream(
            config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                let Ok(mut guard) = writer.lock() else {
                    return;
                };
                let Some(writer) = guard.as_mut() else {
                    return;
                };

                for frame in data.chunks(channels) {
                    if frame.len() != channels {
                        continue;
                    }
                    let mono = frame
                        .iter()
                        .map(|sample| (*sample).to_sample::<f32>())
                        .sum::<f32>()
                        / channels as f32;
                    let output = (mono * 32_767.0).clamp(-32_768.0, 32_767.0) as i16;

                    resample_phase += u64::from(OUTPUT_SAMPLE_RATE);
                    while resample_phase >= input_rate {
                        resample_phase -= input_rate;
                        if writer.write_sample(output).is_err() {
                            break;
                        }
                    }
                }
            },
            move |error| {
                error!("Windows input stream error: {}", error);
                stream_failed.store(true, Ordering::SeqCst);
            },
            None,
        )
        .context("Failed to build the Windows input stream")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel::{unbounded, Receiver};
    use std::sync::atomic::AtomicUsize;
    use std::time::{Duration, Instant};

    const TEST_TIMEOUT: Duration = Duration::from_secs(2);

    fn spawn_test_worker(
        runner: SessionRunner,
    ) -> (
        Sender<RecordCommand>,
        Receiver<Option<PathBuf>>,
        Receiver<AppEvent>,
        JoinHandle<()>,
    ) {
        let (record_tx, record_rx) = unbounded();
        let (audio_tx, audio_rx) = unbounded();
        let (status_tx, status_rx) = unbounded();
        let handle = std::thread::spawn(move || {
            audio_worker_with_runner(record_rx, audio_tx, status_tx, runner);
        });
        (record_tx, audio_rx, status_rx, handle)
    }

    fn wait_for_stop(recording: &AtomicBool) -> Result<()> {
        let deadline = Instant::now() + TEST_TIMEOUT;
        while recording.load(Ordering::SeqCst) {
            if Instant::now() >= deadline {
                anyhow::bail!("synthetic recording did not stop");
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        Ok(())
    }

    fn synthetic_recording() -> Result<PathBuf> {
        let mut file = tempfile::Builder::new()
            .prefix("oswispa_windows_session_test_")
            .suffix(".wav")
            .tempfile()?;
        std::io::Write::write_all(&mut file, b"synthetic recording")?;
        let (file, path) = file.keep()?;
        drop(file);
        Ok(path)
    }

    fn remove_audio_output(output: Option<PathBuf>) {
        if let Some(path) = output {
            let _ = std::fs::remove_file(path);
        }
    }

    fn expect_error(status_rx: &Receiver<AppEvent>) {
        match status_rx.recv_timeout(TEST_TIMEOUT).unwrap() {
            AppEvent::Error(message) => assert!(!message.is_empty()),
            event => panic!("expected error event, received {event:?}"),
        }
    }

    #[test]
    fn output_format_matches_whisper_input_contract() {
        assert_eq!(OUTPUT_SAMPLE_RATE, 16_000);
        assert_eq!(OUTPUT_CHANNELS, 1);
    }

    #[test]
    fn stop_then_immediate_start_never_overlaps_sessions() {
        let active = Arc::new(AtomicUsize::new(0));
        let maximum_active = Arc::new(AtomicUsize::new(0));
        let calls = Arc::new(AtomicUsize::new(0));
        let (started_tx, started_rx) = unbounded();

        let runner: SessionRunner = Arc::new({
            let active = Arc::clone(&active);
            let maximum_active = Arc::clone(&maximum_active);
            let calls = Arc::clone(&calls);
            move |recording| {
                let call = calls.fetch_add(1, Ordering::SeqCst) + 1;
                let current_active = active.fetch_add(1, Ordering::SeqCst) + 1;
                maximum_active.fetch_max(current_active, Ordering::SeqCst);
                started_tx.send(call).unwrap();
                wait_for_stop(&recording)?;
                active.fetch_sub(1, Ordering::SeqCst);
                synthetic_recording()
            }
        });

        let (record_tx, audio_rx, _status_rx, worker) = spawn_test_worker(runner);
        record_tx.send(RecordCommand::Start).unwrap();
        assert_eq!(started_rx.recv_timeout(TEST_TIMEOUT).unwrap(), 1);

        record_tx.send(RecordCommand::Stop).unwrap();
        record_tx.send(RecordCommand::Start).unwrap();
        assert_eq!(started_rx.recv_timeout(TEST_TIMEOUT).unwrap(), 2);
        assert_eq!(maximum_active.load(Ordering::SeqCst), 1);

        record_tx.send(RecordCommand::Stop).unwrap();
        drop(record_tx);
        worker.join().unwrap();
        remove_audio_output(audio_rx.recv_timeout(TEST_TIMEOUT).unwrap());
        remove_audio_output(audio_rx.recv_timeout(TEST_TIMEOUT).unwrap());
    }

    #[test]
    fn old_session_completion_cannot_stop_new_session() {
        let calls = Arc::new(AtomicUsize::new(0));
        let (started_tx, started_rx) = unbounded();
        let runner: SessionRunner = Arc::new({
            let calls = Arc::clone(&calls);
            move |recording| {
                let call = calls.fetch_add(1, Ordering::SeqCst) + 1;
                started_tx.send((call, Arc::clone(&recording))).unwrap();
                wait_for_stop(&recording)?;
                synthetic_recording()
            }
        });

        let (record_tx, audio_rx, _status_rx, worker) = spawn_test_worker(runner);
        record_tx.send(RecordCommand::Start).unwrap();
        let (first_call, first_token) = started_rx.recv_timeout(TEST_TIMEOUT).unwrap();
        assert_eq!(first_call, 1);

        record_tx.send(RecordCommand::Stop).unwrap();
        record_tx.send(RecordCommand::Start).unwrap();
        let (second_call, second_token) = started_rx.recv_timeout(TEST_TIMEOUT).unwrap();
        assert_eq!(second_call, 2);
        assert!(!first_token.load(Ordering::SeqCst));
        assert!(second_token.load(Ordering::SeqCst));
        std::thread::sleep(Duration::from_millis(25));
        assert!(second_token.load(Ordering::SeqCst));

        record_tx.send(RecordCommand::Stop).unwrap();
        drop(record_tx);
        worker.join().unwrap();
        remove_audio_output(audio_rx.recv_timeout(TEST_TIMEOUT).unwrap());
        remove_audio_output(audio_rx.recv_timeout(TEST_TIMEOUT).unwrap());
    }

    #[test]
    fn repeated_start_does_not_create_a_second_session() {
        let calls = Arc::new(AtomicUsize::new(0));
        let (started_tx, started_rx) = unbounded();
        let runner: SessionRunner = Arc::new({
            let calls = Arc::clone(&calls);
            move |recording| {
                started_tx.send(()).unwrap();
                calls.fetch_add(1, Ordering::SeqCst);
                wait_for_stop(&recording)?;
                synthetic_recording()
            }
        });

        let (record_tx, audio_rx, _status_rx, worker) = spawn_test_worker(runner);
        record_tx.send(RecordCommand::Start).unwrap();
        started_rx.recv_timeout(TEST_TIMEOUT).unwrap();
        record_tx.send(RecordCommand::Start).unwrap();
        assert!(started_rx.recv_timeout(Duration::from_millis(100)).is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        record_tx.send(RecordCommand::Stop).unwrap();
        drop(record_tx);
        worker.join().unwrap();
        remove_audio_output(audio_rx.recv_timeout(TEST_TIMEOUT).unwrap());
    }

    #[test]
    fn cancel_removes_the_session_recording() {
        let (path_tx, path_rx) = unbounded();
        let runner: SessionRunner = Arc::new(move |recording| {
            let path = synthetic_recording()?;
            path_tx.send(path.clone()).unwrap();
            wait_for_stop(&recording)?;
            Ok(path)
        });

        let (record_tx, audio_rx, _status_rx, worker) = spawn_test_worker(runner);
        record_tx.send(RecordCommand::Start).unwrap();
        let path = path_rx.recv_timeout(TEST_TIMEOUT).unwrap();
        assert!(path.exists());
        record_tx.send(RecordCommand::Cancel).unwrap();
        assert!(audio_rx.recv_timeout(TEST_TIMEOUT).unwrap().is_none());
        assert!(!path.exists());

        drop(record_tx);
        worker.join().unwrap();
    }

    #[test]
    fn failed_session_is_reaped_before_the_next_start() {
        let calls = Arc::new(AtomicUsize::new(0));
        let (started_tx, started_rx) = unbounded();
        let runner: SessionRunner = Arc::new({
            let calls = Arc::clone(&calls);
            move |recording| {
                let call = calls.fetch_add(1, Ordering::SeqCst) + 1;
                started_tx.send(call).unwrap();
                if call == 1 {
                    anyhow::bail!("synthetic recorder failure");
                }
                wait_for_stop(&recording)?;
                synthetic_recording()
            }
        });

        let (record_tx, audio_rx, status_rx, worker) = spawn_test_worker(runner);
        record_tx.send(RecordCommand::Start).unwrap();
        assert_eq!(started_rx.recv_timeout(TEST_TIMEOUT).unwrap(), 1);
        assert!(audio_rx.recv_timeout(TEST_TIMEOUT).unwrap().is_none());
        expect_error(&status_rx);

        record_tx.send(RecordCommand::Start).unwrap();
        assert_eq!(started_rx.recv_timeout(TEST_TIMEOUT).unwrap(), 2);
        record_tx.send(RecordCommand::Stop).unwrap();
        remove_audio_output(audio_rx.recv_timeout(TEST_TIMEOUT).unwrap());

        drop(record_tx);
        worker.join().unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn panicked_session_is_contained_and_the_next_start_succeeds() {
        let calls = Arc::new(AtomicUsize::new(0));
        let (started_tx, started_rx) = unbounded();
        let runner: SessionRunner = Arc::new({
            let calls = Arc::clone(&calls);
            move |recording| {
                let call = calls.fetch_add(1, Ordering::SeqCst) + 1;
                started_tx.send(call).unwrap();
                if call == 1 {
                    panic!("synthetic recorder panic");
                }
                wait_for_stop(&recording)?;
                synthetic_recording()
            }
        });

        let (record_tx, audio_rx, status_rx, worker) = spawn_test_worker(runner);
        record_tx.send(RecordCommand::Start).unwrap();
        assert_eq!(started_rx.recv_timeout(TEST_TIMEOUT).unwrap(), 1);
        assert!(audio_rx.recv_timeout(TEST_TIMEOUT).unwrap().is_none());
        expect_error(&status_rx);

        record_tx.send(RecordCommand::Start).unwrap();
        assert_eq!(started_rx.recv_timeout(TEST_TIMEOUT).unwrap(), 2);
        record_tx.send(RecordCommand::Stop).unwrap();
        remove_audio_output(audio_rx.recv_timeout(TEST_TIMEOUT).unwrap());

        drop(record_tx);
        worker.join().unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }
}
