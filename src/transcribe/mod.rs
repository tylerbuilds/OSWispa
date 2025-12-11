//! Whisper.cpp transcription module
//!
//! Uses whisper-rs bindings to whisper.cpp for fast local transcription.
//!
//! Features:
//! - Standard batch transcription of complete recordings
//! - Streaming mode for real-time partial transcriptions

use crate::{AppEvent, Config};
use anyhow::Result;
use crossbeam_channel::{Receiver, Sender};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, error, info};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// Maximum retries before falling back to CPU
const MAX_GPU_RETRIES: u32 = 3;
/// Delay between retries in milliseconds
const RETRY_DELAY_MS: u64 = 500;

/// Transcription worker that processes audio files
pub fn transcription_worker(
    audio_rx: Receiver<Option<PathBuf>>,
    event_tx: Sender<AppEvent>,
    config: &Config,
) {
    info!("Initializing Whisper context (GPU mode)...");

    // Initialize primary Whisper context (GPU mode)
    let gpu_ctx = match WhisperContext::new_with_params(
        config.model_path.to_str().unwrap(),
        WhisperContextParameters::default(),
    ) {
        Ok(ctx) => {
            info!("Whisper GPU context initialized successfully");
            Some(Arc::new(ctx))
        }
        Err(e) => {
            error!("Failed to initialize Whisper GPU context: {}", e);
            None
        }
    };

    // Lazy-initialized CPU fallback context
    let mut cpu_ctx: Option<Arc<WhisperContext>> = None;

    if gpu_ctx.is_none() {
        error!("No GPU context available, will use CPU fallback for all requests");
    }

    for audio_path_opt in audio_rx {
        // Handle cancelled recordings (None)
        let audio_path = match audio_path_opt {
            Some(path) => path,
            None => {
                debug!("Received None - recording was cancelled, skipping transcription");
                continue;
            }
        };

        info!("Processing audio file: {:?}", audio_path);

        // Try GPU with retries first
        let mut transcription_result = None;
        
        if let Some(ref ctx) = gpu_ctx {
            for attempt in 1..=MAX_GPU_RETRIES {
                match transcribe_audio(ctx, &audio_path, config) {
                    Ok(text) => {
                        let trimmed = text.trim();
                        // Check for garbage output (repeated punctuation = GPU failure)
                        if !is_garbage_output(trimmed) {
                            transcription_result = Some(Ok(text));
                            break;
                        } else {
                            info!("GPU attempt {} returned garbage output, retrying...", attempt);
                        }
                    }
                    Err(e) => {
                        info!("GPU attempt {} failed: {}, retrying...", attempt, e);
                    }
                }
                
                if attempt < MAX_GPU_RETRIES {
                    std::thread::sleep(std::time::Duration::from_millis(RETRY_DELAY_MS));
                }
            }
        }

        // If GPU failed, fall back to CPU
        if transcription_result.is_none() {
            info!("GPU transcription failed after {} attempts, falling back to CPU...", MAX_GPU_RETRIES);
            
            // Initialize CPU context if not already done
            if cpu_ctx.is_none() {
                info!("Initializing CPU fallback context...");
                let mut cpu_params = WhisperContextParameters::default();
                cpu_params.use_gpu(false);
                
                match WhisperContext::new_with_params(
                    config.model_path.to_str().unwrap(),
                    cpu_params,
                ) {
                    Ok(ctx) => {
                        info!("CPU fallback context initialized successfully");
                        cpu_ctx = Some(Arc::new(ctx));
                    }
                    Err(e) => {
                        error!("Failed to initialize CPU fallback context: {}", e);
                    }
                }
            }
            
            // Try CPU transcription
            if let Some(ref ctx) = cpu_ctx {
                transcription_result = Some(transcribe_audio(ctx, &audio_path, config));
            }
        }

        // Process result
        match transcription_result {
            Some(Ok(text)) => {
                let text = text.trim().to_string();
                if !text.is_empty() && !is_garbage_output(&text) {
                    info!("Transcription successful: {} chars", text.len());
                    let _ = event_tx.send(AppEvent::TranscriptionComplete(text));
                } else {
                    info!("Empty or garbage transcription");
                    let _ = event_tx.send(AppEvent::Error("No speech detected".to_string()));
                }
            }
            Some(Err(e)) => {
                error!("Transcription failed: {}", e);
                let _ = event_tx.send(AppEvent::Error(format!("Transcription failed: {}", e)));
            }
            None => {
                error!("All transcription attempts failed");
                let _ = event_tx.send(AppEvent::Error("Transcription failed - GPU and CPU both unavailable".to_string()));
            }
        }

        // Clean up temp file
        if let Err(e) = std::fs::remove_file(&audio_path) {
            debug!("Failed to remove temp audio file: {}", e);
        }
    }
}

