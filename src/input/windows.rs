//! Clipboard and text insertion for Windows.

use anyhow::{Context, Result};
use tracing::{debug, info, warn};

pub fn copy_to_clipboard(text: &str) -> Result<()> {
    info!("Copying {} chars to the Windows clipboard", text.len());
    let mut clipboard = arboard::Clipboard::new().context("Failed to access Windows clipboard")?;
    clipboard
        .set_text(text)
        .context("Failed to set Windows clipboard text")
}

pub fn copy_to_clipboard_verified(text: &str) -> Result<()> {
    copy_to_clipboard(text)?;
    std::thread::sleep(std::time::Duration::from_millis(50));

    let mut clipboard = arboard::Clipboard::new().context("Failed to reopen Windows clipboard")?;
    let contents = clipboard
        .get_text()
        .context("Failed to verify Windows clipboard text")?;
    if contents.trim() != text.trim() {
        anyhow::bail!("Windows clipboard verification failed");
    }
    debug!("Windows clipboard verified");
    Ok(())
}

pub fn paste_text(text: &str) -> Result<()> {
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};

    let mut enigo = Enigo::new(&Settings::default())
        .map_err(|error| anyhow::anyhow!("Failed to initialise Windows input: {:?}", error))?;

    if let Err(error) = enigo.text(text) {
        warn!(
            "Direct Windows text insertion failed: {:?}; falling back to Ctrl+V",
            error
        );
        enigo
            .key(Key::Control, Direction::Press)
            .map_err(|error| anyhow::anyhow!("Failed to press Ctrl: {:?}", error))?;
        let paste_result = enigo.key(Key::V, Direction::Click);
        let release_result = enigo.key(Key::Control, Direction::Release);
        paste_result.map_err(|error| anyhow::anyhow!("Failed to press V: {:?}", error))?;
        release_result.map_err(|error| anyhow::anyhow!("Failed to release Ctrl: {:?}", error))?;
    }

    Ok(())
}

#[allow(dead_code)]
pub fn get_from_clipboard() -> Result<String> {
    let mut clipboard = arboard::Clipboard::new().context("Failed to access Windows clipboard")?;
    clipboard
        .get_text()
        .context("Failed to get Windows clipboard text")
}
