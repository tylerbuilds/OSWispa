//! Global hotkey detection for macOS using the rdev crate.
//!
//! Uses `rdev::listen()` to capture global keyboard events via CGEventTap.
//! Requires Accessibility permission in System Settings > Privacy & Security.
//!
//! IMPORTANT: The rdev callback runs on a Core Foundation run loop and must
//! never block. All mutex acquisitions use `try_lock()` to avoid stalling
//! the macOS event tap (which would cause the system to revoke it).

use crate::{AppEvent, Config, HotkeyConfig};
use anyhow::Result;
use crossbeam_channel::{Receiver, Sender};
use rdev::{listen, Event, EventType, Key};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

/// Time threshold for detecting a "quick tap" (cancel gesture)
const QUICK_TAP_THRESHOLD_MS: u64 = 200;

/// Map rdev Key to our modifier concept.
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

/// Parse a trigger key string to an rdev Key.
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

/// Check if the configured hotkey combination is currently pressed.
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

    // Reject if any non-modifier, non-trigger key is pressed
    let has_unconfigured_non_modifier = pressed_keys
        .iter()
        .any(|k| !is_modifier(k) && trigger.as_ref().map(|t| t != k).unwrap_or(true));

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

/// Format hotkey for display.
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
        parts.push("Cmd".to_string()); // macOS convention
    }
    if let Some(trigger) = config
        .trigger_key
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        parts.push(trigger.to_ascii_uppercase());
    }
    if parts.is_empty() {
        "None".to_string()
    } else {
        parts.join("+")
    }
}

/// Listen for configured hotkey combination using rdev.
///
/// This function blocks forever. It should be spawned in a dedicated thread.
pub fn listen_for_hotkey(
    event_tx: Sender<AppEvent>,
    config_rx: Receiver<Arc<Config>>,
    initial_config: Arc<Config>,
) -> Result<()> {
    info!("Starting hotkey listener (macOS/rdev)...");

    let config = Arc::new(std::sync::Mutex::new((*initial_config).clone()));
    let config_for_thread = Arc::clone(&config);

    // Spawn a thread to handle config updates
    std::thread::spawn(move || {
        for new_config in config_rx {
            if let Ok(mut cfg) = config_for_thread.lock() {
                *cfg = (*new_config).clone();
                info!("Hotkey listener reloaded configuration");
            }
        }
    });

    let hotkey_str = {
        let cfg = config.lock().unwrap_or_else(|e| e.into_inner());
        format_hotkey(&cfg.hotkey)
    };
    info!("Hotkey listener active: Hold {} to record", hotkey_str);

    // State shared with the rdev callback. Use atomics where possible to
    // avoid blocking the Core Foundation run loop.
    let pressed_keys = Arc::new(std::sync::Mutex::new(HashSet::<Key>::new()));
    let is_recording = Arc::new(AtomicBool::new(false));
    // Store recording start as epoch millis (0 = not recording). Avoids
    // a Mutex<Option<Instant>> that would need locking in the callback.
    let recording_start_ms = Arc::new(AtomicU64::new(0));

    let pressed_clone = Arc::clone(&pressed_keys);
    let recording_clone = Arc::clone(&is_recording);
    let start_clone = Arc::clone(&recording_start_ms);
    let config_clone = Arc::clone(&config);

    // Capture a reference epoch so we can convert Instant to/from AtomicU64.
    let epoch = Instant::now();

    let callback = move |event: Event| {
        let (key, is_press) = match event.event_type {
            EventType::KeyPress(k) => (k, true),
            EventType::KeyRelease(k) => (k, false),
            _ => return,
        };

        // try_lock: never block the CFRunLoop callback
        let mut pressed = match pressed_clone.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                warn!("Hotkey: pressed_keys lock contention, skipping event");
                return;
            }
        };

        let was_recording = recording_clone.load(Ordering::SeqCst);

        if is_press {
            pressed.insert(key);
            debug!("Key pressed: {:?}", key);

            // ESC cancels recording
            if key == Key::Escape && was_recording {
                info!("ESC pressed - cancelling recording");
                recording_clone.store(false, Ordering::SeqCst);
                start_clone.store(0, Ordering::SeqCst);
                let _ = event_tx.send(AppEvent::CancelRecording);
                return;
            }
        } else {
            pressed.remove(&key);
            debug!("Key released: {:?}", key);
        }

        // Read config with try_lock; skip event if contended
        let hotkey_cfg = match config_clone.try_lock() {
            Ok(cfg) => cfg.hotkey.clone(),
            Err(_) => {
                warn!("Hotkey: config lock contention, skipping event");
                return;
            }
        };

        // Drop pressed lock before sending events (which may block briefly)
        let combo_active = is_hotkey_active(&pressed, &hotkey_cfg);
        drop(pressed);

        if combo_active && !was_recording {
            info!("Recording started ({} pressed)", format_hotkey(&hotkey_cfg));
            recording_clone.store(true, Ordering::SeqCst);
            let elapsed_ms = epoch.elapsed().as_millis() as u64;
            start_clone.store(elapsed_ms, Ordering::SeqCst);
            let _ = event_tx.send(AppEvent::StartRecording);
        } else if !combo_active && was_recording {
            let start_ms = start_clone.swap(0, Ordering::SeqCst);
            let was_quick_tap = if start_ms > 0 {
                let now_ms = epoch.elapsed().as_millis() as u64;
                now_ms.saturating_sub(start_ms) < QUICK_TAP_THRESHOLD_MS
            } else {
                false
            };

            recording_clone.store(false, Ordering::SeqCst);

            if was_quick_tap {
                info!("Quick tap detected - cancelling recording");
                let _ = event_tx.send(AppEvent::CancelRecording);
            } else {
                info!(
                    "Recording stopped ({} released)",
                    format_hotkey(&hotkey_cfg)
                );
                let _ = event_tx.send(AppEvent::StopRecording);
            }
        }
    };

    // rdev::listen is blocking
    if let Err(error) = listen(callback) {
        error!("rdev listen error: {:?}", error);
        anyhow::bail!(
            "Global hotkey listener failed. On macOS, grant Accessibility permission \
             in System Settings > Privacy & Security > Accessibility."
        );
    }

    Ok(())
}
