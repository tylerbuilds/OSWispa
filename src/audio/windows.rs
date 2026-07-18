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
use tracing::{debug, error, info};

const OUTPUT_SAMPLE_RATE: u32 = 16_000;
const OUTPUT_CHANNELS: u16 = 1;

type WavWriter = hound::WavWriter<BufWriter<File>>;
type SharedWriter = Arc<Mutex<Option<WavWriter>>>;

/// Listen for start, stop and cancel commands from the application event loop.
pub fn audio_worker(
    record_rx: Receiver<RecordCommand>,
    audio_tx: Sender<Option<PathBuf>>,
    status_tx: Sender<AppEvent>,
) {
    info!("Audio worker thread started (Windows/WASAPI backend)");
    let recording = Arc::new(AtomicBool::new(false));
    let cancelled = Arc::new(AtomicBool::new(false));
    let mut recording_thread: Option<std::thread::JoinHandle<()>> = None;

    for command in record_rx {
        match command {
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

                recording_thread = Some(std::thread::spawn(move || {
                    let result = run_wasapi_session(&recording_clone);
                    match result {
                        Ok(path) if cancelled_clone.load(Ordering::SeqCst) => {
                            info!("Recording was cancelled, deleting file");
                            let _ = std::fs::remove_file(&path);
                            let _ = audio_tx_clone.send(None);
                        }
                        Ok(path) => {
                            info!("Recording saved to {:?}", path);
                            let _ = audio_tx_clone.send(Some(path));
                        }
                        Err(error) => {
                            if !cancelled_clone.load(Ordering::SeqCst) {
                                error!("WASAPI recording session failed: {}", error);
                                let _ = status_tx_clone.send(AppEvent::Error(format!(
                                    "Audio recording failed: {}",
                                    error
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

    if let Some(thread) = recording_thread {
        let _ = thread.join();
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

    #[test]
    fn output_format_matches_whisper_input_contract() {
        assert_eq!(OUTPUT_SAMPLE_RATE, 16_000);
        assert_eq!(OUTPUT_CHANNELS, 1);
    }
}
