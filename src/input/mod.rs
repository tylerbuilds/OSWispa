//! Input simulation and clipboard management.

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use macos::*;

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
use anyhow::Result;

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn copy_to_clipboard(_text: &str) -> Result<()> {
    anyhow::bail!("Clipboard integration is not implemented on this OS yet")
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn copy_to_clipboard_verified(_text: &str) -> Result<()> {
    anyhow::bail!("Clipboard integration is not implemented on this OS yet")
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn paste_text(_text: &str) -> Result<()> {
    anyhow::bail!("Text insertion is not implemented on this OS yet")
}
