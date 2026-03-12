//! Whisper.cpp transcription module
//!
//! Uses whisper-rs bindings to whisper.cpp for fast local transcription.
//!
//! Features:
//! - Lazy GPU context initialization (only allocates VRAM when needed)
//! - Automatic fallback to smaller model when VRAM is constrained
//! - CPU fallback when GPU is unavailable

use crate::{AppEvent, Config, StreamingAudioMessage, TranscriptionBackend};
use anyhow::Result;
use crossbeam_channel::{select, Receiver, Sender};
use reqwest::blocking::{multipart, Client};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tracing::{debug, error, info, warn};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

/// Minimum available VRAM in bytes to attempt GPU transcription (2GB)
const MIN_VRAM_BYTES: u64 = 2 * 1024 * 1024 * 1024;
/// Keep a conservative reserve so OSWispa does not consume the last chunk of free VRAM.
const GPU_RESERVED_HEADROOM_BYTES: u64 = 6 * 1024 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
struct CacheSignature {
    backend: TranscriptionBackend,
    model_path: PathBuf,
    fallback_model_path: Option<PathBuf>,
}

impl From<&Config> for CacheSignature {
    fn from(config: &Config) -> Self {
        Self {
            backend: config.backend.clone(),
            model_path: config.model_path.clone(),
            fallback_model_path: config.fallback_model_path.clone(),
        }
    }
}

struct CachedContext {
    model_path: PathBuf,
    use_gpu: bool,
    ctx: WhisperContext,
}

#[derive(Default)]
struct LiveStreamState {
    transcript: String,
}

#[derive(Default)]
struct ContextCache {
    contexts: Vec<CachedContext>,
}

impl ContextCache {
    fn clear(&mut self) {
        self.contexts.clear();
    }

    fn contains(&self, model_path: &Path, use_gpu: bool) -> bool {
        self.contexts
            .iter()
            .any(|entry| entry.use_gpu == use_gpu && entry.model_path == model_path)
    }

    fn get_or_create(&mut self, model_path: &Path, use_gpu: bool) -> Result<&WhisperContext> {
        if let Some(index) = self
            .contexts
            .iter()
            .position(|entry| entry.use_gpu == use_gpu && entry.model_path == model_path)
        {
            debug!(
                "Reusing Whisper context ({} mode) for {:?}",
                if use_gpu { "GPU" } else { "CPU" },
                model_path
            );
            return Ok(&self.contexts[index].ctx);
        }

        let mode_str = if use_gpu { "GPU" } else { "CPU" };
        info!(
            "Creating Whisper context ({} mode) for {:?}",
            mode_str, model_path
        );

        let mut ctx_params = WhisperContextParameters::default();
        if !use_gpu {
            ctx_params.use_gpu(false);
        }

        let ctx = WhisperContext::new_with_params(
            model_path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid model path"))?,
            ctx_params,
        )?;

        info!("Whisper {} context created successfully", mode_str);

        self.contexts.push(CachedContext {
            model_path: model_path.to_path_buf(),
            use_gpu,
            ctx,
        });
        let index = self.contexts.len() - 1;
        Ok(&self.contexts[index].ctx)
    }
}

