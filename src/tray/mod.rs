//! System tray and clipboard history UI
//!
//! Uses ksni for system tray indicator (works with GNOME's AppIndicator extension)

use crate::{AppEvent, AppState, ClipboardEntry};
use anyhow::Result;
use crossbeam_channel::Sender;
use ksni::{menu::*, ToolTip, Tray, TrayService};
use std::sync::{Arc, Mutex};
use tracing::info;

/// System tray icon and menu
struct OswispaTray {
    event_tx: Sender<AppEvent>,
    state: Arc<Mutex<AppState>>,
}

impl Tray for OswispaTray {
    fn id(&self) -> String {
        "oswispa".to_string()
    }

    fn title(&self) -> String {
        let state = self.state.lock().unwrap();
        if state.is_recording {
            "OSWispa [RECORDING]".to_string()
        } else {
            "OSWispa".to_string()
        }
    }

    fn icon_name(&self) -> String {
        let state = self.state.lock().unwrap();
        if state.is_recording {
            // Mic icon when recording
            "audio-input-microphone-high".to_string()
        } else {
            // Smiley face when idle
            "face-smile".to_string()
        }
    }

    fn tool_tip(&self) -> ToolTip {
        let state = self.state.lock().unwrap();
        let (status, icon) = if state.is_recording {
            ("RECORDING - Release Ctrl+Super to transcribe, ESC to cancel", "audio-input-microphone-high")
        } else {
            ("Ready - Hold Ctrl+Super to record", "face-smile")
        };

        ToolTip {
            icon_name: icon.to_string(),
            icon_pixmap: Vec::new(),
            title: "OSWispa - Voice to Text".to_string(),
            description: status.to_string(),
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let state = self.state.lock().unwrap();
        let history_count = state.clipboard_history.len();
        let is_recording = state.is_recording;

        let status_label = if is_recording {
            ">> RECORDING... (ESC to cancel)".to_string()
        } else {
            "OSWispa - Voice to Text".to_string()
        };

        let mut menu = vec![
            StandardItem {
                label: status_label,
                enabled: false,
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: format!("Clipboard History ({} items)", history_count),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.event_tx.send(AppEvent::ShowHistory);
                }),
                ..Default::default()
            }
            .into(),
        ];

        // Add recent items to menu (last 5)
        if !state.clipboard_history.is_empty() {
            menu.push(MenuItem::Separator);

            for (i, entry) in state.clipboard_history.iter().take(5).enumerate() {
                let preview = if entry.text.chars().count() > 40 {
                    format!("{}...", entry.text.chars().take(40).collect::<String>())
                } else {
                    entry.text.clone()
                };
                let preview = preview.replace('\n', " ");

                let text = entry.text.clone();
                menu.push(
                    StandardItem {
                        label: format!("{}. {}", i + 1, preview),
                        activate: Box::new(move |_tray: &mut Self| {
                            // Copy this item back to clipboard
                            if let Err(e) = crate::input::copy_to_clipboard(&text) {
                                tracing::warn!("Failed to copy: {}", e);
                            }
                        }),
                        ..Default::default()
                    }
                    .into(),
                );
            }
        }

        menu.extend(vec![
            MenuItem::Separator,
            StandardItem {
                label: "Settings...".to_string(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.event_tx.send(AppEvent::OpenSettings);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Help".to_string(),
                activate: Box::new(|_| {
                    let _ = std::process::Command::new("xdg-open")
                        .arg("https://github.com/oswispa/oswispa")
                        .spawn();
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "Quit".to_string(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.event_tx.send(AppEvent::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]);

        menu
    }
}

/// Run the system tray
pub fn run_tray(event_tx: Sender<AppEvent>, state: Arc<Mutex<AppState>>) -> Result<()> {
    info!("Starting system tray indicator...");

    let tray = OswispaTray { event_tx, state };

    let service = TrayService::new(tray);

    // Run the tray service (blocks)
    match service.run() {
        Ok(()) => {
            info!("System tray service exited normally");
            Ok(())
        }
        Err(e) => {
            tracing::error!(
                "System tray failed: {}. Make sure AppIndicator extension is enabled:\n\
                gnome-extensions enable ubuntu-appindicators@ubuntu.com", 
                e
            );
            Err(anyhow::anyhow!("Tray service failed: {}", e))
        }
    }
}

/// GTK4 Clipboard History Window (optional, for ShowHistory event)
#[allow(dead_code)]
pub fn show_history_window(history: &[ClipboardEntry]) {
    // This would require GTK4 initialization
    // For now, we show history in the tray menu
    info!("History window requested with {} items", history.len());
}
