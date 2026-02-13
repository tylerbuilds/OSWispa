//! Whisper.cpp transcription module
//!
//! Uses whisper-rs bindings to whisper.cpp for fast local transcription.
//!
//! Features:
//! - Lazy GPU context initialization (only allocates VRAM when needed)
//! - Automatic fallback to smaller model when VRAM is constrained
//! - CPU fallback when GPU is unavailable

use crate::{AppEvent, Config};
use anyhow::Result;
use crossbeam_channel::{Receiver, Sender};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tracing::{debug, error, info, warn};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// Minimum available VRAM in bytes to attempt GPU transcription (2GB)
const MIN_VRAM_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// Transcription worker that processes audio files with lazy context initialization
pub fn transcription_worker(
    audio_rx: Receiver<Option<PathBuf>>,
    event_tx: Sender<AppEvent>,
    config: Arc<RwLock<Config>>,
) {
    info!("Transcription worker started (lazy initialization mode)");

    let startup_config = config.read().unwrap().clone();
    if let Some(ref fallback) = startup_config.fallback_model_path {
        info!("Fallback model configured: {:?}", fallback);
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

        let current_config = config.read().unwrap().clone();

        // Try transcription with fallback chain
        let result = transcribe_with_fallback(&audio_path, &current_config);

        // Process result
        match result {
            Ok(text) => {
                let text = text.trim().to_string();
                if !text.is_empty() && !is_garbage_output(&text) {
                    info!("Transcription successful: {} chars", text.len());
                    let _ = event_tx.send(AppEvent::TranscriptionComplete(text));
                } else {
                    info!("Empty or garbage transcription");
                    let _ = event_tx.send(AppEvent::Error("No speech detected".to_string()));
                }
            }
            Err(e) => {
                error!("Transcription failed: {}", e);
                let _ = event_tx.send(AppEvent::Error(format!("Transcription failed: {}", e)));
            }
        }

        // Clean up temp file
        if let Err(e) = std::fs::remove_file(&audio_path) {
            debug!("Failed to remove temp audio file: {}", e);
        }
    }
}

/// Transcribe with fallback chain: Primary GPU -> Fallback GPU -> Primary CPU -> Fallback CPU
fn transcribe_with_fallback(audio_path: &PathBuf, config: &Config) -> Result<String> {
    let primary_model = &config.model_path;
    let fallback_model = config.fallback_model_path.as_ref();

    // Check available VRAM
    let available_vram = get_available_vram();
    info!("Available VRAM: {:.2} GB", available_vram as f64 / 1024.0 / 1024.0 / 1024.0);

    // Strategy based on VRAM availability
    if available_vram >= MIN_VRAM_BYTES {
        // Enough VRAM - try primary model on GPU
        info!("Sufficient VRAM available, attempting GPU transcription with primary model");

        match transcribe_with_model(audio_path, primary_model, config, true) {
            Ok(text) if !is_garbage_output(text.trim()) => {
                return Ok(text);
            }
            Ok(_) => {
                warn!("Primary GPU returned garbage output");
            }
            Err(e) => {
                warn!("Primary GPU transcription failed: {}", e);
            }
        }

        // GPU failed - try fallback model on GPU if available
        if let Some(fallback) = fallback_model {
            if fallback.exists() {
                info!("Trying fallback model on GPU: {:?}", fallback);
                match transcribe_with_model(audio_path, fallback, config, true) {
                    Ok(text) if !is_garbage_output(text.trim()) => {
                        return Ok(text);
                    }
                    Ok(_) => {
                        warn!("Fallback GPU returned garbage output");
                    }
                    Err(e) => {
                        warn!("Fallback GPU transcription failed: {}", e);
                    }
                }
            }
        }
    } else {
        // Low VRAM - skip straight to fallback model if available
        warn!("Low VRAM detected ({:.2} GB < {:.2} GB threshold), skipping primary GPU",
              available_vram as f64 / 1024.0 / 1024.0 / 1024.0,
              MIN_VRAM_BYTES as f64 / 1024.0 / 1024.0 / 1024.0);

        if let Some(fallback) = fallback_model {
            if fallback.exists() {
                info!("Trying fallback model on GPU (smaller footprint): {:?}", fallback);
                match transcribe_with_model(audio_path, fallback, config, true) {
                    Ok(text) if !is_garbage_output(text.trim()) => {
                        return Ok(text);
                    }
                    Ok(_) => {
                        warn!("Fallback GPU returned garbage output");
                    }
                    Err(e) => {
                        warn!("Fallback GPU transcription failed: {}", e);
                    }
                }
            }
        }
    }

    // All GPU attempts failed - fall back to CPU
    info!("GPU transcription failed, falling back to CPU");

    // Try fallback model on CPU first (faster)
    if let Some(fallback) = fallback_model {
        if fallback.exists() {
            info!("Trying fallback model on CPU: {:?}", fallback);
            match transcribe_with_model(audio_path, fallback, config, false) {
                Ok(text) if !is_garbage_output(text.trim()) => {
                    return Ok(text);
                }
                Ok(_) => {
                    warn!("Fallback CPU returned garbage output");
                }
                Err(e) => {
                    warn!("Fallback CPU transcription failed: {}", e);
                }
            }
        }
    }

    // Last resort: primary model on CPU
    info!("Trying primary model on CPU (last resort): {:?}", primary_model);
    transcribe_with_model(audio_path, primary_model, config, false)
}

