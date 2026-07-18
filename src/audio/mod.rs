//! Audio recording integration.

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
use crate::{AppEvent, RecordCommand};
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
use crossbeam_channel::{Receiver, Sender};
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
use std::path::PathBuf;

pub(crate) fn private_recording_temp_path() -> anyhow::Result<tempfile::TempPath> {
    // The legacy prefix is retained so existing diagnostics and cleanup guidance stay valid.
    Ok(tempfile::Builder::new()
        .prefix("oswispa_recording_")
        .suffix(".wav")
        .tempfile()?
        .into_temp_path())
}

#[cfg(any(target_os = "macos", test))]
mod conversion;

#[cfg(target_os = "linux")]
mod linux;

#[cfg(target_os = "linux")]
pub use linux::audio_worker;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use macos::audio_worker;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "windows")]
pub use windows::audio_worker;

#[cfg(target_os = "linux")]
pub fn backend_name() -> &'static str {
    "linux-arecord"
}

#[cfg(target_os = "macos")]
pub fn backend_name() -> &'static str {
    "macos-coreaudio-cpal"
}

#[cfg(target_os = "windows")]
pub fn backend_name() -> &'static str {
    "windows-wasapi-cpal"
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
pub fn backend_name() -> &'static str {
    "unsupported"
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temporary_recording_is_unique_and_deleted_on_drop() {
        let first = private_recording_temp_path().unwrap();
        let second = private_recording_temp_path().unwrap();
        let first_path = first.to_path_buf();
        let second_path = second.to_path_buf();
        assert_ne!(first_path, second_path);

        assert!(first_path.exists());
        drop(first);
        assert!(!first_path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn temporary_recording_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;

        let recording = private_recording_temp_path().unwrap();
        let mode = std::fs::metadata(&recording).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
}
