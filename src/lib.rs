//! Reusable OSWispa dictation engine and compatibility CLI entry points.

mod audio;
pub mod engine;
mod feedback;
mod gpu;
mod hotkey;
mod input;
mod models;
mod persistence;
pub mod personalisation;
mod punctuation;
mod runtime;
mod settings;
mod setup;
pub mod state;
mod transcribe;
mod tray;

use anyhow::Result;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

pub use engine::{EngineCommand, EngineEvent, EngineHandle, EngineOptions, EnginePhase};
pub use runtime::{
    clear_remote_api_key, get_config_dir, get_data_dir, get_remote_api_key, get_socket_path,
    save_config, set_remote_api_key, Config, HotkeyConfig, RemoteBackendConfig, StreamingConfig,
    TranscriptionBackend, VadConfig,
};
pub(crate) use runtime::{format_hotkey, AppEvent, AppState, RecordCommand, StreamingAudioMessage};
pub use state::{AppPhase, DeliveryOutcome};

/// Run the existing command-line application contract.
///
/// Desktop shells should start [`EngineHandle`] directly instead of invoking
/// this compatibility entry point.
pub fn run_cli() -> Result<()> {
    if let Err(error) = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("oswispa=info".parse()?))
        .try_init()
    {
        warn!("Logging subscriber was already initialised: {}", error);
    }

    info!("Starting OSWispa - Voice to Text");

    if std::env::args_os()
        .skip(1)
        .any(|argument| argument == "--platform-smoke-test")
    {
        return runtime::run_platform_smoke_test();
    }

    EngineHandle::start(EngineOptions::default())?.wait()
}
