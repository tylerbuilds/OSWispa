//! Reusable MorpheOS Voice dictation engine and compatibility CLI entry points.

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

/// Public product identity. Installed `oswispa` identifiers remain as compatibility contracts.
pub const PRODUCT_NAME: &str = "MorpheOS Voice";
pub const COMPANY_NAME: &str = "MorpheOS";
pub const CANONICAL_PRODUCT_URL: &str = "https://morpheos.net/voice";
pub const LEGACY_CLI_NAME: &str = "oswispa";

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

    info!("Starting {} - Voice Typing", PRODUCT_NAME);

    if std::env::args_os()
        .skip(1)
        .any(|argument| argument == "--platform-smoke-test")
    {
        return runtime::run_platform_smoke_test();
    }

    EngineHandle::start(EngineOptions::default())?.wait()
}

#[cfg(test)]
mod brand_contract_tests {
    use super::*;

    #[test]
    fn public_identity_is_morpheos_voice() {
        assert_eq!(PRODUCT_NAME, "MorpheOS Voice");
        assert_eq!(COMPANY_NAME, "MorpheOS");
        assert_eq!(CANONICAL_PRODUCT_URL, "https://morpheos.net/voice");
    }

    #[test]
    fn transition_release_retains_legacy_cli_contract() {
        assert_eq!(LEGACY_CLI_NAME, "oswispa");
    }
}
