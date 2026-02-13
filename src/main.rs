mod audio;
mod feedback;
mod hotkey;
mod input;
mod models;
mod punctuation;
mod settings;
mod transcribe;
mod tray;

use anyhow::Result;
use crossbeam_channel::{bounded, Receiver, Sender};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

/// Application state shared across components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardEntry {
    pub text: String,
    pub timestamp: chrono::DateTime<chrono::Local>,
}

#[derive(Debug, Default)]
pub struct AppState {
    pub is_recording: bool,
    pub clipboard_history: Vec<ClipboardEntry>,
}

/// Events flowing through the application
#[derive(Debug, Clone)]
pub enum AppEvent {
    StartRecording,
    StopRecording,
    CancelRecording,
    /// VAD detected silence - auto-stop
    VadSilenceDetected,
    TranscriptionComplete(String),
    /// Streaming partial result
    StreamingPartial(String),
    Error(String),
    ShowHistory,
    OpenSettings,
    /// Reload configuration from disk
    ReloadConfig,
    Quit,
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

fn format_hotkey(hotkey: &HotkeyConfig) -> String {
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

pub fn get_socket_path() -> PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        return PathBuf::from(runtime_dir).join("oswispa.sock");
    }

    let uid = unsafe { libc::geteuid() };
    PathBuf::from(format!("/tmp/oswispa-{}.sock", uid))
}

fn load_config() -> Config {
    let config_path = get_config_dir().join("config.json");
    if config_path.exists() {
        fs::read_to_string(&config_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        let config = Config::default();
        let _ = fs::create_dir_all(get_config_dir());
        let _ = fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap());
        config
    }
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
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, token)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    }

    Ok(())
}

