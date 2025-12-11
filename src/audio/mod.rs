//! Audio recording module using CPAL
//!
//! Records audio from the default input device when triggered,
//! saves to a temporary WAV file for Whisper processing.
//!
//! Features:
//! - Voice Activity Detection (VAD) for auto-stop on silence
//! - Streaming mode for real-time transcription chunks

use crate::{AppEvent, Config, RecordCommand};
use anyhow::Result;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, SampleRate};
use crossbeam_channel::{Receiver, Sender};
use hound::WavWriter;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, error, info};

/// Whisper expects 16kHz mono audio
const WHISPER_SAMPLE_RATE: u32 = 16000;

/// Audio worker that listens for start/stop/cancel signals
pub fn audio_worker(
    record_rx: Receiver<RecordCommand>,
    audio_tx: Sender<Option<PathBuf>>,
    status_tx: Sender<AppEvent>,
    config: &Config,
) {
    let recording = Arc::new(AtomicBool::new(false));
    let cancelled = Arc::new(AtomicBool::new(false));
    let mut recording_thread: Option<std::thread::JoinHandle<Option<PathBuf>>> = None;

    let vad_config = config.vad.clone();
    let streaming_config = config.streaming.clone();

    for cmd in record_rx {
        match cmd {
            RecordCommand::Start => {
                if recording.load(Ordering::SeqCst) {
                    info!("Already recording, ignoring start signal");
                    continue;
                }

                let recording_clone = Arc::clone(&recording);
                let cancelled_clone = Arc::clone(&cancelled);
                let status_tx_clone = status_tx.clone();
                let vad_config_clone = vad_config.clone();

                recording.store(true, Ordering::SeqCst);
                cancelled.store(false, Ordering::SeqCst);

                recording_thread = Some(std::thread::spawn(move || {
                    match record_audio(&recording_clone, &vad_config_clone, &status_tx_clone) {
                        Ok(path) => {
                            if cancelled_clone.load(Ordering::SeqCst) {
                                info!("Recording was cancelled, deleting temp file");
                                let _ = std::fs::remove_file(&path);
                                None
                            } else {
                                info!("Recording saved to {:?}", path);
                                Some(path)
                            }
                        }
                        Err(e) => {
                            if !cancelled_clone.load(Ordering::SeqCst) {
                                error!("Recording failed: {}", e);
                                let _ = status_tx_clone
                                    .send(AppEvent::Error(format!("Recording failed: {}", e)));
                            }
                            None
                        }
                    }
                }));
            }
            RecordCommand::Stop => {
                info!("Stop recording signal received");
                recording.store(false, Ordering::SeqCst);

                if let Some(handle) = recording_thread.take() {
                    match handle.join() {
                        Ok(Some(path)) => {
                            if let Err(e) = audio_tx.send(Some(path)) {
                                error!("Failed to send audio path: {}", e);
                            }
                        }
                        Ok(None) => {
                            info!("No audio file produced (cancelled or error)");
                        }
                        Err(_) => {
                            error!("Recording thread panicked");
                        }
                    }
                }
            }
            RecordCommand::Cancel => {
                info!("Cancel recording signal received");
                cancelled.store(true, Ordering::SeqCst);
                recording.store(false, Ordering::SeqCst);

                if let Some(handle) = recording_thread.take() {
                    let _ = handle.join();
                }
                info!("Recording cancelled");
            }
        }
    }
}

/// Voice Activity Detection state
struct VadState {
    enabled: bool,
    threshold: f32,
    silence_duration_ms: u32,
    min_recording_ms: u32,
    recording_start: Instant,
    last_voice_time: Instant,
    triggered: bool,
}

impl VadState {
    fn new(config: &crate::VadConfig) -> Self {
        let now = Instant::now();
        Self {
            enabled: config.enabled,
            threshold: config.threshold,
            silence_duration_ms: config.silence_duration_ms,
            min_recording_ms: config.min_recording_ms,
            recording_start: now,
            last_voice_time: now,
            triggered: false,
        }
    }

