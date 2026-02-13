//! Input simulation and clipboard management for Wayland
//!
//! On Wayland, we use:
//! - wl-clipboard-rs for clipboard operations
//! - ydotool for text input simulation (requires ydotoold daemon)
//!
//! ydotool works by writing to /dev/uinput which requires permissions.

use anyhow::{Context, Result};
use std::io::Write;
use std::process::{Command, Stdio};
use tracing::{debug, info, warn};
use wl_clipboard_rs::copy::{MimeType, Options, Source};

/// Copy text to the Wayland clipboard
pub fn copy_to_clipboard(text: &str) -> Result<()> {
    info!("Copying {} chars to clipboard", text.len());

    let opts = Options::new();
    opts.copy(Source::Bytes(text.as_bytes().into()), MimeType::Text)
        .context("Failed to copy to Wayland clipboard")?;

    debug!("Text copied to clipboard successfully");
    Ok(())
}

/// Paste text by simulating keyboard input
///
/// To avoid leaking dictated text in process arguments, we never pass the
/// transcript to command-line tools. We paste from clipboard via Ctrl+V.
pub fn paste_text(_text: &str) -> Result<()> {
    info!("Pasting clipboard contents via simulated Ctrl+V");

    // First, try to check if ydotoold is running
    let daemon_check = Command::new("pgrep").arg("ydotoold").output();

    match daemon_check {
        Ok(output) if output.status.success() => {
            debug!("ydotoold daemon is running");
        }
        _ => {
            debug!("ydotoold may not be running.");
            // We don't try to spawn it here as the binary is often missing or requires sudo
        }
    }

    if let Err(e) = paste_with_ctrl_v() {
        warn!("ydotool Ctrl+V failed: {}", e);
        info!("Trying wtype Ctrl+V fallback...");
        return paste_with_wtype_ctrl_v();
    }

    debug!("Clipboard pasted successfully via ydotool Ctrl+V");
    Ok(())
}

/// Alternative: use wtype to send Ctrl+V
fn paste_with_wtype_ctrl_v() -> Result<()> {
    let status = Command::new("wtype")
        .args(["-M", "ctrl", "v", "-m", "ctrl"])
        .status()
        .context(
            "Failed to run wtype for Ctrl+V fallback. Install with: sudo apt install wtype\n\
            Note: wtype only works on wlroots-based compositors, not GNOME.",
        )?;

    if !status.success() {
        anyhow::bail!("wtype Ctrl+V fallback failed");
    }

    Ok(())
}

/// Fallback: simulate Ctrl+V keystroke
/// This assumes text is already in clipboard
fn paste_with_ctrl_v() -> Result<()> {
    // Small delay to ensure clipboard is ready
    std::thread::sleep(std::time::Duration::from_millis(50));

    let status = Command::new("ydotool")
        .args(["key", "29:1", "47:1", "47:0", "29:0"]) // Ctrl down, V down, V up, Ctrl up
        .status()
        .context("Failed to simulate Ctrl+V with ydotool")?;

    if !status.success() {
        anyhow::bail!("Failed to paste with Ctrl+V simulation");
    }

    Ok(())
}

/// Alternative clipboard copy using wl-copy command
#[allow(dead_code)]
pub fn copy_to_clipboard_cmd(text: &str) -> Result<()> {
    let mut child = Command::new("wl-copy")
        .stdin(Stdio::piped())
        .spawn()
        .context("Failed to run wl-copy. Install with: sudo apt install wl-clipboard")?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(text.as_bytes())?;
    }

    let status = child.wait()?;
    if !status.success() {
        anyhow::bail!("wl-copy failed");
    }

    Ok(())
}

/// Get text from clipboard
#[allow(dead_code)]
pub fn get_from_clipboard() -> Result<String> {
    use std::io::Read;
    use wl_clipboard_rs::paste::{get_contents, ClipboardType, MimeType, Seat};

    let (mut pipe, _mime_type) =
        get_contents(ClipboardType::Regular, Seat::Unspecified, MimeType::Text)?;

    let mut text = String::new();
    pipe.read_to_string(&mut text)?;
    Ok(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // Requires Wayland session
    fn test_clipboard_roundtrip() {
        let test_text = "Hello from OSWispa test!";
        copy_to_clipboard(test_text).unwrap();
        let retrieved = get_from_clipboard().unwrap();
        assert_eq!(test_text, retrieved);
    }
}
