//! System tray integration.

#[cfg(not(target_os = "linux"))]
use crate::{AppEvent, AppState, Config};
#[cfg(not(target_os = "linux"))]
use anyhow::Result;
#[cfg(not(target_os = "linux"))]
use crossbeam_channel::Sender;
#[cfg(not(target_os = "linux"))]
use std::sync::{Arc, Mutex, RwLock};

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(not(target_os = "linux"))]
pub fn run_tray(
    _event_tx: Sender<AppEvent>,
    _state: Arc<Mutex<AppState>>,
    _config: Arc<RwLock<Config>>,
) -> Result<()> {
    tracing::warn!("System tray is not implemented on this OS yet");
    Ok(())
}