    fn process_samples(&mut self, samples: &[f32]) -> bool {
        if !self.enabled || self.triggered {
            return false;
        }

        // Calculate RMS (root mean square) for volume level
        let rms = if samples.is_empty() {
            0.0
        } else {
            let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
            (sum_sq / samples.len() as f32).sqrt()
        };

        let now = Instant::now();

        // Check if there's voice activity
        if rms > self.threshold {
            self.last_voice_time = now;
        }

        // Check if we should trigger auto-stop
        let recording_duration = now.duration_since(self.recording_start).as_millis() as u32;
        let silence_duration = now.duration_since(self.last_voice_time).as_millis() as u32;

        if recording_duration >= self.min_recording_ms && silence_duration >= self.silence_duration_ms {
            debug!(
                "VAD triggered: recording={}ms, silence={}ms",
                recording_duration, silence_duration
            );
            self.triggered = true;
            return true;
        }

        false
    }
}

/// Record audio until the recording flag is set to false or VAD triggers
fn record_audio(
    recording: &Arc<AtomicBool>,
    vad_config: &crate::VadConfig,
    event_tx: &Sender<AppEvent>,
) -> Result<PathBuf> {
    let host = cpal::default_host();

    let device = host
        .default_input_device()
        .ok_or_else(|| anyhow::anyhow!("No input device available"))?;

    info!("Using input device: {}", device.name()?);

    let supported_configs = device.supported_input_configs()?;

    let config = supported_configs
        .filter(|c| c.channels() == 1 || c.channels() == 2)
        .filter(|c| {
            c.sample_format() == SampleFormat::F32 || c.sample_format() == SampleFormat::I16
        })
        .min_by_key(|c| {
            let rate = c.min_sample_rate().0;
            (rate as i32 - WHISPER_SAMPLE_RATE as i32).abs()
        })
        .ok_or_else(|| anyhow::anyhow!("No suitable audio config found"))?;

    let sample_rate = if config.min_sample_rate().0 <= WHISPER_SAMPLE_RATE
        && config.max_sample_rate().0 >= WHISPER_SAMPLE_RATE
    {
        SampleRate(WHISPER_SAMPLE_RATE)
    } else {
        config.min_sample_rate()
    };

    let config = config.with_sample_rate(sample_rate);
    let sample_format = config.sample_format();
    let channels = config.channels();

    info!(
        "Recording at {} Hz, {} channels, {:?}",
        sample_rate.0, channels, sample_format
    );

    let temp_dir = std::env::temp_dir();
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S_%3f");
    let audio_path = temp_dir.join(format!("oswispa_recording_{}.wav", timestamp));

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: WHISPER_SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let writer = WavWriter::create(&audio_path, spec)?;
    let writer = Arc::new(std::sync::Mutex::new(Some(writer)));
    let writer_clone = Arc::clone(&writer);

    let recording_clone = Arc::clone(recording);
    let source_rate = sample_rate.0;
    let source_channels = channels as usize;

    let resample_ratio = WHISPER_SAMPLE_RATE as f64 / source_rate as f64;
    let resample_state = Arc::new(std::sync::Mutex::new(ResampleState::new(resample_ratio)));

    // VAD state
    let vad_state = Arc::new(std::sync::Mutex::new(VadState::new(vad_config)));
    let vad_state_clone = Arc::clone(&vad_state);
    let event_tx_clone = event_tx.clone();

    let err_fn = |err| error!("Audio stream error: {}", err);

    let stream = match sample_format {
        SampleFormat::F32 => {
            let resample_state = Arc::clone(&resample_state);
            device.build_input_stream(
                &config.into(),
                move |data: &[f32], _: &_| {
                    if !recording_clone.load(Ordering::SeqCst) {
                        return;
                    }

                    let mono: Vec<f32> = if source_channels == 2 {
                        data.chunks(2)
                            .map(|chunk| (chunk[0] + chunk.get(1).copied().unwrap_or(0.0)) / 2.0)
                            .collect()
                    } else {
                        data.to_vec()
                    };

                    // Check VAD
                    {
                        let mut vad = vad_state_clone.lock().unwrap();
                        if vad.process_samples(&mono) {
                            info!("VAD: Silence detected, auto-stopping");
                            let _ = event_tx_clone.send(AppEvent::VadSilenceDetected);
                        }
                    }

                    let mut state = resample_state.lock().unwrap();
                    let resampled = state.resample(&mono);

                    if let Some(ref mut writer) = *writer_clone.lock().unwrap() {
                        for sample in resampled {
                            let sample_i16 = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
                            let _ = writer.write_sample(sample_i16);
                        }
                    }
                },
                err_fn,
                None,
            )?
        }
        SampleFormat::I16 => {
            let resample_state = Arc::clone(&resample_state);
            device.build_input_stream(
                &config.into(),
                move |data: &[i16], _: &_| {
                    if !recording_clone.load(Ordering::SeqCst) {
                        return;
                    }

                    let mono: Vec<f32> = if source_channels == 2 {
                        data.chunks(2)
                            .map(|chunk| {
                                let l = chunk[0] as f32 / 32768.0;
                                let r = chunk.get(1).map(|&v| v as f32 / 32768.0).unwrap_or(0.0);
                                (l + r) / 2.0
                            })
                            .collect()
                    } else {
                        data.iter().map(|&s| s as f32 / 32768.0).collect()
                    };

                    // Check VAD
                    {
                        let mut vad = vad_state_clone.lock().unwrap();
                        if vad.process_samples(&mono) {
                            info!("VAD: Silence detected, auto-stopping");
                            let _ = event_tx_clone.send(AppEvent::VadSilenceDetected);
                        }
                    }

                    let mut state = resample_state.lock().unwrap();
                    let resampled = state.resample(&mono);

                    if let Some(ref mut writer) = *writer_clone.lock().unwrap() {
                        for sample in resampled {
                            let sample_i16 = (sample * 32767.0).clamp(-32768.0, 32767.0) as i16;
                            let _ = writer.write_sample(sample_i16);
                        }
                    }
                },
                err_fn,
                None,
            )?
        }
        _ => {
            anyhow::bail!("Unsupported sample format: {:?}", sample_format);
        }
    };

    stream.play()?;
    info!("Recording started...");

    while recording.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    drop(stream);

    if let Some(writer) = writer.lock().unwrap().take() {
        writer.finalize()?;
    }

    info!("Recording finalized");
    Ok(audio_path)
}

/// Simple resampling state for linear interpolation
struct ResampleState {
    ratio: f64,
    position: f64,
    #[allow(dead_code)]
    last_sample: f32,
}

impl ResampleState {
    fn new(ratio: f64) -> Self {
        Self {
            ratio,
            position: 0.0,
            last_sample: 0.0,
        }
    }

    fn resample(&mut self, input: &[f32]) -> Vec<f32> {
        if (self.ratio - 1.0).abs() < 0.001 {
            return input.to_vec();
        }

        let mut output = Vec::new();
        let input_len = input.len();

        while (self.position as usize) < input_len {
            let idx = self.position as usize;
            let frac = self.position - idx as f64;

            let sample = if idx + 1 < input_len {
                input[idx] * (1.0 - frac as f32) + input[idx + 1] * frac as f32
            } else {
                input[idx]
            };

            output.push(sample);
            self.position += 1.0 / self.ratio;
        }

        self.position -= input_len as f64;
        if let Some(&last) = input.last() {
            self.last_sample = last;
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resample_ratio() {
        let mut state = ResampleState::new(0.5);
        let input: Vec<f32> = (0..100).map(|i| i as f32 / 100.0).collect();
        let output = state.resample(&input);
        assert!(output.len() < input.len());
    }
}
