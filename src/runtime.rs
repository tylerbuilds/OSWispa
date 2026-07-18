use anyhow::{Context, Result};
use crossbeam_channel::{bounded, select, Receiver, Sender};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use tracing::{error, info, warn};

use crate::engine::{EngineCommand, EngineEvent, EngineOptions, EnginePhase};
#[cfg(feature = "gui")]
use crate::settings;
use crate::state::{reduce_phase, AppPhase, DeliveryOutcome, LifecycleEvent};
use crate::{
    audio, feedback, hotkey, input, models, persistence, personalisation, punctuation, setup,
    transcribe, tray,
};

/// Application state shared across components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardEntry {
    pub text: String,
    pub timestamp: chrono::DateTime<chrono::Local>,
}

#[derive(Debug)]
pub struct AppState {
    pub phase: AppPhase,
    pub clipboard_history: Vec<ClipboardEntry>,
}

impl AppState {
    fn apply_lifecycle(&mut self, event: LifecycleEvent) -> bool {
        let next = reduce_phase(&self.phase, event);
        let changed = next != self.phase;
        self.phase = next;
        changed
    }
}

fn apply_lifecycle_and_publish(
    state: &Arc<Mutex<AppState>>,
    event: LifecycleEvent,
    lifecycle_tx: &Sender<EngineEvent>,
) -> bool {
    let mut state = state.lock().unwrap();
    let changed = state.apply_lifecycle(event);
    let phase = changed.then(|| EnginePhase::from(&state.phase));
    drop(state);

    if let Some(phase) = phase {
        let _ = lifecycle_tx.send(EngineEvent::PhaseChanged(phase));
    }

    changed
}

fn app_event_for_command(command: EngineCommand) -> AppEvent {
    match command {
        EngineCommand::Start => AppEvent::StartRecording,
        EngineCommand::Stop => AppEvent::StopRecording,
        EngineCommand::Cancel => AppEvent::CancelRecording,
        EngineCommand::Reload => AppEvent::ReloadConfig,
        EngineCommand::Shutdown => AppEvent::Quit,
    }
}

/// Events flowing through the application
#[derive(Debug, Clone)]
pub enum AppEvent {
    StartRecording,
    /// The platform audio backend has confirmed that capture is live.
    CaptureStarted {
        device_name: String,
    },
    StopRecording,
    CancelRecording,
    /// VAD detected silence - auto-stop.
    ///
    /// Retained as part of the internal backend event contract; current
    /// recorders do not emit it until VAD capture is wired on every platform.
    #[allow(dead_code)]
    VadSilenceDetected,
    TranscriptionComplete(String),
    /// Streaming partial result
    StreamingPartial(String),
    Error(String),
    OpenSettings,
    /// Reload configuration from disk
    ReloadConfig,
    Quit,
}

/// Streaming audio messages flowing from the recorder to the transcription worker.
#[derive(Debug)]
pub enum StreamingAudioMessage {
    Begin,
    Chunk(Vec<f32>),
    Finalize,
    Cancel,
}

/// Hotkey configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeyConfig {
    /// Use Ctrl modifier
    #[serde(default = "default_true")]
    pub ctrl: bool,
    /// Use Alt modifier
    #[serde(default)]
    pub alt: bool,
    /// Use Shift modifier
    #[serde(default)]
    pub shift: bool,
    /// Use Super/Meta modifier (Windows key)
    #[serde(default = "default_true")]
    pub super_key: bool,
    /// Optional trigger key used with modifiers (for example: "space", "f8", "r")
    #[serde(default)]
    pub trigger_key: Option<String>,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            ctrl: true,
            alt: false,
            shift: false,
            super_key: true,
            trigger_key: None,
        }
    }
}

/// Transcription backend selection
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TranscriptionBackend {
    Local,
    Remote,
}

fn default_backend() -> TranscriptionBackend {
    TranscriptionBackend::Local
}

