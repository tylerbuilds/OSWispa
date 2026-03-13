//! Audio recording module for macOS using cpal + hound.
//!
//! Uses the `cpal` crate (CoreAudio backend) for microphone input and
//! `hound` for WAV file writing. Pure Rust, no CLI dependencies.

use crate::{AppEvent, RecordCommand};
use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::{Receiver, Sender};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info, warn};

const SAMPLE_RATE: u32 = 16000;
const CHANNELS: u16 = 1;

/// Audio worker that listens for start/stop/cancel signals.
pub fn audio_worker(
    record_rx: Receiver<RecordCommand>,
    audio_tx: Sender<Option<PathBuf>>,
    status_tx: Sender<AppEvent>,
) {
    info!("Audio worker thread started (macOS/cpal backend)");
    let recording = Arc::new(AtomicBool::new(false));
    let cancelled = Arc::new(AtomicBool::new(false));

    let mut _recording_thread: Option<std::thread::JoinHandle<()>> = None;

    for cmd in record_rx {
        match cmd {
            RecordCommand::Start => {
                if recording.load(Ordering::SeqCst) {
                    info!("Already recording, ignoring start signal");
                    continue;
                }

                let recording_clone = Arc::clone(&recording);
                let cancelled_clone = Arc::clone(&cancelled);
                let audio_tx_clone = audio_tx.clone();
                let status_tx_clone = status_tx.clone();

                recording.store(true, Ordering::SeqCst);
                cancelled.store(false, Ordering::SeqCst);

                _recording_thread = Some(std::thread::spawn(move || {
                    info!("AudioWorker: Starting cpal recording session");
                    let result = run_cpal_session(&recording_clone);

                    match result {
                        Ok(path) => {
                            if cancelled_clone.load(Ordering::SeqCst) {
                                info!("Recording was cancelled, deleting file");
                                let _ = std::fs::remove_file(&path);
                                let _ = audio_tx_clone.send(None);
                            } else {
                                info!("Recording saved to {:?}", path);
                                let _ = audio_tx_clone.send(Some(path));
                            }
                        }
                        Err(e) => {
                            if !cancelled_clone.load(Ordering::SeqCst) {
                                error!("cpal recording session failed: {}", e);
                                let _ = status_tx_clone.send(AppEvent::Error(format!(
                                    "Audio recording failed: {}",
                                    e
                                )));
                            }
                            let _ = audio_tx_clone.send(None);
                        }
                    }

                    recording_clone.store(false, Ordering::SeqCst);
                }));
            }
            RecordCommand::Stop => {
                info!("Stop recording signal received");
                recording.store(false, Ordering::SeqCst);
            }
            RecordCommand::Cancel => {
                info!("Cancel recording signal received");
                cancelled.store(true, Ordering::SeqCst);
                recording.store(false, Ordering::SeqCst);
            }
        }
    }
}

/// Record audio from the default input device using cpal.
fn run_cpal_session(recording: &Arc<AtomicBool>) -> Result<PathBuf> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .context("No input device available")?;

    info!("Using input device: {}", device.name().unwrap_or_default());

    // Verify the device supports our desired config. CoreAudio can often
    // resample transparently, but we check to give a clear error if not.
    let supported = device
        .supported_input_configs()
        .context("Failed to query supported input configs")?;
    let supports_16k = supported.into_iter().any(|range| {
        range.channels() >= CHANNELS
            && range.min_sample_rate().0 <= SAMPLE_RATE
            && range.max_sample_rate().0 >= SAMPLE_RATE
    });

    if !supports_16k {
        warn!(
            "Input device may not natively support {}Hz mono. \
             CoreAudio will attempt transparent resampling.",
            SAMPLE_RATE
        );
    }

    let config = cpal::StreamConfig {
        channels: CHANNELS,
        sample_rate: cpal::SampleRate(SAMPLE_RATE),
        buffer_size: cpal::BufferSize::Default,
    };

    let temp_dir = std::env::temp_dir();
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S_%3f");
    let audio_path = temp_dir.join(format!("oswispa_recording_{}.wav", timestamp));

    let spec = hound::WavSpec {
        channels: CHANNELS,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let writer =
        hound::WavWriter::create(&audio_path, spec).context("Failed to create WAV file")?;
    let writer = Arc::new(Mutex::new(Some(writer)));
    let writer_clone = Arc::clone(&writer);

    let err_flag = Arc::new(AtomicBool::new(false));
    let err_flag_clone = Arc::clone(&err_flag);

    let stream = device
        .build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                if let Ok(mut guard) = writer_clone.lock() {
                    if let Some(ref mut w) = *guard {
                        for &sample in data {
                            let s16 = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
                            if w.write_sample(s16).is_err() {
                                break;
                            }
                        }
                    }
                }
            },
            move |err| {
                error!("cpal stream error: {}", err);
                err_flag_clone.store(true, Ordering::SeqCst);
            },
            None,
        )
        .context("Failed to build input stream")?;

    stream.play().context("Failed to start audio stream")?;
    info!("Recording started via cpal");

    // Wait until recording flag is cleared
    while recording.load(Ordering::SeqCst) && !err_flag.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    // Stop stream and finalize WAV
    drop(stream);

    if let Ok(mut guard) = writer.lock() {
        if let Some(w) = guard.take() {
            w.finalize().context("Failed to finalize WAV file")?;
        }
    }

    // Verify file
    let metadata = std::fs::metadata(&audio_path)?;
    if metadata.len() < 100 {
        let _ = std::fs::remove_file(&audio_path);
        anyhow::bail!("Audio file too small ({} bytes)", metadata.len());
    }

    debug!("Audio file ready: {} bytes", metadata.len());
    Ok(audio_path)
}
