//! Input simulation and clipboard management for Linux (Wayland/X11)
//!
//! On Linux we support both Wayland and X11 sessions:
//! - Clipboard:
//!   - Wayland: `wl-clipboard-rs` (with optional `wl-copy` fallback)
//!   - X11: `xclip` (clipboard)
//! - Text insertion:
//!   - X11: `xdotool` Ctrl+V when available
//!   - Wayland/X11: `ydotool` Ctrl+V (requires `ydotoold`)
//!
//! ydotool works by writing to /dev/uinput which requires permissions.

use anyhow::{Context, Result};
use std::io::Write;
use std::process::{Command, Stdio};
use tracing::{debug, info, warn};
use wl_clipboard_rs::copy::{MimeType, Options, Source};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SessionKind {
    Wayland,
    X11,
    Unknown,
}

fn session_kind() -> SessionKind {
    let xdg = std::env::var("XDG_SESSION_TYPE")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();

    if xdg == "wayland" {
        return SessionKind::Wayland;
    }
    if xdg == "x11" {
        return SessionKind::X11;
    }

    // Prefer explicit Wayland indicator. Note: DISPLAY is often set on Wayland due to XWayland.
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
        return SessionKind::Wayland;
    }
    if std::env::var_os("DISPLAY").is_some() {
        return SessionKind::X11;
    }

    SessionKind::Unknown
}

fn command_exists(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| {
            for dir in std::env::split_paths(&paths) {
                let candidate = dir.join(name);
                if candidate.is_file() {
                    return true;
                }
            }
            false
        })
        .unwrap_or(false)
}

fn copy_to_wayland_clipboard_rs(text: &str) -> Result<()> {
    let opts = Options::new();
    opts.copy(Source::Bytes(text.as_bytes().into()), MimeType::Text)
        .context("Failed to copy to Wayland clipboard")?;

    debug!("Text copied to Wayland clipboard");
    Ok(())
}

fn copy_to_x11_clipboard_xclip(text: &str) -> Result<()> {
    if !command_exists("xclip") {
        anyhow::bail!("xclip not found. Install with: sudo apt install xclip");
    }

    let mut child = Command::new("xclip")
        .args(["-selection", "clipboard", "-in"])
        .stdin(Stdio::piped())
        .spawn()
        .context("Failed to run xclip. Install with: sudo apt install xclip")?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin.write_all(text.as_bytes())?;
    }

    let status = child.wait()?;
    if !status.success() {
        anyhow::bail!("xclip failed");
    }

    debug!("Text copied to X11 clipboard via xclip");
    Ok(())
}

/// Copy text to the clipboard.
///
/// On Wayland we use `wl-clipboard-rs` (with a `wl-copy` fallback).
/// On X11 we use `xclip`.
pub fn copy_to_clipboard(text: &str) -> Result<()> {
    info!("Copying {} chars to clipboard", text.len());

    match session_kind() {
        SessionKind::Wayland => copy_to_wayland_clipboard_rs(text).or_else(|e| {
            warn!("Wayland clipboard copy failed (wl-clipboard-rs): {}", e);
            copy_to_clipboard_cmd(text)
        }),
        SessionKind::X11 => copy_to_x11_clipboard_xclip(text),
        SessionKind::Unknown => {
            // Best-effort: try Wayland first, then X11.
            copy_to_wayland_clipboard_rs(text).or_else(|_| copy_to_x11_clipboard_xclip(text))
        }
    }
}

/// Paste text by simulating keyboard input.
///
/// To avoid leaking dictated text in process arguments, we never pass the
/// transcript to command-line tools. We paste from clipboard via Ctrl+V.
pub fn paste_text(_text: &str) -> Result<()> {
    info!("Pasting clipboard contents via simulated Ctrl+V");

    let kind = session_kind();

    // On X11, xdotool is usually the most reliable "no daemon" option.
    if kind == SessionKind::X11 && command_exists("xdotool") {
        if paste_with_xdotool_ctrl_v().is_ok() {
            debug!("Clipboard pasted via xdotool Ctrl+V");
            return Ok(());
        }
    }

    check_ydotoold_running();

    if let Err(e) = paste_with_ydotool_ctrl_v() {
        warn!("ydotool Ctrl+V failed: {}", e);

        // wtype fallback is for wlroots-based Wayland compositors. It won't help on GNOME.
        if kind == SessionKind::Wayland && command_exists("wtype") {
            info!("Trying wtype Ctrl+V fallback...");
            return paste_with_wtype_ctrl_v();
        }

        return Err(e);
    }

    debug!("Clipboard pasted via ydotool Ctrl+V");
    Ok(())
}

fn check_ydotoold_running() {
    let daemon_check = Command::new("pgrep").arg("ydotoold").output();

    match daemon_check {
        Ok(output) if output.status.success() => {
            debug!("ydotoold daemon is running");
        }
        _ => {
            debug!("ydotoold may not be running.");
        }
    }
}

/// X11-only: use xdotool to send Ctrl+V.
fn paste_with_xdotool_ctrl_v() -> Result<()> {
    let status = Command::new("xdotool")
        .args(["key", "--clearmodifiers", "ctrl+v"])
        .status()
        .context("Failed to run xdotool. Install with: sudo apt install xdotool")?;

    if !status.success() {
        anyhow::bail!("xdotool Ctrl+V failed");
    }

    Ok(())
}

/// Alternative: use wtype to send Ctrl+V.
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

/// Fallback: simulate Ctrl+V keystroke using ydotool.
/// This assumes text is already in clipboard.
fn paste_with_ydotool_ctrl_v() -> Result<()> {
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

/// Alternative clipboard copy using wl-copy command (Wayland only).
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

/// Get text from clipboard (Wayland only).
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
