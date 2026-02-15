//! Audio recording integration.

use crate::{AppEvent, RecordCommand};
use crossbeam_channel::{Receiver, Sender};
use std::path::PathBuf;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::audio_worker;

#[cfg(not(target_os = "linux"))]
pub fn audio_worker(
    record_rx: Receiver<RecordCommand>,
    audio_tx: Sender<Option<PathBuf>>,
    status_tx: Sender<AppEvent>,
) {
    // Stub implementation for non-Linux platforms.
    // macOS/Windows recording will be implemented in v0.4.0/v0.5.0.
    for cmd in record_rx {
        if matches!(cmd, RecordCommand::Start) {
            let _ = status_tx.send(AppEvent::Error(
                "Audio recording is not implemented on this OS yet".to_string(),
            ));
            let _ = audio_tx.send(None);
        }
    }
}
