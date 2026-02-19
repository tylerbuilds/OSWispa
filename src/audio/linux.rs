//! Audio recording module using 'arecord' CLI
//!
//! Replaces cpal with system `arecord` for reliability on systems where
//! native Rust audio libraries might hang or fail.
//!
//! arecord is part of alsa-utils and is universally available on Linux.

use crate::{AppEvent, RecordCommand};
use anyhow::{Context, Result};
use crossbeam_channel::{Receiver, Sender};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info};

/// Default ALSA device. Using "pulse" routes through PipeWire/PulseAudio
/// which respects the user's configured default input source.
const DEFAULT_AUDIO_DEVICE: &str = "pulse";

/// Audio worker that listens for start/stop/cancel signals
pub fn audio_worker(
    record_rx: Receiver<RecordCommand>,
    audio_tx: Sender<Option<PathBuf>>,
    status_tx: Sender<AppEvent>,
) {
    info!("Audio worker thread started");
    let recording = Arc::new(AtomicBool::new(false));
    let cancelled = Arc::new(AtomicBool::new(false));

    let mut _recording_thread: Option<std::thread::JoinHandle<()>> = None;

    for cmd in record_rx {
        match cmd {
            RecordCommand::Start => {
                if recording.load(Ordering::SeqCst) {
                    info!("Already recording, ignoring start signal");
                    continue;
                }

                let recording_clone = Arc::clone(&recording);
                let cancelled_clone = Arc::clone(&cancelled);
                let audio_tx_clone = audio_tx.clone();
                let status_tx_clone = status_tx.clone();

                recording.store(true, Ordering::SeqCst);
                cancelled.store(false, Ordering::SeqCst);

                // Spawn Supervisor Thread
                _recording_thread = Some(std::thread::spawn(move || {
                    info!("AudioWorker: Starting arecord session");
                    let result = run_arecord_session(&recording_clone, &status_tx_clone);

                    match result {
                        Ok(path) => {
                            if cancelled_clone.load(Ordering::SeqCst) {
                                info!("Recording was cancelled, deleting file");
                                let _ = std::fs::remove_file(&path);
                                let _ = audio_tx_clone.send(None);
                            } else {
                                info!("Recording saved to {:?}", path);
                                let _ = audio_tx_clone.send(Some(path));
                            }
                        }
                        Err(e) => {
                            if !cancelled_clone.load(Ordering::SeqCst) {
                                error!("arecord session failed: {}", e);
                                let _ = status_tx_clone.send(AppEvent::Error(format!(
                                    "Audio recording failed: {}",
                                    e
                                )));
                            }
                            let _ = audio_tx_clone.send(None);
                        }
                    }

                    recording_clone.store(false, Ordering::SeqCst);
                }));
            }
            RecordCommand::Stop => {
                info!("Stop recording signal received");
                recording.store(false, Ordering::SeqCst);
                // Supervisor thread will detect this and kill arecord
            }
            RecordCommand::Cancel => {
                info!("Cancel recording signal received");
                cancelled.store(true, Ordering::SeqCst);
                recording.store(false, Ordering::SeqCst);
            }
        }
    }
}

/// Manages the arecord process execution
fn run_arecord_session(
    recording: &Arc<AtomicBool>,
    _status_tx: &Sender<AppEvent>,
) -> Result<PathBuf> {
    let temp_dir = std::env::temp_dir();
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S_%3f");
    let audio_path = temp_dir.join(format!("oswispa_recording_{}.wav", timestamp));

    info!("Starting arecord process -> {:?}", audio_path);

    // arecord parameters for Whisper: 16kHz, Mono, S16_LE
    // Use "pulse" device by default to route through PipeWire/PulseAudio,
    // which respects the user's configured default input source.
    let mut child = Command::new("arecord")
        .arg("-D")
        .arg(DEFAULT_AUDIO_DEVICE)
        .arg("-r")
        .arg("16000")
        .arg("-c")
        .arg("1")
        .arg("-f")
        .arg("S16_LE")
        .arg(&audio_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("Failed to spawn 'arecord'. Is alsa-utils installed?")?;

    let child_pid = child.id();
    debug!("arecord started with PID {}", child_pid);

    // Monitoring loop
    while recording.load(Ordering::SeqCst) {
        // check if child exited unexpectedly
        match child.try_wait() {
            Ok(Some(status)) => {
                // Process exited unexpectedly
                let _ = std::fs::remove_file(&audio_path);
                anyhow::bail!("arecord exited prematurely with status: {}", status);
            }
            Ok(None) => {
                // Still running, wait a bit
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => {
                error!("Error waiting on arecord child: {}", e);
                break;
            }
        }
    }

    // Stop requested (or loop broke). Terminate process cleanly.
    debug!("Stopping arecord PID {}", child_pid);

    // Send SIGTERM to stop arecord.
    let _ = Command::new("kill")
        .arg("-TERM")
        .arg(child_pid.to_string())
        .output();

    // Wait for it to close
    let _ = child.wait();

    // Verify file
    if !audio_path.exists() {
        anyhow::bail!("Audio file not created");
    }

    let metadata = std::fs::metadata(&audio_path)?;
    if metadata.len() < 100 {
        let _ = std::fs::remove_file(&audio_path);
        anyhow::bail!(
            "Audio file too small ({} bytes), recording failed",
            metadata.len()
        );
    }

    // arecord leaves a placeholder data-length in the WAV header when
    // interrupted by a signal. Patch the RIFF and data chunk sizes so
    // downstream readers (hound / whisper) see the real sample count.
    fix_wav_header(&audio_path)?;

    info!("Audio file ready: {} bytes", metadata.len());
    Ok(audio_path)
}

/// Patch a WAV file's RIFF and data chunk sizes to match the actual file size.
///
/// arecord writes `0x7FFFFFFF` or similar placeholder values when it doesn't
/// know the recording duration upfront (i.e. every signal-interrupted session).
/// Standard PCM WAV layout:
///   offset  0: "RIFF"
///   offset  4: u32 LE = file_size - 8
///   offset  8: "WAVE"
///   ...
///   offset 36: "data"
///   offset 40: u32 LE = file_size - 44
fn fix_wav_header(path: &std::path::Path) -> Result<()> {
    use std::io::{Read, Seek, SeekFrom, Write};

    let file_len = std::fs::metadata(path)?.len();
    if file_len < 44 {
        anyhow::bail!("WAV file too small to contain a valid header");
    }

    let mut f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)?;

    // Validate RIFF magic at offset 0
    let mut magic = [0u8; 4];
    f.read_exact(&mut magic)?;
    if &magic != b"RIFF" {
        anyhow::bail!("Not a RIFF file");
    }

    // Validate "data" tag at offset 36
    f.seek(SeekFrom::Start(36))?;
    f.read_exact(&mut magic)?;
    if &magic != b"data" {
        anyhow::bail!("WAV data chunk not at expected offset 36; non-standard header layout");
    }

    // RIFF chunk size at offset 4
    let riff_size = (file_len - 8) as u32;
    f.seek(SeekFrom::Start(4))?;
    f.write_all(&riff_size.to_le_bytes())?;

    // data chunk size at offset 40
    let data_size = (file_len - 44) as u32;
    f.seek(SeekFrom::Start(40))?;
    f.write_all(&data_size.to_le_bytes())?;

    debug!(
        "Fixed WAV header: riff_size={}, data_size={} ({:.1}s)",
        riff_size,
        data_size,
        data_size as f32 / (16000.0 * 2.0)
    );

    Ok(())
}
