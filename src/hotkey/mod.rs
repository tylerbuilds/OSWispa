//! Global hotkey detection using evdev
//!
//! On Wayland, we can't use X11 APIs for global hotkeys.
//! Instead, we read directly from /dev/input/event* devices.
//! This requires either root privileges or membership in the 'input' group.
//!
//! Features:
//! - Configurable hotkey (Ctrl, Alt, Shift, Super in any combination)
//! - Hold to record, release to transcribe
//! - ESC while recording: Cancel recording
//! - Quick tap: Cancel recording

use crate::{AppEvent, Config, HotkeyConfig};
use anyhow::{Context, Result};
use crossbeam_channel::{Receiver, Sender};
use evdev::{Device, EventType, InputEventKind, Key};
use std::collections::HashSet;
use std::fs;
use std::sync::Arc;

use std::time::{Duration, Instant};
use tracing::{debug, info, error};

/// Time threshold for detecting a "quick tap" (cancel gesture)
const QUICK_TAP_THRESHOLD_MS: u64 = 200;

/// Find keyboard devices
fn find_keyboards() -> Result<Vec<Device>> {
    let mut keyboards = Vec::new();

    for entry in fs::read_dir("/dev/input")? {
        let entry = entry?;
        let path = entry.path();

        if let Some(name) = path.file_name() {
            let name = name.to_string_lossy();
            if name.starts_with("event") {
                if let Ok(device) = Device::open(&path) {
                    // Check if device has keyboard capabilities
                    if device.supported_keys().map_or(false, |keys| {
                        keys.contains(Key::KEY_LEFTCTRL) || keys.contains(Key::KEY_LEFTALT)
                    }) {
                        info!(
                            "Found keyboard: {} at {:?}",
                            device.name().unwrap_or("Unknown"),
                            path
                        );
                        keyboards.push(device);
                    }
                }
            }
        }
    }

    if keyboards.is_empty() {
        anyhow::bail!(
            "No keyboard devices found. Make sure you're in the 'input' group:\n\
            sudo usermod -aG input $USER\n\
            Then log out and back in."
        );
    }

    Ok(keyboards)
}

/// Check if the configured hotkey combination is currently pressed
fn is_hotkey_active(pressed_keys: &HashSet<Key>, config: &HotkeyConfig) -> bool {
    let ctrl_ok = if config.ctrl {
        pressed_keys.contains(&Key::KEY_LEFTCTRL) || pressed_keys.contains(&Key::KEY_RIGHTCTRL)
    } else {
        !pressed_keys.contains(&Key::KEY_LEFTCTRL) && !pressed_keys.contains(&Key::KEY_RIGHTCTRL)
    };

    let alt_ok = if config.alt {
        pressed_keys.contains(&Key::KEY_LEFTALT) || pressed_keys.contains(&Key::KEY_RIGHTALT)
    } else {
        !pressed_keys.contains(&Key::KEY_LEFTALT) && !pressed_keys.contains(&Key::KEY_RIGHTALT)
    };

    let shift_ok = if config.shift {
        pressed_keys.contains(&Key::KEY_LEFTSHIFT) || pressed_keys.contains(&Key::KEY_RIGHTSHIFT)
    } else {
        !pressed_keys.contains(&Key::KEY_LEFTSHIFT) && !pressed_keys.contains(&Key::KEY_RIGHTSHIFT)
    };

    let super_ok = if config.super_key {
        pressed_keys.contains(&Key::KEY_LEFTMETA) || pressed_keys.contains(&Key::KEY_RIGHTMETA)
    } else {
        !pressed_keys.contains(&Key::KEY_LEFTMETA) && !pressed_keys.contains(&Key::KEY_RIGHTMETA)
    };

    // Stricter check: No OTHER keys should be pressed besides the configured modifiers
    // This prevents "Ctrl + Print Screen" from triggering if only "Ctrl + Shift" is configured.
    let modifier_keys = [
        Key::KEY_LEFTCTRL, Key::KEY_RIGHTCTRL,
        Key::KEY_LEFTALT, Key::KEY_RIGHTALT,
        Key::KEY_LEFTSHIFT, Key::KEY_RIGHTSHIFT,
        Key::KEY_LEFTMETA, Key::KEY_RIGHTMETA,
    ];
    
    let has_unconfigured_key = pressed_keys.iter().any(|k| !modifier_keys.contains(k));

    let has_any_modifier = config.ctrl || config.alt || config.shift || config.super_key;

    has_any_modifier && ctrl_ok && alt_ok && shift_ok && super_ok && !has_unconfigured_key
}

