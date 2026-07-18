//! Global hotkey detection for Windows using `rdev`.

use crate::{AppEvent, Config, HotkeyConfig};
use anyhow::Result;
use crossbeam_channel::{Receiver, Sender};
use rdev::{listen, Event, EventType, Key};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, error, info, warn};

const QUICK_TAP_THRESHOLD_MS: u64 = 200;

fn is_modifier(key: &Key) -> bool {
    matches!(
        key,
        Key::ControlLeft
            | Key::ControlRight
            | Key::Alt
            | Key::AltGr
            | Key::ShiftLeft
            | Key::ShiftRight
            | Key::MetaLeft
            | Key::MetaRight
    )
}

fn parse_trigger_key(trigger_key: Option<&str>) -> Option<Key> {
    let key = trigger_key?.trim().to_ascii_lowercase();
    if key.is_empty() {
        return None;
    }

    match key.as_str() {
        "space" => Some(Key::Space),
        "tab" => Some(Key::Tab),
        "enter" => Some(Key::Return),
        "backspace" => Some(Key::Backspace),
        "capslock" => Some(Key::CapsLock),
        "grave" => Some(Key::BackQuote),
        "f1" => Some(Key::F1),
        "f2" => Some(Key::F2),
        "f3" => Some(Key::F3),
        "f4" => Some(Key::F4),
        "f5" => Some(Key::F5),
        "f6" => Some(Key::F6),
        "f7" => Some(Key::F7),
        "f8" => Some(Key::F8),
        "f9" => Some(Key::F9),
        "f10" => Some(Key::F10),
        "f11" => Some(Key::F11),
        "f12" => Some(Key::F12),
        "a" => Some(Key::KeyA),
        "b" => Some(Key::KeyB),
        "c" => Some(Key::KeyC),
        "d" => Some(Key::KeyD),
        "e" => Some(Key::KeyE),
        "f" => Some(Key::KeyF),
        "g" => Some(Key::KeyG),
        "h" => Some(Key::KeyH),
        "i" => Some(Key::KeyI),
        "j" => Some(Key::KeyJ),
        "k" => Some(Key::KeyK),
        "l" => Some(Key::KeyL),
        "m" => Some(Key::KeyM),
        "n" => Some(Key::KeyN),
        "o" => Some(Key::KeyO),
        "p" => Some(Key::KeyP),
        "q" => Some(Key::KeyQ),
        "r" => Some(Key::KeyR),
        "s" => Some(Key::KeyS),
        "t" => Some(Key::KeyT),
        "u" => Some(Key::KeyU),
        "v" => Some(Key::KeyV),
        "w" => Some(Key::KeyW),
        "x" => Some(Key::KeyX),
        "y" => Some(Key::KeyY),
        "z" => Some(Key::KeyZ),
        "0" => Some(Key::Num0),
        "1" => Some(Key::Num1),
        "2" => Some(Key::Num2),
        "3" => Some(Key::Num3),
        "4" => Some(Key::Num4),
        "5" => Some(Key::Num5),
        "6" => Some(Key::Num6),
        "7" => Some(Key::Num7),
        "8" => Some(Key::Num8),
        "9" => Some(Key::Num9),
        _ => None,
    }
}

fn is_hotkey_active(pressed_keys: &HashSet<Key>, config: &HotkeyConfig) -> bool {
    let ctrl_ok = if config.ctrl {
        pressed_keys.contains(&Key::ControlLeft) || pressed_keys.contains(&Key::ControlRight)
    } else {
        !pressed_keys.contains(&Key::ControlLeft) && !pressed_keys.contains(&Key::ControlRight)
    };
    let alt_ok = if config.alt {
        pressed_keys.contains(&Key::Alt) || pressed_keys.contains(&Key::AltGr)
    } else {
        !pressed_keys.contains(&Key::Alt) && !pressed_keys.contains(&Key::AltGr)
    };
    let shift_ok = if config.shift {
        pressed_keys.contains(&Key::ShiftLeft) || pressed_keys.contains(&Key::ShiftRight)
    } else {
        !pressed_keys.contains(&Key::ShiftLeft) && !pressed_keys.contains(&Key::ShiftRight)
    };
    let super_ok = if config.super_key {
        pressed_keys.contains(&Key::MetaLeft) || pressed_keys.contains(&Key::MetaRight)
    } else {
        !pressed_keys.contains(&Key::MetaLeft) && !pressed_keys.contains(&Key::MetaRight)
    };
    let trigger = parse_trigger_key(config.trigger_key.as_deref());
    let trigger_ok = trigger
        .as_ref()
        .map(|key| pressed_keys.contains(key))
        .unwrap_or(true);
    let has_unconfigured_non_modifier = pressed_keys
        .iter()
        .any(|key| !is_modifier(key) && trigger.as_ref().map(|item| item != key).unwrap_or(true));
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

fn format_hotkey(config: &HotkeyConfig) -> String {
    let mut parts = Vec::new();
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
        parts.push("Win".to_string());
    }
    if let Some(trigger) = config
        .trigger_key
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        parts.push(trigger.to_ascii_uppercase());
    }
    if parts.is_empty() {
        "None".to_string()
    } else {
        parts.join("+")
    }
}