/// Transcription worker that processes audio files with lazy context initialization
pub fn transcription_worker(
    audio_rx: Receiver<Option<PathBuf>>,
    stream_rx: Receiver<StreamingAudioMessage>,
    event_tx: Sender<AppEvent>,
    config: Arc<RwLock<Config>>,
) {
    info!("Transcription worker started (lazy initialization mode)");

    let startup_config = config.read().unwrap().clone();
    if let Some(ref fallback) = startup_config.fallback_model_path {
        info!("Fallback model configured: {:?}", fallback);
    }
    let mut context_cache = ContextCache::default();
    let mut cache_signature = CacheSignature::from(&startup_config);
    let mut live_stream = LiveStreamState::default();

    if startup_config.backend == TranscriptionBackend::Local {
        prewarm_active_local_context(&startup_config, &mut context_cache);
    }

    loop {
        select! {
            recv(stream_rx) -> stream_msg => {
                let current_config = refresh_config_cache(
                    &config,
                    &mut context_cache,
                    &mut cache_signature,
                );

                match stream_msg {
                    Ok(StreamingAudioMessage::Begin) => {
                        live_stream = LiveStreamState::default();
                    }
                    Ok(StreamingAudioMessage::Chunk(samples)) => {
                        if !should_use_live_streaming(&current_config) {
                            continue;
                        }

                        match transcribe_stream_chunk(&samples, &current_config, &mut context_cache) {
                            Ok(text) => {
                                let text = text.trim();
                                if !text.is_empty() && !is_garbage_output(text) {
                                    append_stream_text(&mut live_stream.transcript, text);
                                    let _ = event_tx.send(AppEvent::StreamingPartial(
                                        live_stream.transcript.clone(),
                                    ));
                                }
                            }
                            Err(err) => warn!("Streaming chunk transcription failed: {}", err),
                        }
                    }
                    Ok(StreamingAudioMessage::Finalize) => {
                        debug!(
                            "Streaming finalized with {} chars of partial text; waiting for full-file pass",
                            live_stream.transcript.len()
                        );
                    }
                    Ok(StreamingAudioMessage::Cancel) => {
                        live_stream = LiveStreamState::default();
                    }
                    Err(_) => break,
                }
            }
            recv(audio_rx) -> audio_path_opt => {
                let current_config = refresh_config_cache(
                    &config,
                    &mut context_cache,
                    &mut cache_signature,
                );

                let audio_path = match audio_path_opt {
                    Ok(Some(path)) => path,
                    Ok(None) => {
                        debug!("Received None - recording was cancelled, skipping transcription");
                        live_stream = LiveStreamState::default();
                        continue;
                    }
                    Err(_) => break,
                };

                info!("Processing audio file: {:?}", audio_path);

                let result = match current_config.backend {
                    TranscriptionBackend::Local => {
                        transcribe_with_fallback(&audio_path, &current_config, &mut context_cache)
                    }
                    TranscriptionBackend::Remote => transcribe_remote_with_local_fallback(
                        &audio_path,
                        &current_config,
                        &mut context_cache,
                    ),
                };

                emit_transcription_result(result, &event_tx);
                live_stream = LiveStreamState::default();

                if let Err(e) = std::fs::remove_file(&audio_path) {
                    debug!("Failed to remove temp audio file: {}", e);
                }
            }
        }
    }
}

fn refresh_config_cache(
    config: &Arc<RwLock<Config>>,
    context_cache: &mut ContextCache,
    cache_signature: &mut CacheSignature,
) -> Config {
    let current_config = config.read().unwrap().clone();
    let current_signature = CacheSignature::from(&current_config);
    if current_signature != *cache_signature {
        info!("Transcription config changed - resetting cached Whisper contexts");
        context_cache.clear();
        *cache_signature = current_signature;

        if current_config.backend == TranscriptionBackend::Local {
            prewarm_active_local_context(&current_config, context_cache);
        }
    }

    current_config
}

fn should_use_live_streaming(config: &Config) -> bool {
    config.backend == TranscriptionBackend::Local && config.streaming.enabled
}

fn emit_transcription_result(result: Result<String>, event_tx: &Sender<AppEvent>) {
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
}

fn append_stream_text(existing: &mut String, incoming: &str) {
    let incoming_words: Vec<&str> = incoming.split_whitespace().collect();
    if incoming_words.is_empty() {
        return;
    }

    if existing.trim().is_empty() {
        existing.push_str(incoming.trim());
        return;
    }

    let existing_words: Vec<&str> = existing.split_whitespace().collect();
    let max_overlap = existing_words.len().min(incoming_words.len()).min(6);
    let mut overlap = 0;

    for candidate in (1..=max_overlap).rev() {
        let existing_tail = &existing_words[existing_words.len() - candidate..];
        let incoming_head = &incoming_words[..candidate];
        if existing_tail
            .iter()
            .zip(incoming_head.iter())
            .all(|(left, right)| left.eq_ignore_ascii_case(right))
        {
            overlap = candidate;
            break;
        }
    }

    let suffix = incoming_words[overlap..].join(" ");
    if suffix.is_empty() {
        return;
    }

    if !existing.ends_with(' ') {
        existing.push(' ');
    }
    existing.push_str(&suffix);
}