/// Remote backend configuration (VPS/hosted endpoint)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteBackendConfig {
    /// Endpoint URL for transcription requests
    #[serde(default)]
    pub endpoint: String,
    /// Remote model identifier (for OpenAI-compatible endpoints)
    #[serde(default = "default_remote_model")]
    pub model: String,
    /// HTTP request timeout in milliseconds
    #[serde(default = "default_remote_timeout_ms")]
    pub timeout_ms: u64,
    /// Allow plain HTTP instead of HTTPS (not recommended)
    #[serde(default)]
    pub allow_insecure_http: bool,
    /// Optional environment variable name to read API key from
    #[serde(default)]
    pub api_key_env: Option<String>,
}

fn default_remote_model() -> String {
    "whisper-1".to_string()
}

fn default_remote_timeout_ms() -> u64 {
    20_000
}

impl Default for RemoteBackendConfig {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            model: default_remote_model(),
            timeout_ms: default_remote_timeout_ms(),
            allow_insecure_http: false,
            api_key_env: None,
        }
    }
}

/// Voice Activity Detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VadConfig {
    /// Enable VAD auto-stop
    #[serde(default)]
    pub enabled: bool,
    /// Silence threshold (0.0 - 1.0, lower = more sensitive)
    #[serde(default = "default_vad_threshold")]
    pub threshold: f32,
    /// Silence duration in ms before auto-stop
    #[serde(default = "default_vad_silence_ms")]
    pub silence_duration_ms: u32,
    /// Minimum recording time in ms before VAD can trigger
    #[serde(default = "default_vad_min_recording_ms")]
    pub min_recording_ms: u32,
}

fn default_vad_threshold() -> f32 {
    0.01
}

fn default_vad_silence_ms() -> u32 {
    1500
}

fn default_vad_min_recording_ms() -> u32 {
    500
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold: 0.01,
            silence_duration_ms: 1500,
            min_recording_ms: 500,
        }
    }
}

/// Streaming transcription configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    /// Enable streaming/real-time transcription
    #[serde(default)]
    pub enabled: bool,
    /// Chunk duration in ms for streaming
    #[serde(default = "default_chunk_ms")]
    pub chunk_duration_ms: u32,
}

fn default_chunk_ms() -> u32 {
    3000
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            chunk_duration_ms: 3000,
        }
    }
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub model_path: PathBuf,
    /// Fallback model path for VRAM-constrained situations (smaller/faster model)
    #[serde(default)]
    pub fallback_model_path: Option<PathBuf>,
    pub max_history: usize,
    pub auto_paste: bool,
    pub notification_enabled: bool,
    /// Audio feedback enabled
    #[serde(default = "default_true")]
    pub audio_feedback: bool,
    /// Optional PulseAudio/PipeWire source name on Linux.
    /// When unset, OSWispa follows the system default input source.
    #[serde(default)]
    pub audio_source: Option<String>,
    /// Language for transcription (e.g., "en", "es", "de", "fr", "auto")
    #[serde(default = "default_language")]
    pub language: String,
    /// Translate to English (if language is not English)
    #[serde(default)]
    pub translate_to_english: bool,
    /// Hotkey configuration
    #[serde(default)]
    pub hotkey: HotkeyConfig,
    /// Voice Activity Detection configuration
    #[serde(default)]
    pub vad: VadConfig,
    /// Streaming transcription configuration
    #[serde(default)]
    pub streaming: StreamingConfig,
    /// Enable punctuation commands (say "period" for ".")
    #[serde(default = "default_true")]
    pub punctuation_commands: bool,
    /// Active transcription backend
    #[serde(default = "default_backend")]
    pub backend: TranscriptionBackend,
    /// Remote backend settings for VPS/hosted inference
    #[serde(default)]
    pub remote_backend: RemoteBackendConfig,
}

fn default_true() -> bool {
    true
}

fn default_language() -> String {
    "en".to_string()
}

impl Default for Config {
    fn default() -> Self {
        let data_dir = get_data_dir();
        Self {
            model_path: data_dir.join("models").join("ggml-base.en.bin"),
            fallback_model_path: None,
            max_history: 50,
            auto_paste: true,
            notification_enabled: true,
            audio_feedback: true,
            audio_source: None,
            language: "en".to_string(),
            translate_to_english: false,
            hotkey: HotkeyConfig::default(),
            vad: VadConfig::default(),
            streaming: StreamingConfig::default(),
            punctuation_commands: true,
            backend: TranscriptionBackend::Local,
            remote_backend: RemoteBackendConfig::default(),
        }
    }
}

