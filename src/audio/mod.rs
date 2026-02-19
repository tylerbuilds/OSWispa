//! Audio recording integration.

use crate::{AppEvent, RecordCommand};
use crossbeam_channel::{Receiver, Sender};
use std::path::PathBuf;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::audio_worker;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use macos::audio_worker;

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn audio_worker(
    record_rx: Receiver<RecordCommand>,
    audio_tx: Sender<Option<PathBuf>>,
    status_tx: Sender<AppEvent>,
) {
    // Stub implementation for unsupported platforms.
    for cmd in record_rx {
        if matches!(cmd, RecordCommand::Start) {
            let _ = status_tx.send(AppEvent::Error(
                "Audio recording is not implemented on this OS yet".to_string(),
            ));
            let _ = audio_tx.send(None);
        }
    }
}