/// Remote transcription with automatic fallback to local model execution.
fn transcribe_remote_with_local_fallback(
    audio_path: &PathBuf,
    config: &Config,
    context_cache: &mut ContextCache,
) -> Result<String> {
    match transcribe_with_remote_backend(audio_path, config) {
        Ok(text) => Ok(text),
        Err(err) => {
            warn!("Remote backend transcription failed: {}", err);

            if config.model_path.exists()
                || config
                    .fallback_model_path
                    .as_ref()
                    .map(|p| p.exists())
                    .unwrap_or(false)
            {
                warn!("Falling back to local transcription backend");
                transcribe_with_fallback(audio_path, config, context_cache)
            } else {
                Err(anyhow::anyhow!(
                    "Remote transcription failed and no local model is available: {}",
                    err
                ))
            }
        }
    }
}

fn resolve_remote_api_key(config: &Config) -> Option<String> {
    if let Some(env_var) = config
        .remote_backend
        .api_key_env
        .as_ref()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
    {
        return std::env::var(env_var)
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty());
    }

    crate::get_remote_api_key()
}

fn validate_remote_endpoint(endpoint: &str, allow_insecure_http: bool) -> Result<()> {
    let url = reqwest::Url::parse(endpoint)?;
    if url.scheme() != "https" && !allow_insecure_http {
        anyhow::bail!(
            "Remote endpoint must use HTTPS. Enable allow_insecure_http to opt into plain HTTP."
        );
    }
    Ok(())
}

fn transcribe_with_remote_backend(audio_path: &PathBuf, config: &Config) -> Result<String> {
    let endpoint = config.remote_backend.endpoint.trim();
    if endpoint.is_empty() {
        anyhow::bail!("Remote backend endpoint is empty");
    }

    validate_remote_endpoint(endpoint, config.remote_backend.allow_insecure_http)?;

    let timeout_ms = config.remote_backend.timeout_ms.max(1_000);
    let client = Client::builder()
        .timeout(Duration::from_millis(timeout_ms))
        .build()?;

    let mut form = multipart::Form::new().text("model", config.remote_backend.model.clone());

    if config.language != "auto" {
        form = form.text("language", config.language.clone());
    }
    if config.translate_to_english {
        form = form.text("task", "translate");
    }

    let audio_bytes = std::fs::read(audio_path)?;
    let audio_part = multipart::Part::bytes(audio_bytes)
        .file_name("audio.wav")
        .mime_str("audio/wav")?;
    form = form.part("file", audio_part);

    let mut request = client
        .post(endpoint)
        .header("Accept", "application/json")
        .multipart(form);

    if let Some(token) = resolve_remote_api_key(config) {
        request = request.bearer_auth(token);
    }

    let response = request.send()?;
    let status = response.status();
    let body = response.text()?;

    if !status.is_success() {
        let snippet: String = body.chars().take(300).collect();
        anyhow::bail!("Remote backend returned {}: {}", status, snippet);
    }

    if let Ok(json) = serde_json::from_str::<Value>(&body) {
        if let Some(text) = json.get("text").and_then(|v| v.as_str()) {
            return Ok(text.to_string());
        }

        if let Some(text) = json
            .get("choices")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|first| first.get("text"))
            .and_then(|v| v.as_str())
        {
            return Ok(text.to_string());
        }
    }

    let plain = body.trim();
    if !plain.is_empty() {
        return Ok(plain.to_string());
    }

    anyhow::bail!("Remote backend response did not include transcribed text")
}

