//! Global hotkey integration.

use crate::{AppEvent, Config};
use anyhow::Result;
use crossbeam_channel::{Receiver, Sender};
use std::sync::Arc;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::listen_for_hotkey;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use macos::listen_for_hotkey;

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn listen_for_hotkey(
    _event_tx: Sender<AppEvent>,
    _config_rx: Receiver<Arc<Config>>,
    _initial_config: Arc<Config>,
) -> Result<()> {
    anyhow::bail!("Global hotkeys are not implemented on this OS yet")
}