/// Transcribe using a specific model with lazy context initialization
fn transcribe_with_model(
    audio_path: &PathBuf,
    model_path: &Path,
    config: &Config,
    use_gpu: bool,
) -> Result<String> {
    let mode_str = if use_gpu { "GPU" } else { "CPU" };
    info!("Creating Whisper context ({} mode) for {:?}", mode_str, model_path);

    // Create context parameters
    let mut ctx_params = WhisperContextParameters::default();
    if !use_gpu {
        ctx_params.use_gpu(false);
    }

    // Lazy initialization - create context just for this transcription
    let ctx = WhisperContext::new_with_params(
        model_path.to_str().ok_or_else(|| anyhow::anyhow!("Invalid model path"))?,
        ctx_params,
    )?;

    info!("Whisper {} context created successfully", mode_str);

    // Perform transcription
    let result = transcribe_audio(&ctx, audio_path, config);

    // Context is dropped here, releasing VRAM
    info!("Whisper {} context released", mode_str);

    result
}

/// Get available VRAM in bytes by querying the AMD GPU
fn get_available_vram() -> u64 {
    // Try to read from sysfs (works for AMD GPUs)
    // Look for the discrete GPU (card1 typically, but we'll check for the larger one)

    let cards = [
        "/sys/class/drm/card1/device/mem_info_vram_used",
        "/sys/class/drm/card0/device/mem_info_vram_used",
    ];

    let totals = [
        "/sys/class/drm/card1/device/mem_info_vram_total",
        "/sys/class/drm/card0/device/mem_info_vram_total",
    ];

    for (used_path, total_path) in cards.iter().zip(totals.iter()) {
        if let (Ok(used_str), Ok(total_str)) = (
            std::fs::read_to_string(used_path),
            std::fs::read_to_string(total_path),
        ) {
            if let (Ok(used), Ok(total)) = (
                used_str.trim().parse::<u64>(),
                total_str.trim().parse::<u64>(),
            ) {
                // Only consider GPUs with significant VRAM (>1GB = discrete GPU)
                if total > 1024 * 1024 * 1024 {
                    let available = total.saturating_sub(used);
                    debug!("Found GPU at {}: total={}, used={}, available={}",
                           used_path, total, used, available);
                    return available;
                }
            }
        }
    }

    // Fallback: try rocm-smi
    if let Ok(output) = std::process::Command::new("rocm-smi")
        .args(["--showmeminfo", "vram"])
        .output()
    {
        if let Ok(stdout) = String::from_utf8(output.stdout) {
            // Parse rocm-smi output for VRAM info
            // Look for lines like "GPU[0]		: VRAM Total Used Memory (B): 16974520320"
            let mut total: u64 = 0;
            let mut used: u64 = 0;

            for line in stdout.lines() {
                if line.contains("VRAM Total Memory") && line.contains("GPU[0]") {
                    if let Some(val) = line.split(':').last() {
                        total = val.trim().parse().unwrap_or(0);
                    }
                }
                if line.contains("VRAM Total Used") && line.contains("GPU[0]") {
                    if let Some(val) = line.split(':').last() {
                        used = val.trim().parse().unwrap_or(0);
                    }
                }
            }

            if total > 0 {
                return total.saturating_sub(used);
            }
        }
    }

    // If we can't determine VRAM, assume we have enough (optimistic)
    warn!("Could not determine available VRAM, assuming sufficient");
    MIN_VRAM_BYTES + 1
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
    fn test_garbage_detection() {
        assert!(is_garbage_output("...."));
        assert!(is_garbage_output("!!!???..."));
        assert!(!is_garbage_output("Hello world"));
        assert!(!is_garbage_output("Hello, world!"));
    }

    #[test]
    fn test_vram_detection() {
        let vram = get_available_vram();
        println!("Detected available VRAM: {} bytes", vram);
        // Just ensure it doesn't panic
    }
}
