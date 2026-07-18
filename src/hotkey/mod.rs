//! Global hotkey integration.

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
use crate::{AppEvent, Config};
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
use anyhow::Result;
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
use crossbeam_channel::{Receiver, Sender};
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
use std::sync::Arc;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::listen_for_hotkey;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use macos::listen_for_hotkey;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "windows")]
pub use windows::listen_for_hotkey;

#[cfg(target_os = "linux")]
pub fn backend_name() -> &'static str {
    "linux-evdev"
}

#[cfg(target_os = "macos")]
pub fn backend_name() -> &'static str {
    "macos-rdev"
}

#[cfg(target_os = "windows")]
pub fn backend_name() -> &'static str {
    "windows-rdev"
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub fn backend_name() -> &'static str {
    "unsupported"
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub fn listen_for_hotkey(
    _event_tx: Sender<AppEvent>,
    _config_rx: Receiver<Arc<Config>>,
    _initial_config: Arc<Config>,
) -> Result<()> {
    anyhow::bail!("Global hotkeys are not implemented on this OS yet")
}
