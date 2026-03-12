//! Audio recording module using 'arecord' CLI
//!
//! Replaces cpal with system `arecord` for reliability on systems where
//! native Rust audio libraries might hang or fail.
//!
//! arecord is part of alsa-utils and is universally available on Linux.

use crate::{AppEvent, Config, RecordCommand, StreamingAudioMessage, TranscriptionBackend};
use anyhow::{Context, Result};
use crossbeam_channel::{Receiver, Sender};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tracing::{debug, error, info};

/// Default ALSA device. Using "pulse" routes through PipeWire/PulseAudio
/// which respects the user's configured default input source.
const DEFAULT_AUDIO_DEVICE: &str = "pulse";

/// Audio worker that listens for start/stop/cancel signals
pub fn audio_worker(
    record_rx: Receiver<RecordCommand>,
    audio_tx: Sender<Option<PathBuf>>,
    stream_tx: Sender<StreamingAudioMessage>,
    status_tx: Sender<AppEvent>,
    config: Arc<RwLock<Config>>,
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
                let stream_tx_clone = stream_tx.clone();
                let status_tx_clone = status_tx.clone();
                let config_snapshot = config.read().unwrap().clone();

                recording.store(true, Ordering::SeqCst);
                cancelled.store(false, Ordering::SeqCst);

                // Spawn Supervisor Thread
                _recording_thread = Some(std::thread::spawn(move || {
                    info!("AudioWorker: Starting arecord session");
                    let result = run_arecord_session(
                        &recording_clone,
                        &cancelled_clone,
                        &status_tx_clone,
                        &stream_tx_clone,
                        &config_snapshot,
                    );

                    match result {
                        Ok(path) => {
                            if cancelled_clone.load(Ordering::SeqCst) {
                                info!("Recording was cancelled, deleting file");
                                let _ = std::fs::remove_file(&path);
                                let _ = stream_tx_clone.send(StreamingAudioMessage::Cancel);
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
    cancelled: &Arc<AtomicBool>,
    _status_tx: &Sender<AppEvent>,
    stream_tx: &Sender<StreamingAudioMessage>,
    config: &Config,
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

    let streaming_active = config.backend == TranscriptionBackend::Local && config.streaming.enabled;
    let chunk_duration_ms = config.streaming.chunk_duration_ms.clamp(250, 1000);
    let chunk_bytes = chunk_duration_ms as usize * 16000 * 2 / 1000;
    let mut stream_offset = 44_u64;
    let mut pending_pcm = Vec::new();

    if streaming_active {
        let _ = stream_tx.send(StreamingAudioMessage::Begin);
    }

    // Monitoring loop
    while recording.load(Ordering::SeqCst) {
        if streaming_active {
            stream_new_pcm_data(
                &audio_path,
                &mut stream_offset,
                &mut pending_pcm,
                chunk_bytes,
                stream_tx,
            )?;
        }

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

    if streaming_active {
        stream_new_pcm_data(
            &audio_path,
            &mut stream_offset,
            &mut pending_pcm,
            chunk_bytes,
            stream_tx,
        )?;
        if !pending_pcm.is_empty() {
            let samples = pcm_bytes_to_samples(&pending_pcm);
            if !samples.is_empty() {
                let _ = stream_tx.send(StreamingAudioMessage::Chunk(samples));
            }
        }
        if !cancelled.load(Ordering::SeqCst) {
            let _ = stream_tx.send(StreamingAudioMessage::Finalize);
        }
    }

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

fn stream_new_pcm_data(
    audio_path: &std::path::Path,
    offset: &mut u64,
    pending_pcm: &mut Vec<u8>,
    chunk_bytes: usize,
    stream_tx: &Sender<StreamingAudioMessage>,
) -> Result<()> {
    if !audio_path.exists() {
        return Ok(());
    }

    let mut file = File::open(audio_path)?;
    let file_len = file.metadata()?.len();
    if file_len <= *offset {
        return Ok(());
    }

    file.seek(SeekFrom::Start(*offset))?;
    let mut new_bytes = Vec::new();
    file.read_to_end(&mut new_bytes)?;
    *offset += new_bytes.len() as u64;

    pending_pcm.extend_from_slice(&new_bytes);
    if pending_pcm.len() % 2 != 0 {
        pending_pcm.pop();
    }

    while pending_pcm.len() >= chunk_bytes {
        let chunk: Vec<u8> = pending_pcm.drain(..chunk_bytes).collect();
        let samples = pcm_bytes_to_samples(&chunk);
        if !samples.is_empty() {
            let _ = stream_tx.send(StreamingAudioMessage::Chunk(samples));
        }
    }

    Ok(())
}

fn pcm_bytes_to_samples(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]) as f32 / 32768.0)
        .collect()
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
