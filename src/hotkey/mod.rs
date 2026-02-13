//! Global hotkey detection using evdev
//!
//! On Wayland, we can't use X11 APIs for global hotkeys.
//! Instead, we read directly from /dev/input/event* devices.
//! This requires either root privileges or membership in the 'input' group.
//!
//! Features:
//! - Configurable hotkey (modifiers + optional trigger key)
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

const MODIFIER_KEYS: &[Key] = &[
    Key::KEY_LEFTCTRL,
    Key::KEY_RIGHTCTRL,
    Key::KEY_LEFTALT,
    Key::KEY_RIGHTALT,
    Key::KEY_LEFTSHIFT,
    Key::KEY_RIGHTSHIFT,
    Key::KEY_LEFTMETA,
    Key::KEY_RIGHTMETA,
];

fn parse_trigger_key(trigger_key: Option<&str>) -> Option<Key> {
    let key = trigger_key?.trim().to_ascii_lowercase();
    if key.is_empty() {
        return None;
    }

    match key.as_str() {
        "space" => Some(Key::KEY_SPACE),
        "tab" => Some(Key::KEY_TAB),
        "enter" => Some(Key::KEY_ENTER),
        "backspace" => Some(Key::KEY_BACKSPACE),
        "capslock" => Some(Key::KEY_CAPSLOCK),
        "grave" => Some(Key::KEY_GRAVE),
        "f1" => Some(Key::KEY_F1),
        "f2" => Some(Key::KEY_F2),
        "f3" => Some(Key::KEY_F3),
        "f4" => Some(Key::KEY_F4),
        "f5" => Some(Key::KEY_F5),
        "f6" => Some(Key::KEY_F6),
        "f7" => Some(Key::KEY_F7),
        "f8" => Some(Key::KEY_F8),
        "f9" => Some(Key::KEY_F9),
        "f10" => Some(Key::KEY_F10),
        "f11" => Some(Key::KEY_F11),
        "f12" => Some(Key::KEY_F12),
        "a" => Some(Key::KEY_A),
        "b" => Some(Key::KEY_B),
        "c" => Some(Key::KEY_C),
        "d" => Some(Key::KEY_D),
        "e" => Some(Key::KEY_E),
        "f" => Some(Key::KEY_F),
        "g" => Some(Key::KEY_G),
        "h" => Some(Key::KEY_H),
        "i" => Some(Key::KEY_I),
        "j" => Some(Key::KEY_J),
        "k" => Some(Key::KEY_K),
        "l" => Some(Key::KEY_L),
        "m" => Some(Key::KEY_M),
        "n" => Some(Key::KEY_N),
        "o" => Some(Key::KEY_O),
        "p" => Some(Key::KEY_P),
        "q" => Some(Key::KEY_Q),
        "r" => Some(Key::KEY_R),
        "s" => Some(Key::KEY_S),
        "t" => Some(Key::KEY_T),
        "u" => Some(Key::KEY_U),
        "v" => Some(Key::KEY_V),
        "w" => Some(Key::KEY_W),
        "x" => Some(Key::KEY_X),
        "y" => Some(Key::KEY_Y),
        "z" => Some(Key::KEY_Z),
        "0" => Some(Key::KEY_0),
        "1" => Some(Key::KEY_1),
        "2" => Some(Key::KEY_2),
        "3" => Some(Key::KEY_3),
        "4" => Some(Key::KEY_4),
        "5" => Some(Key::KEY_5),
        "6" => Some(Key::KEY_6),
        "7" => Some(Key::KEY_7),
        "8" => Some(Key::KEY_8),
        "9" => Some(Key::KEY_9),
        _ => None,
    }
}

fn display_trigger_key(trigger_key: Option<&str>) -> Option<String> {
    let key = trigger_key?.trim();
    if key.is_empty() {
        return None;
    }

    let pretty = match key.to_ascii_lowercase().as_str() {
        "space" => "Space".to_string(),
        "tab" => "Tab".to_string(),
        "enter" => "Enter".to_string(),
        "backspace" => "Backspace".to_string(),
        "capslock" => "CapsLock".to_string(),
        "grave" => "`".to_string(),
        other => other.to_ascii_uppercase(),
    };

    Some(pretty)
}

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

    let trigger = parse_trigger_key(config.trigger_key.as_deref());
    let trigger_ok = trigger
        .as_ref()
        .map(|key| pressed_keys.contains(key))
        .unwrap_or(true);

    // Stricter check: no non-modifier key except configured trigger key.
    let has_unconfigured_non_modifier = pressed_keys.iter().any(|k| {
        !MODIFIER_KEYS.contains(k) && trigger.as_ref().map(|t| t != k).unwrap_or(true)
    });

    let has_activation_key =
        config.ctrl || config.alt || config.shift || config.super_key || trigger.is_some();

    has_activation_key
        && ctrl_ok
        && alt_ok
        && shift_ok
        && super_ok
        && trigger_ok
        && !has_unconfigured_non_modifier
}

/// Format hotkey for display
fn format_hotkey(config: &HotkeyConfig) -> String {
    let mut parts: Vec<String> = Vec::new();
    if config.ctrl {
        parts.push("Ctrl".to_string());
    }
    if config.alt {
        parts.push("Alt".to_string());
    }
    if config.shift {
        parts.push("Shift".to_string());
    }
    if config.super_key {
        parts.push("Super".to_string());
    }
    if let Some(trigger_key) = display_trigger_key(config.trigger_key.as_deref()) {
        parts.push(trigger_key);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_trigger_key() {
        assert_eq!(parse_trigger_key(Some("space")), Some(Key::KEY_SPACE));
        assert_eq!(parse_trigger_key(Some("F8")), Some(Key::KEY_F8));
        assert_eq!(parse_trigger_key(Some("unknown")), None);
        assert_eq!(parse_trigger_key(None), None);
    }

    #[test]
    fn test_hotkey_with_trigger_key() {
        let config = HotkeyConfig {
            ctrl: true,
            alt: false,
            shift: false,
            super_key: false,
            trigger_key: Some("space".to_string()),
        };

        let mut pressed = HashSet::new();
        pressed.insert(Key::KEY_LEFTCTRL);
        assert!(!is_hotkey_active(&pressed, &config));

        pressed.insert(Key::KEY_SPACE);
        assert!(is_hotkey_active(&pressed, &config));
    }

    #[test]
    fn test_hotkey_rejects_extra_non_modifier_key() {
        let config = HotkeyConfig {
            ctrl: true,
            alt: false,
            shift: false,
            super_key: false,
            trigger_key: Some("space".to_string()),
        };

        let mut pressed = HashSet::new();
        pressed.insert(Key::KEY_LEFTCTRL);
        pressed.insert(Key::KEY_SPACE);
        pressed.insert(Key::KEY_A);

        assert!(!is_hotkey_active(&pressed, &config));
    }
}