/// Transcribe with fallback chain: Primary GPU -> Fallback GPU -> Primary CPU -> Fallback CPU
fn transcribe_with_fallback(
    audio_path: &PathBuf,
    config: &Config,
    context_cache: &mut ContextCache,
) -> Result<String> {
    let primary_model = &config.model_path;
    let fallback_model = config.fallback_model_path.as_ref();

    // Check available VRAM
    let available_vram = get_available_vram();
    info!(
        "Available VRAM: {:.2} GB",
        available_vram as f64 / 1024.0 / 1024.0 / 1024.0
    );

    // Strategy based on VRAM availability
    let primary_gpu_safe =
        context_cache.contains(primary_model, true) || can_run_model_on_gpu(primary_model, available_vram);
    if primary_gpu_safe {
        // Enough VRAM - try primary model on GPU
        info!("Sufficient VRAM available, attempting GPU transcription with primary model");

        match transcribe_with_model(audio_path, primary_model, config, true, context_cache) {
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

        warn!("Primary GPU attempt did not complete cleanly, skipping further GPU fallback");
    } else {
        // Safe GPU headroom unavailable - skip GPU entirely.
        warn!(
            "Safe GPU headroom unavailable ({:.2} GB free), skipping primary GPU",
            available_vram as f64 / 1024.0 / 1024.0 / 1024.0,
        );
    }

    // All GPU attempts failed - fall back to CPU
    info!("GPU transcription failed, falling back to CPU");

    // Try fallback model on CPU first (faster)
    if let Some(fallback) = fallback_model {
        if fallback.exists() {
            info!("Trying fallback model on CPU: {:?}", fallback);
            match transcribe_with_model(audio_path, fallback, config, false, context_cache) {
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
    info!(
        "Trying primary model on CPU (last resort): {:?}",
        primary_model
    );
    transcribe_with_model(audio_path, primary_model, config, false, context_cache)
}

fn prewarm_active_local_context(config: &Config, context_cache: &mut ContextCache) {
    if !config.model_path.exists() {
        return;
    }

    let use_gpu = can_run_model_on_gpu(&config.model_path, get_available_vram());
    match context_cache.get_or_create(&config.model_path, use_gpu) {
        Ok(_) => info!(
            "Prewarmed active local model ({}) for {:?}",
            if use_gpu { "GPU" } else { "CPU" },
            config.model_path
        ),
        Err(err) => warn!("Failed to prewarm active local model: {}", err),
    }
}

/// Transcribe using a specific model with a reusable cached context
fn transcribe_with_model(
    audio_path: &PathBuf,
    model_path: &Path,
    config: &Config,
    use_gpu: bool,
    context_cache: &mut ContextCache,
) -> Result<String> {
    let ctx = context_cache.get_or_create(model_path, use_gpu)?;
    transcribe_audio(ctx, audio_path, config)
}

/// Get available VRAM in bytes by querying the GPU.
///
/// Detection chain: AMD sysfs → rocm-smi → nvidia-smi → assume sufficient.
fn get_available_vram() -> u64 {
    // 1. AMD sysfs (fastest, no subprocess)
    if let Some(vram) = detect_vram_amd_sysfs() {
        return vram;
    }

    // 2. rocm-smi (AMD fallback)
    if let Some(vram) = detect_vram_rocm_smi() {
        return vram;
    }

    // 3. nvidia-smi (NVIDIA GPUs)
    if let Some(vram) = detect_vram_nvidia_smi() {
        return vram;
    }

    // If we can't determine VRAM, stay conservative and avoid GPU use.
    warn!("Could not determine available VRAM, defaulting to CPU-safe mode");
    0
}

/// Detect available VRAM via AMD sysfs paths.
fn detect_vram_amd_sysfs() -> Option<u64> {
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
                    debug!(
                        "AMD sysfs GPU at {}: total={}, used={}, available={}",
                        used_path, total, used, available
                    );
                    return Some(available);
                }
            }
        }
    }

    None
}

/// Detect available VRAM via rocm-smi (AMD).
fn detect_vram_rocm_smi() -> Option<u64> {
    let output = std::process::Command::new("rocm-smi")
        .args(["--showmeminfo", "vram"])
        .output()
        .ok()?;

    let stdout = String::from_utf8(output.stdout).ok()?;
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
        let available = total.saturating_sub(used);
        debug!(
            "rocm-smi: total={}, used={}, available={}",
            total, used, available
        );
        Some(available)
    } else {
        None
    }
}

