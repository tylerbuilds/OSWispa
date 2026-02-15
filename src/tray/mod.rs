//! System tray integration.

use crate::{AppEvent, AppState};
use anyhow::Result;
use crossbeam_channel::Sender;
use std::sync::{Arc, Mutex};

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(not(target_os = "linux"))]
pub fn run_tray(_event_tx: Sender<AppEvent>, _state: Arc<Mutex<AppState>>) -> Result<()> {
    tracing::warn!("System tray is not implemented on this OS yet");
    Ok(())
}
