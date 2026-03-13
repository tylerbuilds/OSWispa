//! Input simulation and clipboard management for macOS.
//!
//! - Clipboard: `arboard` crate (backed by NSPasteboard)
//! - Text injection: `enigo` crate (backed by CGEvent / Accessibility API)
//! - Fallback: `pbcopy`/`pbpaste` commands + `osascript` keystroke injection

use anyhow::{Context, Result};
use std::io::Write as IoWrite;
use std::process::{Command, Stdio};
use tracing::{debug, info, warn};

/// Copy text to the macOS clipboard via arboard.
pub fn copy_to_clipboard(text: &str) -> Result<()> {
    info!("Copying {} chars to clipboard", text.len());

    let mut clipboard = arboard::Clipboard::new().context("Failed to access macOS clipboard")?;
    clipboard
        .set_text(text)
        .context("Failed to set clipboard text")?;

    debug!("Text copied to clipboard via arboard");
    Ok(())
}

/// Copy text to clipboard and verify it actually landed.
pub fn copy_to_clipboard_verified(text: &str) -> Result<()> {
    copy_to_clipboard(text)?;

    // arboard on macOS is synchronous (NSPasteboard), so a single
    // read-back is sufficient — no Wayland-style race conditions.
    std::thread::sleep(std::time::Duration::from_millis(50));

    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        if let Ok(contents) = clipboard.get_text() {
            if contents.trim() == text.trim() {
                debug!("Clipboard verified");
                return Ok(());
            }
            debug!(
                "Clipboard mismatch: got '{}', expected '{}'",
                contents.chars().take(30).collect::<String>(),
                text.chars().take(30).collect::<String>()
            );
        }
    }

    // Fall back to pbcopy if arboard verification fails
    warn!("arboard verification failed, trying pbcopy fallback");
    copy_via_pbcopy(text)
}

/// Paste text by typing it via enigo, falling back to Cmd+V keystroke.
pub fn paste_text(text: &str) -> Result<()> {
    info!("Pasting {} chars via enigo", text.len());

    // Try enigo direct text input first
    match type_with_enigo(text) {
        Ok(()) => return Ok(()),
        Err(e) => {
            warn!("enigo text input failed: {}, falling back to Cmd+V", e);
        }
    }

    // Fallback: simulate Cmd+V keystroke (text must already be in clipboard)
    paste_with_cmd_v()
}

/// Type text directly using the enigo crate.
fn type_with_enigo(text: &str) -> Result<()> {
    use enigo::{Enigo, Keyboard, Settings};

    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|e| anyhow::anyhow!("Failed to create enigo instance: {:?}", e))?;

    enigo
        .text(text)
        .map_err(|e| anyhow::anyhow!("Failed to type text via enigo: {:?}", e))?;

    debug!("Text typed via enigo");
    Ok(())
}

/// Simulate Cmd+V keystroke via osascript.
fn paste_with_cmd_v() -> Result<()> {
    let status = Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to keystroke \"v\" using command down")
        .status()
        .context("Failed to run osascript for Cmd+V")?;

    if !status.success() {
        anyhow::bail!("osascript Cmd+V failed");
    }

    debug!("Pasted via osascript Cmd+V");
    Ok(())
}

/// Fallback clipboard copy using pbcopy command.
fn copy_via_pbcopy(text: &str) -> Result<()> {
    let mut child = Command::new("pbcopy")
        .stdin(Stdio::piped())
        .spawn()
        .context("Failed to run pbcopy")?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes())?;
    }

    let status = child.wait()?;
    if !status.success() {
        anyhow::bail!("pbcopy failed");
    }

    debug!("Text copied via pbcopy");
    Ok(())
}

/// Get text from clipboard.
#[allow(dead_code)]
pub fn get_from_clipboard() -> Result<String> {
    let mut clipboard = arboard::Clipboard::new().context("Failed to access macOS clipboard")?;
    clipboard.get_text().context("Failed to get clipboard text")
}
