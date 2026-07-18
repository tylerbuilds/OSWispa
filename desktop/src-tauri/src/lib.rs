//! Thin, transcript-free desktop boundary around the reusable MorpheOS Voice engine.

use oswispa::{DeliveryOutcome, EnginePhase};
use serde::Serialize;

/// The complete routine lifecycle payload exposed to the webview.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct UiLifecycle {
    pub state: &'static str,
}

impl UiLifecycle {
    pub const fn from_phase(phase: EnginePhase) -> Self {
        let state = match phase {
            EnginePhase::Booting => "booting",
            EnginePhase::Ready => "ready",
            EnginePhase::Arming => "arming",
            EnginePhase::Listening => "listening",
            EnginePhase::Processing => "processing",
            EnginePhase::Delivering => "delivering",
            EnginePhase::Delivered(DeliveryOutcome::Inserted) => "inserted",
            EnginePhase::Delivered(DeliveryOutcome::CopiedOnly) => "copied",
            EnginePhase::Cancelled => "cancelled",
            EnginePhase::Delivered(DeliveryOutcome::Failed)
            | EnginePhase::NeedsAttention
            | EnginePhase::Stopped => "needs_attention",
        };

        Self { state }
    }
}

#[cfg(feature = "desktop-runtime")]
mod desktop {
    use super::UiLifecycle;
    use oswispa::{EngineCommand, EngineEvent, EngineHandle, EngineOptions, EnginePhase};
    use std::error::Error;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Arc, Mutex, MutexGuard};
    use std::time::Duration;
    use tauri::image::Image;
    use tauri::menu::{Menu, MenuItem};
    use tauri::tray::TrayIconBuilder;
    use tauri::{AppHandle, Emitter, Manager, WebviewWindow, WindowEvent};

    const MENU_OPEN: &str = "open_oswispa";
    const MENU_HISTORY: &str = "history";
    const MENU_SIGNAL: &str = "show_signal";
    const MENU_START: &str = "start";
    const MENU_STOP: &str = "stop";
    const MENU_CANCEL: &str = "cancel";
    const MENU_QUIT: &str = "quit";
    const RECEIPT_VISIBILITY: Duration = Duration::from_millis(1_600);

    struct ShellState {
        engine: Mutex<Option<EngineHandle>>,
        last_lifecycle: Mutex<UiLifecycle>,
        lifecycle_generation: AtomicU64,
    }

    impl ShellState {
        fn new() -> Self {
            Self {
                engine: Mutex::new(None),
                last_lifecycle: Mutex::new(UiLifecycle::from_phase(EnginePhase::Booting)),
                lifecycle_generation: AtomicU64::new(0),
            }
        }

        fn engine(&self) -> MutexGuard<'_, Option<EngineHandle>> {
            self.engine
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
        }

        fn last_lifecycle(&self) -> MutexGuard<'_, UiLifecycle> {
            self.last_lifecycle
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
        }
    }

    pub fn run() -> Result<(), Box<dyn Error>> {
        let shared = Arc::new(ShellState::new());
        let setup_state = Arc::clone(&shared);

        // The official singleton must be registered before setup so a second
        // process cannot start a second engine or global hotkey listener.
        let builder = tauri::Builder::default()
            .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
                let _ = show_focused(app, "settings");
            }))
            .setup(move |app| {
                build_tray(app.handle(), Arc::clone(&setup_state))?;
                spawn_engine(app.handle().clone(), Arc::clone(&setup_state));
                Ok(())
            })
            .on_window_event(|window, event| {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            });

        let app = builder.build(tauri::generate_context!())?;
        let exit_code = app.run_return(|_app, _event| {});

        if let Some(engine) = shared.engine().take() {
            let _ = engine.shutdown();
        }

        std::process::exit(exit_code);
    }

    fn spawn_engine(app: AppHandle, shared: Arc<ShellState>) {
        let thread_app = app.clone();
        let thread_state = Arc::clone(&shared);
        let spawn_result = std::thread::Builder::new()
            .name("morpheos-voice-shell-engine".to_string())
            .spawn(
                move || match EngineHandle::start(EngineOptions::embedded()) {
                    Ok(engine) => {
                        let events = engine.events();
                        thread_state.engine().replace(engine);

                        for event in events {
                            let EngineEvent::PhaseChanged(phase) = event;
                            publish_lifecycle(&thread_app, &thread_state, phase);
                        }

                        if let Some(engine) = thread_state.engine().take() {
                            let _ = engine.wait();
                        }
                    }
                    Err(_) => {
                        publish_lifecycle(&thread_app, &thread_state, EnginePhase::NeedsAttention)
                    }
                },
            );

        if spawn_result.is_err() {
            publish_lifecycle(&app, &shared, EnginePhase::NeedsAttention);
        }
    }

    fn publish_lifecycle(app: &AppHandle, shared: &Arc<ShellState>, phase: EnginePhase) {
        let payload = UiLifecycle::from_phase(phase);
        *shared.last_lifecycle() = payload;
        let generation = shared.lifecycle_generation.fetch_add(1, Ordering::SeqCst) + 1;

        if let Some(signal) = app.get_webview_window("signal") {
            let _ = signal.emit("lifecycle", payload);
            match phase {
                EnginePhase::Ready => {
                    let _ = signal.hide();
                }
                EnginePhase::Cancelled | EnginePhase::Delivered(_) => {
                    show_without_focus(&signal);
                    schedule_signal_hide(app.clone(), Arc::clone(shared), generation);
                }
                _ => show_without_focus(&signal),
            }
        }
    }

    fn schedule_signal_hide(app: AppHandle, shared: Arc<ShellState>, generation: u64) {
        std::thread::spawn(move || {
            std::thread::sleep(RECEIPT_VISIBILITY);
            if shared.lifecycle_generation.load(Ordering::SeqCst) != generation {
                return;
            }
            if let Some(signal) = app.get_webview_window("signal") {
                let _ = signal.hide();
            }
        });
    }

    fn request_engine(app: &AppHandle, shared: &Arc<ShellState>, command: EngineCommand) {
        let result = shared
            .engine()
            .as_ref()
            .ok_or(())
            .and_then(|engine| engine.command(command).map_err(|_| ()));

        if result.is_err() {
            publish_lifecycle(app, shared, EnginePhase::NeedsAttention);
        }
    }

    fn build_tray(app: &AppHandle, shared: Arc<ShellState>) -> tauri::Result<()> {
        let open = MenuItem::with_id(app, MENU_OPEN, "Open MorpheOS Voice", true, None::<&str>)?;
        let history = MenuItem::with_id(app, MENU_HISTORY, "History", true, None::<&str>)?;
        let signal = MenuItem::with_id(app, MENU_SIGNAL, "Show Signal", true, None::<&str>)?;
        let start = MenuItem::with_id(app, MENU_START, "Start", true, None::<&str>)?;
        let stop = MenuItem::with_id(app, MENU_STOP, "Stop", true, None::<&str>)?;
        let cancel = MenuItem::with_id(app, MENU_CANCEL, "Cancel", true, None::<&str>)?;
        let quit = MenuItem::with_id(app, MENU_QUIT, "Quit", true, None::<&str>)?;
        let menu = Menu::with_items(
            app,
            &[&open, &history, &signal, &start, &stop, &cancel, &quit],
        )?;

        TrayIconBuilder::with_id("oswispa")
            .icon(signal_icon())
            .icon_as_template(cfg!(target_os = "macos"))
            .tooltip("MorpheOS Voice — voice typing")
            .menu(&menu)
            .on_menu_event(move |app, event| match event.id().as_ref() {
                MENU_OPEN => {
                    let _ = show_focused(app, "settings");
                }
                MENU_HISTORY => {
                    let _ = show_focused(app, "history");
                }
                MENU_SIGNAL => {
                    if let Some(window) = app.get_webview_window("signal") {
                        let _ = window.emit("lifecycle", *shared.last_lifecycle());
                        show_without_focus(&window);
                    }
                }
                MENU_START => request_engine(app, &shared, EngineCommand::Start),
                MENU_STOP => request_engine(app, &shared, EngineCommand::Stop),
                MENU_CANCEL => request_engine(app, &shared, EngineCommand::Cancel),
                MENU_QUIT => app.exit(0),
                _ => {}
            })
            .build(app)?;

        Ok(())
    }

    fn show_focused(app: &AppHandle, label: &str) -> tauri::Result<()> {
        if let Some(window) = app.get_webview_window(label) {
            window.show()?;
            window.set_focus()?;
        }
        Ok(())
    }

    fn show_without_focus(window: &WebviewWindow) {
        // `show` does not request focus; the Signal window is also configured
        // as non-focusable so dictation does not steal the target application.
        let _ = window.show();
    }

    fn signal_icon() -> Image<'static> {
        const SIZE: usize = 32;
        let mut rgba = vec![0_u8; SIZE * SIZE * 4];

        for y in 0..SIZE {
            for x in 0..SIZE {
                let left_stem = (3..=5).contains(&x) && (7..=24).contains(&y);
                let right_stem = (14..=16).contains(&x) && (7..=24).contains(&y);
                let diagonal_offset = y.saturating_sub(7) / 2;
                let left_diagonal = (7..=15).contains(&y)
                    && (5 + diagonal_offset..=6 + diagonal_offset).contains(&x);
                let right_diagonal = (7..=15).contains(&y)
                    && (13_usize.saturating_sub(diagonal_offset)
                        ..=14_usize.saturating_sub(diagonal_offset))
                        .contains(&x);
                let voice_bar = ((20..=21).contains(&x) && (11..=20).contains(&y))
                    || ((24..=25).contains(&x) && (7..=24).contains(&y))
                    || ((28..=29).contains(&x) && (13..=18).contains(&y));
                let cursor = (19..=30).contains(&x) && (26..=27).contains(&y);

                if left_stem || right_stem || left_diagonal || right_diagonal || voice_bar || cursor
                {
                    let offset = (y * SIZE + x) * 4;
                    rgba[offset..offset + 4].copy_from_slice(&[185, 242, 124, 255]);
                }
            }
        }

        Image::new_owned(rgba, SIZE as u32, SIZE as u32)
    }
}

