//! Input simulation and clipboard management.

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "windows")]
pub use windows::*;

#[cfg(target_os = "linux")]
pub fn backend_name() -> &'static str {
    "linux-native"
}

#[cfg(target_os = "macos")]
pub fn backend_name() -> &'static str {
    "macos-arboard-enigo"
}

#[cfg(target_os = "windows")]
pub fn backend_name() -> &'static str {
    "windows-arboard-enigo"
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
use anyhow::Result;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub fn backend_name() -> &'static str {
    "unsupported"
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub fn copy_to_clipboard(_text: &str) -> Result<()> {
    anyhow::bail!("Clipboard integration is not implemented on this OS yet")
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub fn copy_to_clipboard_verified(_text: &str) -> Result<()> {
    anyhow::bail!("Clipboard integration is not implemented on this OS yet")
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub fn paste_text(_text: &str) -> Result<()> {
    anyhow::bail!("Text insertion is not implemented on this OS yet")
}