/// Load remote API key from local secure file.
pub fn get_remote_api_key() -> Option<String> {
    let path = remote_api_key_path();
    std::fs::read_to_string(path)
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

fn load_history() -> Vec<ClipboardEntry> {
    let history_path = get_data_dir().join("history.json");
    if history_path.exists() {
        fs::read_to_string(&history_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        Vec::new()
    }
}

fn save_history(history: &[ClipboardEntry]) {
    let history_path = get_data_dir().join("history.json");
    let _ = fs::create_dir_all(get_data_dir());
    let _ = fs::write(&history_path, serde_json::to_string_pretty(history).unwrap());
}

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("oswispa=info".parse()?))
        .init();

    info!("Starting OSWispa - Voice to Text");

    // Load configuration and history
    let config = Arc::new(RwLock::new(load_config()));
    let history = load_history();
    let state = Arc::new(Mutex::new(AppState {
        is_recording: false,
        clipboard_history: history,
    }));

    let initial_config = config.read().unwrap().clone();

    info!(
        "Language: {}, Audio feedback: {}",
        initial_config.language, initial_config.audio_feedback
    );
    info!(
        "Hotkey: {}",
        format_hotkey(&initial_config.hotkey)
    );
    info!("Backend: {:?}", initial_config.backend);
    if initial_config.vad.enabled {
        info!(
            "VAD enabled: threshold={}, silence={}ms",
            initial_config.vad.threshold, initial_config.vad.silence_duration_ms
        );
    }
    if initial_config.streaming.enabled {
        info!(
            "Streaming mode enabled: chunk={}ms",
            initial_config.streaming.chunk_duration_ms
        );
    }

    // Verify model exists when local backend is required at startup.
    if !initial_config.model_path.exists() {
        if initial_config.backend == TranscriptionBackend::Local {
            error!(
                "Whisper model not found at {:?}. Run the install script first.",
                initial_config.model_path
            );
            eprintln!(
                "\n[ERROR] Whisper model not found!\n\
                Please run: ./install.sh\n\
                Or manually download a model to {:?}\n",
                initial_config.model_path
            );
            std::process::exit(1);
        } else {
            warn!(
                "Local model {:?} not found. Remote backend is enabled; local fallback will be unavailable.",
                initial_config.model_path
            );
        }
    }

    // Create communication channels
    let (event_tx, event_rx): (Sender<AppEvent>, Receiver<AppEvent>) = bounded(100);
    let (audio_tx, audio_rx): (Sender<Option<PathBuf>>, Receiver<Option<PathBuf>>) = bounded(1);
    let (record_tx, record_rx): (Sender<RecordCommand>, Receiver<RecordCommand>) = bounded(10);

    // Clone handles for threads
    let config_for_main = Arc::clone(&config);
    let config_for_transcribe = Arc::clone(&config);
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
        if let Err(e) =
            hotkey::listen_for_hotkey(event_tx_hotkey, hotkey_config_rx, config_for_initial_hotkey)
        {
            error!("Hotkey listener error: {}", e);
            eprintln!(
                "\n[ERROR] Hotkey listener failed: {}\n\
                Make sure you're in the 'input' group:\n\
                sudo usermod -aG input $USER\n\
                Then log out and back in.\n",
                e
            );
        }
    });

    // Start audio recording thread
    std::thread::spawn(move || {
        audio::audio_worker(record_rx, audio_tx, event_tx_audio);
    });

    // Start transcription thread
    std::thread::spawn(move || {
        transcribe::transcription_worker(audio_rx, event_tx_transcribe, config_for_transcribe);
    });

    // Start system tray in separate thread
    std::thread::spawn(move || {
        if let Err(e) = tray::run_tray(event_tx_tray, state_for_tray) {
            error!("System tray error: {}", e);
        }
    });

    // Start Unix socket listener for IPC (GNOME shortcut integration)
    let event_tx_socket = event_tx.clone();
    let state_for_socket = Arc::clone(&state);
    std::thread::spawn(move || {
        use std::io::Read;
        use std::os::unix::fs::PermissionsExt;
        use std::os::unix::net::UnixListener;
        let socket_path = get_socket_path();
        let socket_path_display = socket_path.display().to_string();
        
        // Remove old socket if exists
        let _ = std::fs::remove_file(&socket_path);
        
        if let Some(parent) = socket_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        match UnixListener::bind(&socket_path) {
            Ok(listener) => {
                let _ = std::fs::set_permissions(
                    &socket_path,
                    std::fs::Permissions::from_mode(0o600),
                );
                info!("Unix socket listener started at {}", socket_path_display);
                info!(
                    "To toggle recording, run: printf toggle | nc -U {}",
                    socket_path_display
                );
                
                for stream in listener.incoming() {
                    match stream {
                        Ok(mut stream) => {
                            let mut buf = String::new();
                            if stream.read_to_string(&mut buf).is_ok() {
                                let cmd = buf.trim().to_ascii_lowercase();
                                match cmd.as_str() {
                                    "toggle" => {
                                        let state = state_for_socket.lock().unwrap();
                                        let is_recording = state.is_recording;
                                        drop(state);

                                        if is_recording {
                                            let _ = event_tx_socket.send(AppEvent::StopRecording);
                                        } else {
                                            let _ = event_tx_socket.send(AppEvent::StartRecording);
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
                                        warn!("Ignoring unknown socket command: '{}'", cmd);
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

    info!("All workers started.");
    info!(
        "Hotkey: {}",
        format_hotkey(&initial_config.hotkey)
    );

    // Main event loop
    for event in event_rx {
        match event {
            AppEvent::StartRecording => {
                info!("Starting recording...");

                // Note: Audio feedback disabled - cpal may hang on some systems
                // if config_for_main.audio_feedback {
                //     feedback::play_start_sequence();
                // }

                let mut state = state_for_main.lock().unwrap();
                state.is_recording = true;
                drop(state);
                
                if let Err(e) = record_tx.send(RecordCommand::Start) {
                    error!("Failed to send start command: {}", e);
                }
            }
            AppEvent::StopRecording | AppEvent::VadSilenceDetected => {
                let is_vad = matches!(event, AppEvent::VadSilenceDetected);
                if is_vad {
                    info!("VAD detected silence - auto-stopping...");
                } else {
                    info!("Stopping recording...");
                }

                if config_for_main.read().unwrap().audio_feedback {
                    feedback::play_stop_sequence();
                }

                let mut state = state_for_main.lock().unwrap();
                state.is_recording = false;
                drop(state);
                let _ = record_tx.send(RecordCommand::Stop);
            }
            AppEvent::CancelRecording => {
                info!("Cancelling recording...");

                if config_for_main.read().unwrap().audio_feedback {
                    feedback::play_cancel_sound();
                }

                let mut state = state_for_main.lock().unwrap();
                state.is_recording = false;
                drop(state);
                let _ = record_tx.send(RecordCommand::Cancel);
            }
            AppEvent::StreamingPartial(text) => {
                // For streaming mode - show partial results
                info!("Streaming partial: {}", text);
                // Could update a live display here
            }
            AppEvent::TranscriptionComplete(text) => {
                info!("Transcription complete: {} chars", text.len());
                let current_config = config_for_main.read().unwrap().clone();

                // Apply punctuation commands if enabled
                let text = if current_config.punctuation_commands {
                    punctuation::apply_punctuation_commands(&text)
                } else {
                    text
                };

                if current_config.audio_feedback {
                    feedback::play_complete_sound();
                }

                if let Err(e) = input::copy_to_clipboard(&text) {
                    warn!("Failed to copy to clipboard: {}", e);
                }

                if current_config.auto_paste {
                    std::thread::sleep(std::time::Duration::from_millis(150));
                    if let Err(e) = input::paste_text(&text) {
                        warn!("Failed to paste text: {}", e);
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
                    save_history(&state.clipboard_history);
                }

                if current_config.notification_enabled {
                    let preview = if text.chars().count() > 50 {
                        format!("{}...", text.chars().take(50).collect::<String>())
                    } else {
                        text.clone()
                    };
                    let _ = notify_rust::Notification::new()
                        .summary("OSWispa")
                        .body(&format!("Transcribed: {}", preview))
                        .timeout(3000)
                        .show();
                }
            }
            AppEvent::Error(msg) => {
                error!("Error: {}", msg);

                if config_for_main.read().unwrap().audio_feedback {
                    feedback::play_error_sound();
                }

                let _ = notify_rust::Notification::new()
                    .summary("OSWispa Error")
                    .body(&msg)
                    .timeout(5000)
                    .show();
            }
            AppEvent::ShowHistory => {
                info!("Show history requested");
            }
            AppEvent::OpenSettings => {
                info!("Opening settings dialog...");
                #[cfg(feature = "gui")]
                settings::show_settings_dialog(&config_for_main, event_tx.clone());
                #[cfg(not(feature = "gui"))]
                warn!("Settings dialog requires the 'gui' feature");
            }
            AppEvent::ReloadConfig => {
                info!("Reloading configuration...");
                let new_config = load_config();
                {
                    let mut config_guard = config_for_main.write().unwrap();
                    *config_guard = new_config.clone();
                }
                let _ = hotkey_config_tx.send(Arc::new(new_config));
            }
            AppEvent::Quit => {
                info!("Shutting down...");
                break;
            }
        }
    }

    Ok(())
}

/// Commands sent to the audio recording worker
#[derive(Debug, Clone, Copy)]
pub enum RecordCommand {
    Start,
    Stop,
    Cancel,
}