pub(crate) fn format_hotkey(hotkey: &HotkeyConfig) -> String {
    let mut parts = Vec::new();
    if hotkey.ctrl {
        parts.push("Ctrl");
    }
    if hotkey.alt {
        parts.push("Alt");
    }
    if hotkey.shift {
        parts.push("Shift");
    }
    if hotkey.super_key {
        parts.push("Super");
    }
    if let Some(trigger) = hotkey
        .trigger_key
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        parts.push(trigger);
    }

    if parts.is_empty() {
        "None".to_string()
    } else {
        parts.join("+")
    }
}

pub fn get_data_dir() -> PathBuf {
    ProjectDirs::from("com", "oswispa", "OSWispa")
        .map(|p| p.data_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".oswispa"))
}

pub fn get_config_dir() -> PathBuf {
    ProjectDirs::from("com", "oswispa", "OSWispa")
        .map(|p| p.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".oswispa"))
}

#[cfg(unix)]
pub fn get_socket_path() -> PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        return PathBuf::from(runtime_dir).join("oswispa.sock");
    }

    let uid = unsafe { libc::geteuid() };
    PathBuf::from(format!("/tmp/oswispa-{}", uid)).join("oswispa.sock")
}

#[cfg(not(unix))]
pub fn get_socket_path() -> PathBuf {
    // Placeholder for future platform-native IPC transports.
    std::env::temp_dir().join("oswispa.sock")
}

fn load_config() -> Result<Config> {
    let config_path = get_config_dir().join("config.json");
    if config_path.exists() {
        persistence::read_json_private(&config_path).with_context(|| {
            format!(
                "Configuration is invalid; fix or move {:?} before restarting OSWispa",
                config_path
            )
        })
    } else {
        let config = Config::default();
        save_config(&config)?;
        Ok(config)
    }
}

pub fn save_config(config: &Config) -> Result<()> {
    persistence::write_json_private(&get_config_dir().join("config.json"), config)
}

fn remote_api_key_path() -> PathBuf {
    get_config_dir().join("secrets").join("remote_api_key")
}

/// Persist remote API key to a local 0600 file.
pub fn set_remote_api_key(token: &str) -> Result<()> {
    let token = token.trim();
    if token.is_empty() {
        return clear_remote_api_key();
    }

    let path = remote_api_key_path();
    if let Some(parent) = path.parent() {
        persistence::ensure_private_dir(parent)?;
    }
    persistence::write_private(&path, token.as_bytes())?;

    Ok(())
}

