//! System tray and clipboard history UI
//!
//! Uses ksni for system tray indicator (works with GNOME's AppIndicator extension)

use crate::state::{AppPhase, DeliveryOutcome};
use crate::{AppEvent, AppState, Config};
use anyhow::Result;
use crossbeam_channel::Sender;
use ksni::{menu::*, ToolTip, Tray, TrayService};
use std::sync::{Arc, Mutex, RwLock};
use tracing::info;

/// System tray icon and menu
struct OswispaTray {
    event_tx: Sender<AppEvent>,
    state: Arc<Mutex<AppState>>,
    config: Arc<RwLock<Config>>,
}

fn phase_title(phase: &AppPhase) -> &'static str {
    match phase {
        AppPhase::Booting => "OSWispa [STARTING]",
        AppPhase::Ready => "OSWispa",
        AppPhase::Arming => "OSWispa [STARTING MICROPHONE]",
        AppPhase::Listening { .. } => "OSWispa [LISTENING]",
        AppPhase::Processing => "OSWispa [PROCESSING]",
        AppPhase::Delivering => "OSWispa [DELIVERING]",
        AppPhase::Delivered(DeliveryOutcome::Inserted) => "OSWispa [INSERTED]",
        AppPhase::Delivered(DeliveryOutcome::CopiedOnly) => "OSWispa [COPIED]",
        AppPhase::Delivered(DeliveryOutcome::Failed) => "OSWispa [DELIVERY FAILED]",
        AppPhase::Cancelled => "OSWispa [CANCELLED]",
        AppPhase::NeedsAttention => "OSWispa [NEEDS ATTENTION]",
    }
}

fn phase_icon(phase: &AppPhase) -> &'static str {
    match phase {
        AppPhase::Booting | AppPhase::Processing => "process-working",
        AppPhase::Ready => "face-smile",
        AppPhase::Arming => "audio-input-microphone-low",
        AppPhase::Listening { .. } => "audio-input-microphone-high",
        AppPhase::Delivering => "edit-paste",
        AppPhase::Delivered(DeliveryOutcome::Inserted) => "emblem-ok",
        AppPhase::Delivered(DeliveryOutcome::CopiedOnly) => "edit-copy",
        AppPhase::Delivered(DeliveryOutcome::Failed) | AppPhase::NeedsAttention => "dialog-error",
        AppPhase::Cancelled => "process-stop",
    }
}

fn phase_description(phase: &AppPhase, hotkey: &str) -> String {
    match phase {
        AppPhase::Booting => "Starting OSWispa…".to_string(),
        AppPhase::Ready => format!("Ready — hold {} to record", hotkey),
        AppPhase::Arming => "Starting microphone…".to_string(),
        AppPhase::Listening { .. } => {
            format!(
                "Listening — release {} to transcribe, Esc to cancel",
                hotkey
            )
        }
        AppPhase::Processing => "Turning speech into text…".to_string(),
        AppPhase::Delivering => "Delivering text…".to_string(),
        AppPhase::Delivered(DeliveryOutcome::Inserted) => "Text inserted".to_string(),
        AppPhase::Delivered(DeliveryOutcome::CopiedOnly) => "Text copied to clipboard".to_string(),
        AppPhase::Delivered(DeliveryOutcome::Failed) => "Text delivery failed".to_string(),
        AppPhase::Cancelled => format!("Cancelled — hold {} to try again", hotkey),
        AppPhase::NeedsAttention => "OSWispa needs attention — check the logs".to_string(),
    }
}

impl Tray for OswispaTray {
    fn id(&self) -> String {
        "oswispa".to_string()
    }

    fn title(&self) -> String {
        let state = self.state.lock().unwrap();
        phase_title(&state.phase).to_string()
    }

    fn icon_name(&self) -> String {
        let state = self.state.lock().unwrap();
        phase_icon(&state.phase).to_string()
    }

    fn tool_tip(&self) -> ToolTip {
        let state = self.state.lock().unwrap();
        let hotkey = crate::format_hotkey(&self.config.read().unwrap().hotkey);
        let status = phase_description(&state.phase, &hotkey);
        let icon = phase_icon(&state.phase);

        ToolTip {
            icon_name: icon.to_string(),
            icon_pixmap: Vec::new(),
            title: "OSWispa - Voice to Text".to_string(),
            description: status,
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let state = self.state.lock().unwrap();
        let history_count = state.clipboard_history.len();
        let hotkey = crate::format_hotkey(&self.config.read().unwrap().hotkey);
        let status_label = phase_description(&state.phase, &hotkey);

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
                enabled: false,
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
                        .arg("https://github.com/tylerbuilds/OSWispa")
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
pub fn run_tray(
    event_tx: Sender<AppEvent>,
    state: Arc<Mutex<AppState>>,
    config: Arc<RwLock<Config>>,
) -> Result<()> {
    info!("Starting system tray indicator...");

    let tray = OswispaTray {
        event_tx,
        state,
        config,
    };

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tray_copy_distinguishes_listening_processing_and_delivery() {
        let listening = AppPhase::Listening {
            device_name: "Private device name".to_string(),
        };
        assert!(phase_description(&listening, "Ctrl+Super").starts_with("Listening"));
        assert_eq!(
            phase_description(&AppPhase::Processing, "Ctrl+Super"),
            "Turning speech into text…"
        );
        assert_eq!(
            phase_description(
                &AppPhase::Delivered(DeliveryOutcome::CopiedOnly),
                "Ctrl+Super"
            ),
            "Text copied to clipboard"
        );
    }

    #[test]
    fn tray_status_never_exposes_device_name() {
        let private_device = "Tyler's private studio microphone";
        let phase = AppPhase::Listening {
            device_name: private_device.to_string(),
        };
        assert!(!phase_title(&phase).contains(private_device));
        assert!(!phase_description(&phase, "Ctrl+Super").contains(private_device));
    }
}