/// Listen for the configured Windows hotkey until the process exits.
pub fn listen_for_hotkey(
    event_tx: Sender<AppEvent>,
    config_rx: Receiver<Arc<Config>>,
    initial_config: Arc<Config>,
) -> Result<()> {
    info!("Starting hotkey listener (Windows/rdev)");
    let config = Arc::new(std::sync::Mutex::new((*initial_config).clone()));
    let config_for_thread = Arc::clone(&config);

    std::thread::spawn(move || {
        for new_config in config_rx {
            if let Ok(mut current) = config_for_thread.lock() {
                *current = (*new_config).clone();
                info!("Hotkey listener reloaded configuration");
            }
        }
    });

    let hotkey = {
        let current = config.lock().unwrap_or_else(|error| error.into_inner());
        format_hotkey(&current.hotkey)
    };
    info!("Hotkey listener active: Hold {} to record", hotkey);

    let pressed_keys = Arc::new(std::sync::Mutex::new(HashSet::<Key>::new()));
    let is_recording = Arc::new(AtomicBool::new(false));
    let recording_start_ms = Arc::new(AtomicU64::new(0));
    let epoch = Instant::now();

    let callback = {
        let pressed_keys = Arc::clone(&pressed_keys);
        let is_recording = Arc::clone(&is_recording);
        let recording_start_ms = Arc::clone(&recording_start_ms);
        let config = Arc::clone(&config);

        move |event: Event| {
            let (key, is_press) = match event.event_type {
                EventType::KeyPress(key) => (key, true),
                EventType::KeyRelease(key) => (key, false),
                _ => return,
            };
            let Ok(mut pressed) = pressed_keys.try_lock() else {
                warn!("Hotkey key-state lock was busy; skipping event");
                return;
            };
            let was_recording = is_recording.load(Ordering::SeqCst);

            if is_press {
                pressed.insert(key);
                debug!("Key pressed: {:?}", key);
                if key == Key::Escape && was_recording {
                    is_recording.store(false, Ordering::SeqCst);
                    recording_start_ms.store(0, Ordering::SeqCst);
                    let _ = event_tx.send(AppEvent::CancelRecording);
                    return;
                }
            } else {
                pressed.remove(&key);
                debug!("Key released: {:?}", key);
            }

            let Ok(current_config) = config.try_lock() else {
                warn!("Hotkey configuration lock was busy; skipping event");
                return;
            };
            let hotkey_config = current_config.hotkey.clone();
            drop(current_config);
            let combo_active = is_hotkey_active(&pressed, &hotkey_config);
            drop(pressed);

            if combo_active && !was_recording {
                is_recording.store(true, Ordering::SeqCst);
                recording_start_ms.store(epoch.elapsed().as_millis() as u64, Ordering::SeqCst);
                let _ = event_tx.send(AppEvent::StartRecording);
            } else if !combo_active && was_recording {
                let start_ms = recording_start_ms.swap(0, Ordering::SeqCst);
                let quick_tap = start_ms > 0
                    && (epoch.elapsed().as_millis() as u64).saturating_sub(start_ms)
                        < QUICK_TAP_THRESHOLD_MS;
                is_recording.store(false, Ordering::SeqCst);
                let _ = event_tx.send(if quick_tap {
                    AppEvent::CancelRecording
                } else {
                    AppEvent::StopRecording
                });
            }
        }
    };

    if let Err(error) = listen(callback) {
        error!("Windows global hotkey listener failed: {:?}", error);
        anyhow::bail!(
            "Global hotkey listener failed. Run OSWispa in an interactive Windows desktop session."
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_hotkey_requires_control_and_windows_keys() {
        let config = HotkeyConfig::default();
        let mut pressed = HashSet::new();
        pressed.insert(Key::ControlLeft);
        assert!(!is_hotkey_active(&pressed, &config));
        pressed.insert(Key::MetaLeft);
        assert!(is_hotkey_active(&pressed, &config));
    }

    #[test]
    fn trigger_keys_are_parsed() {
        assert_eq!(parse_trigger_key(Some("F8")), Some(Key::F8));
        assert_eq!(parse_trigger_key(Some("r")), Some(Key::KeyR));
        assert_eq!(parse_trigger_key(Some("unknown")), None);
    }
}
