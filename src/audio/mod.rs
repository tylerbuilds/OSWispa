//! Audio recording module using 'arecord' CLI
//!
//! Replaces cpal with system `arecord` for reliability on systems where
//! native Rust audio libraries might hang or fail.
//!
//! arecord is part of alsa-utils and is universally available on Linux.

use crate::{AppEvent, Config, RecordCommand};
use anyhow::{Context, Result};
use crossbeam_channel::{Receiver, Sender};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info};

/// Audio worker that listens for start/stop/cancel signals
pub fn audio_worker(
    record_rx: Receiver<RecordCommand>,
    audio_tx: Sender<Option<PathBuf>>,
    status_tx: Sender<AppEvent>,
    _config: &Config,
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
                                let _ = status_tx_clone
                                    .send(AppEvent::Error(format!("Audio recording failed: {}", e)));
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
    let mut child = Command::new("arecord")
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
    
    // Send SIGTERM
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
        anyhow::bail!("Audio file too small ({} bytes), recording failed", metadata.len());
    }

    info!("Audio file ready: {} bytes", metadata.len());
    Ok(audio_path)
}
