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
use std::sync::{Arc, Mutex};
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
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            ctrl: true,
            alt: false,
            shift: false,
            super_key: true,
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
        }
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
    let config = Arc::new(load_config());
    let history = load_history();
    let state = Arc::new(Mutex::new(AppState {
        is_recording: false,
        clipboard_history: history,
    }));

    info!("Language: {}, Audio feedback: {}", config.language, config.audio_feedback);
    info!(
        "Hotkey: {}{}{}{}",
        if config.hotkey.ctrl { "Ctrl+" } else { "" },
        if config.hotkey.alt { "Alt+" } else { "" },
        if config.hotkey.shift { "Shift+" } else { "" },
        if config.hotkey.super_key { "Super" } else { "" }
    );
    if config.vad.enabled {
        info!(
            "VAD enabled: threshold={}, silence={}ms",
            config.vad.threshold, config.vad.silence_duration_ms
        );
    }
    if config.streaming.enabled {
        info!("Streaming mode enabled: chunk={}ms", config.streaming.chunk_duration_ms);
    }

    // Verify model exists
    if !config.model_path.exists() {
        error!(
            "Whisper model not found at {:?}. Run the install script first.",
            config.model_path
        );
        eprintln!(
            "\n[ERROR] Whisper model not found!\n\
            Please run: ./install.sh\n\
            Or manually download a model to {:?}\n",
            config.model_path
        );
        std::process::exit(1);
    }

    // Create communication channels
    let (event_tx, event_rx): (Sender<AppEvent>, Receiver<AppEvent>) = bounded(100);
    let (audio_tx, audio_rx): (Sender<Option<PathBuf>>, Receiver<Option<PathBuf>>) = bounded(1);
    let (record_tx, record_rx): (Sender<RecordCommand>, Receiver<RecordCommand>) = bounded(10);

    // Clone handles for threads
    let config_for_main = Arc::clone(&config);
    let config_for_transcribe = Arc::clone(&config);
    let config_for_hotkey = Arc::clone(&config);
    let config_for_audio = Arc::clone(&config);
    let state_for_main = Arc::clone(&state);
    let state_for_tray = Arc::clone(&state);

    // Channel senders for threads
    let event_tx_hotkey = event_tx.clone();
    let event_tx_audio = event_tx.clone();
    let event_tx_transcribe = event_tx.clone();
    let event_tx_tray = event_tx.clone();

    // Start hotkey listener thread
    let (hotkey_config_tx, hotkey_config_rx) = bounded(1);
    let config_for_initial_hotkey = Arc::clone(&config);
    std::thread::spawn(move || {
        if let Err(e) = hotkey::listen_for_hotkey(event_tx_hotkey, hotkey_config_rx, config_for_initial_hotkey) {
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
        audio::audio_worker(record_rx, audio_tx, event_tx_audio, &config_for_audio);
    });

    // Start transcription thread
    std::thread::spawn(move || {
        transcribe::transcription_worker(audio_rx, event_tx_transcribe, &config_for_transcribe);
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
        use std::os::unix::net::UnixListener;
        use std::io::Read;
        let socket_path = "/tmp/oswispa.sock";
        
        // Remove old socket if exists
        let _ = std::fs::remove_file(socket_path);
        
        match UnixListener::bind(socket_path) {
            Ok(listener) => {
                info!("Unix socket listener started at {}", socket_path);
                info!("To toggle recording, run: echo toggle | nc -U /tmp/oswispa.sock");
                
                for stream in listener.incoming() {
                    match stream {
                        Ok(mut stream) => {
                            let mut buf = String::new();
                            if stream.read_to_string(&mut buf).is_ok() {
                                let state = state_for_socket.lock().unwrap();
                                let is_recording = state.is_recording;
                                drop(state);
                                
                                if is_recording {
                                    let _ = event_tx_socket.send(AppEvent::StopRecording);
                                } else {
                                    let _ = event_tx_socket.send(AppEvent::StartRecording);
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
        "Hotkey: {}{}{}{}",
        if config.hotkey.ctrl { "Ctrl+" } else { "" },
        if config.hotkey.alt { "Alt+" } else { "" },
        if config.hotkey.shift { "Shift+" } else { "" },
        if config.hotkey.super_key { "Super" } else { "" }
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

                if config_for_main.audio_feedback {
                    feedback::play_stop_sequence();
                }

                let mut state = state_for_main.lock().unwrap();
                state.is_recording = false;
                drop(state);
                let _ = record_tx.send(RecordCommand::Stop);
            }
            AppEvent::CancelRecording => {
                info!("Cancelling recording...");

                if config_for_main.audio_feedback {
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

                // Apply punctuation commands if enabled
                let text = if config_for_main.punctuation_commands {
                    punctuation::apply_punctuation_commands(&text)
                } else {
                    text
                };

                if config_for_main.audio_feedback {
                    feedback::play_complete_sound();
                }

                if let Err(e) = input::copy_to_clipboard(&text) {
                    warn!("Failed to copy to clipboard: {}", e);
                }

                if config_for_main.auto_paste {
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
                    if state.clipboard_history.len() > config_for_main.max_history {
                        state.clipboard_history.truncate(config_for_main.max_history);
                    }
                    save_history(&state.clipboard_history);
                }

                if config_for_main.notification_enabled {
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

                if config_for_main.audio_feedback {
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
                let new_config = Arc::new(load_config());
                // We could update config_for_main if it were mutable, but we'll use a local state.
                // However, the Arc<Config> is captured in closures.
                // For now, we'll notify the hotkey listener.
                let _ = hotkey_config_tx.send(new_config);
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