#[cfg(feature = "desktop-runtime")]
pub use desktop::run;

#[cfg(not(feature = "desktop-runtime"))]
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    Err("desktop-runtime feature is disabled".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_projection_is_transcript_free_and_bounded() {
        let cases = [
            (EnginePhase::Booting, "booting"),
            (EnginePhase::Arming, "arming"),
            (EnginePhase::Listening, "listening"),
            (EnginePhase::Processing, "processing"),
            (EnginePhase::Delivering, "delivering"),
            (
                EnginePhase::Delivered(DeliveryOutcome::Inserted),
                "inserted",
            ),
            (
                EnginePhase::Delivered(DeliveryOutcome::CopiedOnly),
                "copied",
            ),
            (
                EnginePhase::Delivered(DeliveryOutcome::Failed),
                "needs_attention",
            ),
            (EnginePhase::Cancelled, "cancelled"),
        ];

        for (phase, expected) in cases {
            let payload = serde_json::to_value(UiLifecycle::from_phase(phase)).unwrap();
            assert_eq!(payload, serde_json::json!({ "state": expected }));
        }
    }

    #[test]
    fn singleton_registration_precedes_setup_and_engine_start() {
        let source = include_str!("lib.rs");
        let singleton = source
            .find(".plugin(tauri_plugin_single_instance::init")
            .unwrap();
        let setup = source.find(".setup(move |app|").unwrap();
        let engine = source.find("spawn_engine(app.handle()").unwrap();

        assert!(singleton < setup);
        assert!(setup < engine);
    }
}