/// Check if output is garbage (repeated punctuation from GPU failure)
fn is_garbage_output(text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    
    // Count punctuation vs letters
    let punct_count = text.chars().filter(|c| c.is_ascii_punctuation()).count();
    let letter_count = text.chars().filter(|c| c.is_alphabetic()).count();
    
    // If more than 80% punctuation, it's garbage
    if letter_count == 0 {
        return punct_count > 3;
    }
    
    let punct_ratio = punct_count as f32 / (punct_count + letter_count) as f32;
    punct_ratio > 0.8
}

/// Transcribe a single audio file
fn transcribe_audio(ctx: &WhisperContext, audio_path: &PathBuf, config: &Config) -> Result<String> {
    // Load audio file
    let samples = load_wav_samples(audio_path)?;

    info!(
        "Loaded {} samples ({:.1}s of audio)",
        samples.len(),
        samples.len() as f32 / 16000.0
    );

    // Create whisper state
    let mut state = ctx.create_state()?;

    // Configure transcription parameters
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

    // Performance settings
    params.set_n_threads(num_cpus::get() as i32);

    // Language settings from config
    let language = if config.language == "auto" {
        None // Let Whisper detect
    } else {
        Some(config.language.as_str())
    };
    params.set_language(language);
    params.set_translate(config.translate_to_english);

    info!(
        "Transcribing with language: {:?}, translate: {}",
        language, config.translate_to_english
    );

    // Output settings
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);

    // Quality settings
    params.set_no_context(true);
    params.set_single_segment(false);

    // Run transcription
    state.full(params, &samples)?;

    // Collect results
    let num_segments = state.full_n_segments()?;
    let mut result = String::new();

    for i in 0..num_segments {
        let segment = state.full_get_segment_text(i)?;
        result.push_str(&segment);
        result.push(' ');
    }

    Ok(result)
}

/// Load WAV file as f32 samples (16kHz mono expected)
fn load_wav_samples(path: &PathBuf) -> Result<Vec<f32>> {
    let mut reader = hound::WavReader::open(path)?;
    let spec = reader.spec();

    debug!(
        "WAV spec: {} Hz, {} channels, {} bits",
        spec.sample_rate, spec.channels, spec.bits_per_sample
    );

    // Whisper expects 16kHz mono f32
    if spec.sample_rate != 16000 {
        anyhow::bail!(
            "Expected 16kHz audio, got {} Hz. Audio should be resampled before saving.",
            spec.sample_rate
        );
    }

    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => {
            let max_val = (1 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .filter_map(|s| s.ok())
                .map(|s| s as f32 / max_val)
                .collect()
        }
        hound::SampleFormat::Float => reader.samples::<f32>().filter_map(|s| s.ok()).collect(),
    };

    // Convert to mono if stereo
    let samples = if spec.channels == 2 {
        samples
            .chunks(2)
            .map(|chunk| (chunk[0] + chunk.get(1).copied().unwrap_or(0.0)) / 2.0)
            .collect()
    } else {
        samples
    };

    Ok(samples)
}

/// Shared Whisper context for streaming mode
pub struct StreamingTranscriber {
    ctx: Arc<WhisperContext>,
    config: Config,
}

impl StreamingTranscriber {
    /// Create a new streaming transcriber
    pub fn new(config: &Config) -> Result<Self> {
        let ctx = WhisperContext::new_with_params(
            config.model_path.to_str().unwrap(),
            WhisperContextParameters::default(),
        )?;

        Ok(Self {
            ctx: Arc::new(ctx),
            config: config.clone(),
        })
    }

    /// Transcribe a chunk of audio samples (f32, 16kHz mono)
    pub fn transcribe_chunk(&self, samples: &[f32]) -> Result<String> {
        if samples.is_empty() {
            return Ok(String::new());
        }

        let mut state = self.ctx.create_state()?;
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

        // Performance settings - use fewer threads for streaming to reduce latency
        params.set_n_threads((num_cpus::get() / 2).max(1) as i32);

        // Language settings
        let language = if self.config.language == "auto" {
            None
        } else {
            Some(self.config.language.as_str())
        };
        params.set_language(language);
        params.set_translate(self.config.translate_to_english);

        // Output settings
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        // Streaming-optimized settings
        params.set_no_context(true);
        params.set_single_segment(true); // Single segment for faster response

        // Run transcription
        state.full(params, samples)?;

        // Collect result
        let num_segments = state.full_n_segments()?;
        let mut result = String::new();

        for i in 0..num_segments {
            let segment = state.full_get_segment_text(i)?;
            result.push_str(&segment);
        }

        Ok(result.trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sample_loading() {
        // Would need a test WAV file
    }
}