/// Load remote API key from local secure file.
pub fn get_remote_api_key() -> Option<String> {
    let path = remote_api_key_path();
    persistence::read_private_string(&path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Remove remote API key from secure file fallback.
pub fn clear_remote_api_key() -> Result<()> {
    let path = remote_api_key_path();
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

fn load_history() -> Result<Vec<ClipboardEntry>> {
    let history_path = get_data_dir().join("history.json");
    if history_path.exists() {
        persistence::read_json_private(&history_path)
    } else {
        Ok(Vec::new())
    }
}

fn save_history(history: &[ClipboardEntry]) -> Result<()> {
    let history_path = get_data_dir().join("history.json");
    persistence::write_json_private(&history_path, history)
}

fn post_process_transcript(
    text: &str,
    config: &Config,
    personalisation: &personalisation::Personalisation,
) -> String {
    let text = personalisation.apply_dictionary(text);
    if config.punctuation_commands {
        punctuation::apply_punctuation_commands(&text)
    } else {
        text
    }
}

pub(crate) fn run_platform_smoke_test() -> Result<()> {
    for (component, backend) in [
        ("audio", audio::backend_name()),
        ("hotkey", hotkey::backend_name()),
        ("input", input::backend_name()),
    ] {
        if backend == "unsupported" {
            anyhow::bail!("{} has no runtime backend on this platform", component);
        }
    }

    let recording = audio::private_recording_temp_path()?;
    let path = recording.to_path_buf();
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(&path, spec)?;
    for sample in [0_i16, 1_024, -1_024, 0] {
        writer.write_sample(sample)?;
    }
    writer.finalize()?;
    let reader = hound::WavReader::open(&path)?;
    let actual_spec = reader.spec();
    if actual_spec.channels != 1
        || actual_spec.sample_rate != 16_000
        || actual_spec.bits_per_sample != 16
        || reader.len() != 4
    {
        anyhow::bail!("Synthetic recording did not satisfy the transcription WAV contract");
    }

    let marker = format!(
        "OSWispa {} {} VM clipboard smoke",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS
    );
    input::copy_to_clipboard_verified(&marker)?;

    println!(
        "OSWISPA_PLATFORM_SMOKE_OK version={} os={} arch={} audio={} hotkey={} input={}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH,
        audio::backend_name(),
        hotkey::backend_name(),
        input::backend_name()
    );
    Ok(())
}

pub(crate) fn run_engine(
    options: EngineOptions,
    command_rx: Receiver<EngineCommand>,
    lifecycle_tx: Sender<EngineEvent>,
) -> Result<()> {
    let _ = lifecycle_tx.send(EngineEvent::PhaseChanged(EnginePhase::Booting));

    // Load configuration, local personalisation, and history.
    let config = Arc::new(RwLock::new(load_config()?));
    let personalisation = Arc::new(RwLock::new(
        personalisation::load_personalisation().unwrap_or_else(|err| {
            warn!(
                "Could not load personalisation; dictionary is disabled and the existing file is preserved: {}",
                err
            );
            personalisation::Personalisation::default()
        }),
    ));
    let history = load_history().unwrap_or_else(|err| {
        warn!(
            "Could not load clipboard history; starting with an empty history: {}",
            err
        );
        Vec::new()
    });
    let state = Arc::new(Mutex::new(AppState {
        phase: AppPhase::Booting,
        clipboard_history: history,
    }));

    let initial_config = config.read().unwrap().clone();

    info!(
        "Language: {}, Audio feedback: {}",
        initial_config.language, initial_config.audio_feedback
    );
    info!("Hotkey: {}", format_hotkey(&initial_config.hotkey));
    info!("Backend: {:?}", initial_config.backend);
    if initial_config.vad.enabled {
        info!(
            "VAD enabled: threshold={}, silence={}ms",
            initial_config.vad.threshold, initial_config.vad.silence_duration_ms
        );
    }
    if initial_config.backend == TranscriptionBackend::Local && initial_config.streaming.enabled {
        info!(
            "Live local streaming active: chunk={}ms",
            initial_config.streaming.chunk_duration_ms.clamp(250, 1000)
        );
    }

    // Verify the configured model rather than trusting an interrupted legacy
    // download merely because a file exists at the expected path.
    let model_validation = models::validate_model_path(&initial_config.model_path);
    if let Err(model_error) = model_validation {
        if initial_config.backend == TranscriptionBackend::Local {
            info!(
                "No usable local model found ({}) — launching first-time setup wizard",
                model_error
            );

            match setup::run_first_time_setup() {
                Ok(model_path) => {
                    // Update the in-memory config with the downloaded model path
                    let mut cfg = config.write().unwrap();
                    cfg.model_path = model_path;

                    // Persist to disk so the wizard doesn't run again
                    save_config(&cfg)?;
                    info!("Config updated with new model path: {:?}", cfg.model_path);
                }
                Err(e) => {
                    error!("Setup wizard failed: {}", e);
                    eprintln!(
                        "\n[ERROR] Setup wizard failed: {}\n\
                        You can manually download a model:\n\
                        1. Visit https://huggingface.co/ggerganov/whisper.cpp/tree/main\n\
                        2. Save a .bin file to {:?}\n",
                        e,
                        models::get_models_dir()
                    );
                    return Err(e).context("First-time setup failed");
                }
            }
        } else {
            warn!(
                "Local model {:?} is unavailable ({}). Remote backend is enabled; local fallback will be unavailable.",
                initial_config.model_path, model_error
            );
        }
    }

    // Create communication channels
    let (event_tx, event_rx): (Sender<AppEvent>, Receiver<AppEvent>) = bounded(100);
    let (audio_tx, audio_rx): (Sender<Option<PathBuf>>, Receiver<Option<PathBuf>>) = bounded(1);
    let (stream_tx, stream_rx): (
        Sender<StreamingAudioMessage>,
        Receiver<StreamingAudioMessage>,
    ) = bounded(32);
    let (record_tx, record_rx): (Sender<RecordCommand>, Receiver<RecordCommand>) = bounded(10);

    // Clone handles for threads
    let config_for_main = Arc::clone(&config);
    let config_for_audio = Arc::clone(&config);
    let config_for_transcribe = Arc::clone(&config);
    let config_for_tray = Arc::clone(&config);
    let personalisation_for_main = Arc::clone(&personalisation);
    let personalisation_for_transcribe = Arc::clone(&personalisation);
    let state_for_main = Arc::clone(&state);
    let state_for_tray = Arc::clone(&state);

    // Channel senders for threads
    let event_tx_hotkey = event_tx.clone();
    let event_tx_audio = event_tx.clone();
    let event_tx_transcribe = event_tx.clone();
    let event_tx_tray = event_tx.clone();

    // Start hotkey listener thread
    let (hotkey_config_tx, hotkey_config_rx) = bounded(1);
    let config_for_initial_hotkey = Arc::new(config.read().unwrap().clone());
    std::thread::spawn(move || {
        let hotkey_error_tx = event_tx_hotkey.clone();
        if let Err(e) =
            hotkey::listen_for_hotkey(event_tx_hotkey, hotkey_config_rx, config_for_initial_hotkey)
        {
            error!("Hotkey listener error: {}", e);
            let _ =
                hotkey_error_tx.send(AppEvent::Error(format!("Global hotkey unavailable: {}", e)));
            #[cfg(target_os = "linux")]
            eprintln!(
                "\n[ERROR] Hotkey listener failed: {}\n\
                Make sure you're in the 'input' group:\n\
                sudo usermod -aG input $USER\n\
                Then log out and back in.\n",
                e
            );
            #[cfg(not(target_os = "linux"))]
            eprintln!("\n[ERROR] Hotkey listener failed: {}\n", e);
        }
    });

    // Start audio recording thread
    #[cfg(target_os = "linux")]
    let audio_worker = std::thread::spawn(move || {
        audio::audio_worker(
            record_rx,
            audio_tx,
            stream_tx,
            event_tx_audio,
            config_for_audio,
        );
    });

    #[cfg(any(target_os = "macos", target_os = "windows"))]
    let audio_worker = std::thread::spawn(move || {
        audio::audio_worker(record_rx, audio_tx, event_tx_audio);
    });

    // Start transcription thread
    let transcription_worker = std::thread::spawn(move || {
        transcribe::transcription_worker(
            audio_rx,
            stream_rx,
            event_tx_transcribe,
            config_for_transcribe,
            personalisation_for_transcribe,
        );
    });

    // Start the compatibility tray unless an embedding shell owns that surface.
    if options.launch_tray {
        std::thread::spawn(move || {
            if let Err(e) = tray::run_tray(event_tx_tray, state_for_tray, config_for_tray) {
                error!("System tray error: {}", e);
            }
        });
    }

    #[cfg(unix)]
    if options.launch_ipc {
        // Start Unix socket listener for IPC (GNOME shortcut integration)
        let event_tx_socket = event_tx.clone();
        let state_for_socket = Arc::clone(&state);
        std::thread::spawn(move || {
            use std::io::Read;
            use std::os::unix::fs::PermissionsExt;
            use std::os::unix::net::UnixListener;
            let socket_path = get_socket_path();
            let socket_path_display = socket_path.display().to_string();

            if let Some(parent) = socket_path.parent() {
                if let Err(error) = persistence::ensure_private_dir(parent) {
                    warn!("Failed to secure Unix socket directory: {}", error);
                    return;
                }
            }

            // Remove an old socket only after its parent has been verified.
            let _ = std::fs::remove_file(&socket_path);

            match UnixListener::bind(&socket_path) {
                Ok(listener) => {
                    if let Err(error) = std::fs::set_permissions(
                        &socket_path,
                        std::fs::Permissions::from_mode(0o600),
                    ) {
                        warn!("Failed to secure Unix socket: {}", error);
                        drop(listener);
                        let _ = std::fs::remove_file(&socket_path);
                        return;
                    }
                    info!("Unix socket listener started at {}", socket_path_display);
                    info!(
                        "To toggle recording, run: printf toggle | nc -U {}",
                        socket_path_display
                    );

                    for stream in listener.incoming() {
                        match stream {
                            Ok(stream) => {
                                let mut buf = String::new();
                                if stream.take(65).read_to_string(&mut buf).is_ok() {
                                    if buf.len() > 64 {
                                        warn!("Ignoring oversized socket command");
                                        continue;
                                    }
                                    let cmd = buf.trim().to_ascii_lowercase();
                                    match cmd.as_str() {
                                        "toggle" => {
                                            let state = state_for_socket.lock().unwrap();
                                            let is_recording = state.phase.is_capturing();
                                            drop(state);

                                            if is_recording {
                                                let _ =
                                                    event_tx_socket.send(AppEvent::StopRecording);
                                            } else {
                                                let _ =
                                                    event_tx_socket.send(AppEvent::StartRecording);
                                            }
                                        }
                                        "start" => {
                                            let _ = event_tx_socket.send(AppEvent::StartRecording);
                                        }
                                        "stop" => {
                                            let _ = event_tx_socket.send(AppEvent::StopRecording);
                                        }
                                        "cancel" => {
                                            let _ = event_tx_socket.send(AppEvent::CancelRecording);
                                        }
                                        _ => {
                                            warn!(
                                                "Ignoring unknown socket command ({} bytes)",
                                                cmd.len()
                                            );
                                        }
                                    }
                                }
                            }
                            Err(e) => warn!("Socket connection failed: {}", e),
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to create Unix socket: {}", e);
                }
            }
        });
    }

    #[cfg(not(unix))]
    if options.launch_ipc {
        warn!("IPC listener is not implemented on this OS yet");
    }

    info!("All workers started.");
    info!("Hotkey: {}", format_hotkey(&initial_config.hotkey));
    apply_lifecycle_and_publish(&state, LifecycleEvent::WorkersReady, &lifecycle_tx);

    // Internal platform events and public controller commands converge here so
    // every caller observes the same reducer-backed lifecycle.
    loop {
        let event = select! {
            recv(event_rx) -> event => match event {
                Ok(event) => event,
                Err(_) => break,
            },
            recv(command_rx) -> command => match command {
                Ok(command) => app_event_for_command(command),
                Err(_) => AppEvent::Quit,
            },
        };

        match event {
            AppEvent::StartRecording => {
                info!("Starting recording...");

                let accepted = apply_lifecycle_and_publish(
                    &state_for_main,
                    LifecycleEvent::StartRequested,
                    &lifecycle_tx,
                );
                if !accepted {
                    info!("Ignoring start request while another dictation is active");
                    continue;
                }

                // Note: Audio feedback disabled - cpal may hang on some systems
                // if config_for_main.audio_feedback {
                //     feedback::play_start_sequence();
                // }

                match record_tx.send(RecordCommand::Start) {
                    Ok(()) => {}
                    Err(e) => {
                        apply_lifecycle_and_publish(
                            &state_for_main,
                            LifecycleEvent::Failed,
                            &lifecycle_tx,
                        );
                        error!("Failed to send start command: {}", e);
                    }
                }
            }
            AppEvent::CaptureStarted { device_name } => {
                info!("Audio capture started");
                apply_lifecycle_and_publish(
                    &state_for_main,
                    LifecycleEvent::CaptureStarted { device_name },
                    &lifecycle_tx,
                );
            }
            AppEvent::StopRecording | AppEvent::VadSilenceDetected => {
                let is_vad = matches!(event, AppEvent::VadSilenceDetected);
                if is_vad {
                    info!("VAD detected silence - auto-stopping...");
                } else {
                    info!("Stopping recording...");
                }

                let accepted = apply_lifecycle_and_publish(
                    &state_for_main,
                    LifecycleEvent::StopRequested,
                    &lifecycle_tx,
                );
                if !accepted {
                    info!("Ignoring stop request because capture is not active");
                    continue;
                }

                if config_for_main.read().unwrap().audio_feedback {
                    feedback::play_stop_sequence();
                }

                if let Err(e) = record_tx.send(RecordCommand::Stop) {
                    apply_lifecycle_and_publish(
                        &state_for_main,
                        LifecycleEvent::Failed,
                        &lifecycle_tx,
                    );
                    error!("Failed to send stop command: {}", e);
                }
            }
            AppEvent::CancelRecording => {
                info!("Cancelling recording...");

                let accepted = apply_lifecycle_and_publish(
                    &state_for_main,
                    LifecycleEvent::CancelRequested,
                    &lifecycle_tx,
                );
                if !accepted {
                    info!("Ignoring cancel request because capture is not active");
                    continue;
                }

                if config_for_main.read().unwrap().audio_feedback {
                    feedback::play_cancel_sound();
                }

                if let Err(e) = record_tx.send(RecordCommand::Cancel) {
                    apply_lifecycle_and_publish(
                        &state_for_main,
                        LifecycleEvent::Failed,
                        &lifecycle_tx,
                    );
                    error!("Failed to send cancel command: {}", e);
                }
            }
            AppEvent::StreamingPartial(text) => {
                // For streaming mode - show partial results
                info!("Streaming partial: {} chars", text.len());
                // Could update a live display here
            }
            AppEvent::TranscriptionComplete(text) => {
                info!("Transcription complete: {} chars", text.len());
                let current_config = config_for_main.read().unwrap().clone();

                apply_lifecycle_and_publish(
                    &state_for_main,
                    LifecycleEvent::TranscriptionReady,
                    &lifecycle_tx,
                );

                // Apply explicit local phrase replacements before spoken punctuation commands.
                let text = personalisation_for_main
                    .read()
                    .map(|dictionary| post_process_transcript(&text, &current_config, &dictionary))
                    .unwrap_or_else(|_| {
                        if current_config.punctuation_commands {
                            punctuation::apply_punctuation_commands(&text)
                        } else {
                            text
                        }
                    });

                let copied = match input::copy_to_clipboard_verified(&text) {
                    Ok(_) => true,
                    Err(e) => {
                        warn!("Failed to copy to clipboard: {}", e);
                        false
                    }
                };

                // Only auto-paste if we successfully updated the clipboard; otherwise we'd risk
                // pasting stale clipboard contents.
                let delivery_outcome = if current_config.auto_paste && copied {
                    match input::paste_text(&text) {
                        Ok(()) => DeliveryOutcome::Inserted,
                        Err(e) => {
                            warn!("Failed to paste text: {}", e);
                            DeliveryOutcome::CopiedOnly
                        }
                    }
                } else if current_config.auto_paste && !copied {
                    warn!("Auto-paste skipped because clipboard copy failed. Paste manually from the app output or retry.");
                    DeliveryOutcome::Failed
                } else if copied {
                    DeliveryOutcome::CopiedOnly
                } else {
                    DeliveryOutcome::Failed
                };

                apply_lifecycle_and_publish(
                    &state_for_main,
                    LifecycleEvent::DeliveryFinished(delivery_outcome),
                    &lifecycle_tx,
                );

                if current_config.audio_feedback {
                    match delivery_outcome {
                        DeliveryOutcome::Inserted | DeliveryOutcome::CopiedOnly => {
                            feedback::play_complete_sound()
                        }
                        DeliveryOutcome::Failed => feedback::play_error_sound(),
                    }
                }

                // Add to history
                {
                    let mut state = state_for_main.lock().unwrap();
                    state.clipboard_history.insert(
                        0,
                        ClipboardEntry {
                            text: text.clone(),
                            timestamp: chrono::Local::now(),
                        },
                    );
                    if state.clipboard_history.len() > current_config.max_history {
                        state.clipboard_history.truncate(current_config.max_history);
                    }
                    if let Err(err) = save_history(&state.clipboard_history) {
                        warn!("Failed to save clipboard history: {}", err);
                    }
                }

                if current_config.notification_enabled {
                    let delivery_status = match delivery_outcome {
                        DeliveryOutcome::Inserted => "Text inserted",
                        DeliveryOutcome::CopiedOnly => "Text copied to clipboard",
                        DeliveryOutcome::Failed => "Text delivery failed",
                    };
                    #[cfg(target_os = "linux")]
                    {
                        let _ = notify_rust::Notification::new()
                            .summary("OSWispa")
                            .body(delivery_status)
                            .timeout(3000)
                            .show();
                    }
                    #[cfg(not(target_os = "linux"))]
                    {
                        info!("Transcription delivery notification: {}", delivery_status);
                    }
                }
            }
            AppEvent::Error(msg) => {
                apply_lifecycle_and_publish(&state_for_main, LifecycleEvent::Failed, &lifecycle_tx);
                error!("Error: {}", msg);

                if config_for_main.read().unwrap().audio_feedback {
                    feedback::play_error_sound();
                }

                #[cfg(target_os = "linux")]
                {
                    let _ = notify_rust::Notification::new()
                        .summary("OSWispa Error")
                        .body(&msg)
                        .timeout(5000)
                        .show();
                }
            }
            AppEvent::OpenSettings => {
                info!("Opening settings dialog...");
                #[cfg(feature = "gui")]
                settings::show_settings_dialog(
                    &config_for_main,
                    &personalisation,
                    event_tx.clone(),
                );
                #[cfg(not(feature = "gui"))]
                warn!("Settings dialog requires the 'gui' feature");
            }
            AppEvent::ReloadConfig => {
                info!("Reloading configuration...");
                match load_config() {
                    Ok(new_config) => {
                        {
                            let mut config_guard = config_for_main.write().unwrap();
                            *config_guard = new_config.clone();
                        }
                        let _ = hotkey_config_tx.send(Arc::new(new_config));
                    }
                    Err(err) => {
                        error!(
                            "Keeping current configuration because reload failed: {}",
                            err
                        );
                    }
                }
            }
            AppEvent::Quit => {
                info!("Shutting down...");
                if state_for_main.lock().unwrap().phase.is_capturing() {
                    apply_lifecycle_and_publish(
                        &state_for_main,
                        LifecycleEvent::CancelRequested,
                        &lifecycle_tx,
                    );
                    let _ = record_tx.send(RecordCommand::Cancel);
                }
                break;
            }
        }
    }

    drop(record_tx);
    audio_worker
        .join()
        .map_err(|_| anyhow::anyhow!("OSWispa audio worker panicked"))?;
    transcription_worker
        .join()
        .map_err(|_| anyhow::anyhow!("OSWispa transcription worker panicked"))?;

    Ok(())
}

/// Commands sent to the audio recording worker
#[derive(Debug, Clone, Copy)]
pub enum RecordCommand {
    Start,
    Stop,
    Cancel,
}

#[cfg(test)]
mod compatibility_tests {
    use super::*;

    #[test]
    fn empty_dictionary_preserves_existing_output() {
        let config = Config {
            punctuation_commands: false,
            ..Config::default()
        };
        assert_eq!(
            post_process_transcript(
                "Testing, testing, one, two, three.",
                &config,
                &personalisation::Personalisation::default(),
            ),
            "Testing, testing, one, two, three."
        );
    }

    #[test]
    fn dictionary_runs_before_spoken_punctuation() {
        let dictionary = personalisation::Personalisation::from_dictionary(vec![
            personalisation::DictionaryEntry {
                spoken: "finish sentence".to_string(),
                written: "period".to_string(),
                enabled: true,
                case_sensitive: false,
            },
        ])
        .unwrap();
        assert_eq!(
            post_process_transcript("hello finish sentence", &Config::default(), &dictionary),
            "hello."
        );
    }

    #[test]
    fn legacy_config_and_history_json_remain_compatible() {
        let config: Config = serde_json::from_str(
            r#"{
                "model_path": "/tmp/model.bin",
                "max_history": 50,
                "auto_paste": true,
                "notification_enabled": true
            }"#,
        )
        .unwrap();
        assert!(config.punctuation_commands);

        let history: Vec<ClipboardEntry> = serde_json::from_str(
            r#"[{"text":"existing transcript","timestamp":"2026-07-18T12:00:00+01:00"}]"#,
        )
        .unwrap();
        assert_eq!(history[0].text, "existing transcript");
    }
}
