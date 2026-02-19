//! Input simulation and clipboard management.

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::*;

#[cfg(not(target_os = "linux"))]
use anyhow::Result;

#[cfg(not(target_os = "linux"))]
pub fn copy_to_clipboard(_text: &str) -> Result<()> {
    anyhow::bail!("Clipboard integration is not implemented on this OS yet")
}

#[cfg(not(target_os = "linux"))]
pub fn copy_to_clipboard_verified(_text: &str) -> Result<()> {
    anyhow::bail!("Clipboard integration is not implemented on this OS yet")
}

#[cfg(not(target_os = "linux"))]
pub fn paste_text(_text: &str) -> Result<()> {
    anyhow::bail!("Text insertion is not implemented on this OS yet")
}