/// Format hotkey for display
fn format_hotkey(config: &HotkeyConfig) -> String {
    let mut parts = Vec::new();
    if config.ctrl {
        parts.push("Ctrl");
    }
    if config.alt {
        parts.push("Alt");
    }
    if config.shift {
        parts.push("Shift");
    }
    if config.super_key {
        parts.push("Super");
    }
    if parts.is_empty() {
        "None".to_string()
    } else {
        parts.join("+")
    }
}

/// Listen for configured hotkey combination using a single thread with select/poll
pub fn listen_for_hotkey(
    event_tx: Sender<AppEvent>,
    config_rx: Receiver<Arc<Config>>,
    initial_config: Arc<Config>
) -> Result<()> {
    info!("Starting hotkey listener...");

    let mut config = initial_config;
    let mut keyboards = find_keyboards().context("Failed to find keyboard devices")?;
    
    info!("Monitoring {} keyboard device(s) for hotkeys", keyboards.len());

    let mut hotkey_str = format_hotkey(&config.hotkey);
    info!("Hotkey listener active: Hold {} to record", hotkey_str);

    // Shared state
    let mut pressed_keys: HashSet<Key> = HashSet::new();
    let mut is_recording = false;
    let mut recording_start_time: Option<Instant> = None;

    // Use nix for poll
    use std::os::unix::io::AsRawFd;
    
    loop {
        // Check for config updates
        while let Ok(new_config) = config_rx.try_recv() {
            info!("Hotkey listener reloading configuration...");
            config = new_config;
            hotkey_str = format_hotkey(&config.hotkey);
            info!("New hotkey: {}", hotkey_str);
        }

        // Create poll fd list for all keyboards
        let mut pollfds: Vec<libc::pollfd> = keyboards.iter().map(|kb| {
            libc::pollfd {
                fd: kb.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            }
        }).collect();

        // Poll with 100ms timeout
        let poll_result = unsafe {
            libc::poll(pollfds.as_mut_ptr(), pollfds.len() as libc::nfds_t, 100)
        };

        if poll_result < 0 {
            let err = std::io::Error::last_os_error();
            if err.kind() != std::io::ErrorKind::Interrupted {
                error!("Poll error: {}", err);
            }
            continue;
        }

        // Check which devices have events
        for (i, pollfd) in pollfds.iter().enumerate() {
            if pollfd.revents & libc::POLLIN != 0 {
                // This keyboard has events
                if let Ok(events) = keyboards[i].fetch_events() {
                    for event in events {
                        if event.event_type() == EventType::KEY {
                            if let InputEventKind::Key(key) = event.kind() {
                                let value = event.value();

                                match value {
                                    1 => {
                                        pressed_keys.insert(key);
                                        debug!("Key pressed: {:?}", key);

                                        // Check for ESC to cancel
                                        if key == Key::KEY_ESC && is_recording {
                                            info!("ESC pressed - cancelling recording");
                                            is_recording = false;
                                            recording_start_time = None;
                                            let _ = event_tx.send(AppEvent::CancelRecording);
                                            continue;
                                        }
                                    }
                                    0 => {
                                        pressed_keys.remove(&key);
                                        debug!("Key released: {:?}", key);
                                    }
                                    2 => {
                                        // Key repeat - ignore
                                    }
                                    _ => {}
                                }
                                
                                let combo_active = is_hotkey_active(&pressed_keys, &config.hotkey);

                                if combo_active && !is_recording {
                                    info!("Recording started ({} pressed)", hotkey_str);
                                    is_recording = true;
                                    recording_start_time = Some(Instant::now());
                                    let _ = event_tx.send(AppEvent::StartRecording);
                                } else if !combo_active && is_recording {
                                    let was_quick_tap = recording_start_time
                                        .map(|t| t.elapsed() < Duration::from_millis(QUICK_TAP_THRESHOLD_MS))
                                        .unwrap_or(false);

                                    is_recording = false;
                                    recording_start_time = None;

                                    if was_quick_tap {
                                        info!("Quick tap detected - cancelling recording");
                                        let _ = event_tx.send(AppEvent::CancelRecording);
                                    } else {
                                        info!("Recording stopped ({} released)", hotkey_str);
                                        let _ = event_tx.send(AppEvent::StopRecording);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