/// Detect available VRAM via nvidia-smi (NVIDIA).
///
/// Uses a single nvidia-smi call to query both total and free memory,
/// avoiding a TOCTOU race between two separate calls.
fn detect_vram_nvidia_smi() -> Option<u64> {
    let output = std::process::Command::new("nvidia-smi")
        .args([
            "--query-gpu=memory.total,memory.free",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;

    let stdout = String::from_utf8(output.stdout).ok()?;

    // First line = first GPU. Format: "total_mib, free_mib"
    let line = stdout.lines().next()?.trim();
    let mut parts = line.split(',');
    let total_mib: u64 = parts.next()?.trim().parse().ok()?;
    let free_mib: u64 = parts.next()?.trim().parse().ok()?;

    let available = free_mib * 1024 * 1024;

    debug!("nvidia-smi: total={}MiB, free={}MiB", total_mib, free_mib);

    Some(available)
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

fn transcribe_stream_chunk(
    samples: &[f32],
    config: &Config,
    context_cache: &mut ContextCache,
) -> Result<String> {
    if samples.is_empty() {
        return Ok(String::new());
    }

    let available_vram = get_available_vram();
    if samples.len() < 16_000 {
        return Ok(String::new());
    }

    let use_gpu = context_cache.contains(&config.model_path, true)
        || can_run_model_on_gpu(&config.model_path, available_vram);
    let model_path = if use_gpu {
        &config.model_path
    } else {
        config
            .fallback_model_path
            .as_ref()
            .filter(|path| path.exists())
            .unwrap_or(&config.model_path)
    };
    let ctx = context_cache.get_or_create(model_path, use_gpu)?;
    transcribe_samples(
        ctx,
        samples,
        config,
        (num_cpus::get() / 2).max(1) as i32,
        true,
    )
}

fn can_run_model_on_gpu(model_path: &Path, available_vram: u64) -> bool {
    if available_vram < MIN_VRAM_BYTES {
        return false;
    }

    let model_bytes = std::fs::metadata(model_path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    let required_vram = model_bytes.saturating_add(GPU_RESERVED_HEADROOM_BYTES);

    if available_vram < required_vram {
        warn!(
            "Keeping model {:?} off GPU: need {:.2} GB free including reserve, have {:.2} GB",
            model_path,
            required_vram as f64 / 1024.0 / 1024.0 / 1024.0,
            available_vram as f64 / 1024.0 / 1024.0 / 1024.0
        );
        return false;
    }

    true
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

    transcribe_samples(ctx, &samples, config, num_cpus::get() as i32, false)
}

fn transcribe_samples(
    ctx: &WhisperContext,
    samples: &[f32],
    config: &Config,
    n_threads: i32,
    single_segment: bool,
) -> Result<String> {
    // Create whisper state
    let mut state = ctx.create_state()?;

    // Configure transcription parameters
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });

    // Performance settings
    params.set_n_threads(n_threads);

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
    params.set_single_segment(single_segment);

    // Run transcription
    state.full(params, samples)?;

    // Collect results
    let num_segments = state.full_n_segments()?;
    let mut result = String::new();

    for i in 0..num_segments {
        let segment = state.full_get_segment_text(i)?;
        result.push_str(&segment);
        result.push(' ');
    }

    Ok(result.trim().to_string())
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
            config
                .model_path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Invalid model path"))?,
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
        transcribe_samples(
            &self.ctx,
            samples,
            &self.config,
            (num_cpus::get() / 2).max(1) as i32,
            true,
        )
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
    fn test_append_stream_text_deduplicates_overlap() {
        let mut text = "hello world".to_string();
        append_stream_text(&mut text, "world again");
        assert_eq!(text, "hello world again");
    }

    #[test]
    fn test_vram_detection() {
        let vram = get_available_vram();
        println!("Detected available VRAM: {} bytes", vram);
        // Just ensure it doesn't panic
    }

    #[test]
    fn test_validate_remote_endpoint_https_required() {
        assert!(
            validate_remote_endpoint("https://example.com/v1/audio/transcriptions", false).is_ok()
        );
        assert!(
            validate_remote_endpoint("http://example.com/v1/audio/transcriptions", false).is_err()
        );
        assert!(
            validate_remote_endpoint("http://example.com/v1/audio/transcriptions", true).is_ok()
        );
    }
}
