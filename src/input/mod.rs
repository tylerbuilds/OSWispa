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
    opts.copy(
        Source::Bytes(text.as_bytes().into()),
        MimeType::Text,
    )
    .context("Failed to copy to Wayland clipboard")?;

    debug!("Text copied to clipboard successfully");
    Ok(())
}

/// Paste text by simulating keyboard input
///
/// We use ydotool because it works on Wayland (unlike xdotool).
/// ydotool requires the ydotoold daemon to be running.
pub fn paste_text(text: &str) -> Result<()> {
    info!("Pasting {} chars via ydotool", text.len());

    // First, try to check if ydotoold is running
    let daemon_check = Command::new("pgrep")
        .arg("ydotoold")
        .output();

    match daemon_check {
        Ok(output) if output.status.success() => {
            debug!("ydotoold daemon is running");
        }
        _ => {
            warn!("ydotoold may not be running. Attempting paste anyway...");
            // Try to start it (might fail without sudo)
            let _ = Command::new("ydotoold")
                .spawn();
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }

    // Use ydotool to type the text
    // Works with both v0.1.x and v1.x
    let mut child = Command::new("ydotool")
        .arg("type")
        .arg("--")
        .arg(text)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .context(
            "Failed to spawn ydotool. Is it installed?\n\
            Install with: sudo apt install ydotool"
        )?;

    let status = child.wait()?;

    if !status.success() {
        // Try alternative method: wtype (another Wayland text input tool)
        info!("ydotool failed, trying wtype...");
        return paste_with_wtype(text);
    }

    debug!("Text pasted successfully via ydotool");
    Ok(())
}

/// Alternative: use wtype for Wayland text input
fn paste_with_wtype(text: &str) -> Result<()> {
    let status = Command::new("wtype")
        .arg("--")
        .arg(text)
        .status()
        .context(
            "Failed to run wtype. Install with: sudo apt install wtype\n\
            Note: wtype only works on wlroots-based compositors, not GNOME."
        )?;

    if !status.success() {
        // Last resort: use Ctrl+V paste
        info!("wtype failed, falling back to Ctrl+V paste");
        return paste_with_ctrl_v();
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
    use wl_clipboard_rs::paste::{get_contents, ClipboardType, Seat, MimeType};

    let (mut pipe, _mime_type) = get_contents(ClipboardType::Regular, Seat::Unspecified, MimeType::Text)?;

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
