//! GTK4 Settings Dialog Implementation
//!
//! Full settings UI with tabs for General, Hotkey, Models, and Backend
//! configuration.

use crate::models::{self, ModelBenchmark, ModelInfo, AVAILABLE_MODELS};
use crate::{
    clear_remote_api_key, get_config_dir, get_remote_api_key, set_remote_api_key, AppEvent, Config,
    HotkeyConfig, TranscriptionBackend,
};
use crossbeam_channel::Sender;
use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box as GtkBox, Button, CheckButton, ComboBoxText, Entry, Grid,
    Label, Notebook, Orientation, ScrolledWindow, Separator, SpinButton,
};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use tracing::{error, info, warn};

/// Show the settings dialog
pub fn show_settings_dialog(config: &Arc<RwLock<Config>>, event_tx: Sender<AppEvent>) {
    let config = config.clone();
    let event_tx = event_tx.clone();

    std::thread::spawn(move || {
        if let Err(e) = gtk4::init() {
            error!("Failed to initialize GTK: {}", e);
            return;
        }

        let app = Application::builder()
            .application_id(format!("com.oswispa.settings.{}", std::process::id()))
            .build();

        let config_state = config.clone();
        let event_tx_clone = event_tx.clone();
        app.connect_activate(move |app| {
            let snapshot = config_state.read().unwrap().clone();
            build_settings_window(app, &snapshot, config_state.clone(), event_tx_clone.clone());
        });

        app.run_with_args::<String>(&[]);
    });
}

fn build_settings_window(
    app: &Application,
    config: &Config,
    config_state: Arc<RwLock<Config>>,
    event_tx: Sender<AppEvent>,
) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("OSWispa Settings")
        .default_width(640)
        .default_height(540)
        .build();

    let notebook = Notebook::new();

    notebook.append_page(
        &create_general_tab(config, config_state.clone(), event_tx.clone()),
        Some(&Label::new(Some("General"))),
    );
    notebook.append_page(
        &create_hotkey_tab(config, config_state.clone(), event_tx.clone()),
        Some(&Label::new(Some("Hotkey"))),
    );
    notebook.append_page(
        &create_models_tab(config, config_state.clone(), event_tx.clone()),
        Some(&Label::new(Some("Models"))),
    );
    notebook.append_page(
        &create_backend_tab(config, config_state, event_tx.clone()),
        Some(&Label::new(Some("Backend"))),
    );

    window.set_child(Some(&notebook));
    window.present();
}

/// Create General settings tab
fn create_general_tab(
    config: &Config,
    config_state: Arc<RwLock<Config>>,
    event_tx: Sender<AppEvent>,
) -> GtkBox {
    let vbox = GtkBox::new(Orientation::Vertical, 12);
    vbox.set_margin_top(20);
    vbox.set_margin_bottom(20);
    vbox.set_margin_start(20);
    vbox.set_margin_end(20);

    let title = Label::new(Some("General Settings"));
    title.add_css_class("title-2");
    vbox.append(&title);

    vbox.append(&Separator::new(Orientation::Horizontal));

    let audio_check = CheckButton::with_label("Audio feedback (tones for recording events)");
    audio_check.set_active(config.audio_feedback);
    vbox.append(&audio_check);

    let paste_check = CheckButton::with_label("Auto-paste transcribed text");
    paste_check.set_active(config.auto_paste);
    vbox.append(&paste_check);

    let notify_check = CheckButton::with_label("Show desktop notifications");
    notify_check.set_active(config.notification_enabled);
    vbox.append(&notify_check);

    let punct_check = CheckButton::with_label("Punctuation commands (say \"period\" for \".\")");
    punct_check.set_active(config.punctuation_commands);
    vbox.append(&punct_check);

    let lang_box = GtkBox::new(Orientation::Horizontal, 8);
    lang_box.append(&Label::new(Some("Language:")));

    let lang_combo = ComboBoxText::new();
    let languages = [
        ("en", "English"),
        ("es", "Spanish"),
        ("de", "German"),
        ("fr", "French"),
        ("it", "Italian"),
        ("pt", "Portuguese"),
        ("nl", "Dutch"),
        ("pl", "Polish"),
        ("ru", "Russian"),
        ("zh", "Chinese"),
        ("ja", "Japanese"),
        ("ko", "Korean"),
        ("auto", "Auto-detect"),
    ];
    for (code, name) in languages {
        lang_combo.append(Some(code), name);
    }
    lang_combo.set_active_id(Some(&config.language));
    lang_box.append(&lang_combo);
    vbox.append(&lang_box);

    let translate_check = CheckButton::with_label("Translate to English (for non-English speech)");
    translate_check.set_active(config.translate_to_english);
    vbox.append(&translate_check);

    let spacer = GtkBox::new(Orientation::Vertical, 0);
    spacer.set_vexpand(true);
    vbox.append(&spacer);

    let save_btn = Button::with_label("Save Settings");
    save_btn.add_css_class("suggested-action");

    let config_state_clone = config_state.clone();
    let event_tx_clone = event_tx.clone();
    save_btn.connect_clicked(move |_| {
        let mut new_config = config_state_clone.read().unwrap().clone();
        new_config.audio_feedback = audio_check.is_active();
        new_config.auto_paste = paste_check.is_active();
        new_config.notification_enabled = notify_check.is_active();
        new_config.punctuation_commands = punct_check.is_active();
        new_config.translate_to_english = translate_check.is_active();

        if let Some(lang) = lang_combo.active_id() {
            new_config.language = lang.to_string();
        }

        if let Err(e) = save_config(&new_config) {
            error!("Failed to save config: {}", e);
        } else {
            if let Ok(mut guard) = config_state_clone.write() {
                *guard = new_config;
            }
            info!("Settings saved successfully");
            let _ = event_tx_clone.send(AppEvent::ReloadConfig);
        }
    });
    vbox.append(&save_btn);

    vbox
}

fn hotkey_trigger_options() -> &'static [(&'static str, &'static str)] {
    &[
        ("None (modifiers only)", "none"),
        ("Space", "space"),
        ("Tab", "tab"),
        ("Enter", "enter"),
        ("Backspace", "backspace"),
        ("CapsLock", "capslock"),
        ("` (grave)", "grave"),
        ("F1", "f1"),
        ("F2", "f2"),
        ("F3", "f3"),
        ("F4", "f4"),
        ("F5", "f5"),
        ("F6", "f6"),
        ("F7", "f7"),
        ("F8", "f8"),
        ("F9", "f9"),
        ("F10", "f10"),
        ("F11", "f11"),
        ("F12", "f12"),
        ("A", "a"),
        ("B", "b"),
        ("C", "c"),
        ("D", "d"),
        ("E", "e"),
        ("F", "f"),
        ("G", "g"),
        ("H", "h"),
        ("I", "i"),
        ("J", "j"),
        ("K", "k"),
        ("L", "l"),
        ("M", "m"),
        ("N", "n"),
        ("O", "o"),
        ("P", "p"),
        ("Q", "q"),
        ("R", "r"),
        ("S", "s"),
        ("T", "t"),
        ("U", "u"),
        ("V", "v"),
        ("W", "w"),
        ("X", "x"),
        ("Y", "y"),
        ("Z", "z"),
    ]
}

fn selected_trigger_key(combo: &ComboBoxText) -> Option<String> {
    let id = combo.active_id()?;
    let trigger = id.as_str().trim().to_string();
    if trigger == "none" || trigger.is_empty() {
        None
    } else {
        Some(trigger)
    }
}

fn hotkey_has_activation_key(hotkey: &HotkeyConfig) -> bool {
    hotkey.ctrl
        || hotkey.alt
        || hotkey.shift
        || hotkey.super_key
        || hotkey
            .trigger_key
            .as_ref()
            .map(|k| !k.trim().is_empty())
            .unwrap_or(false)
}

/// Create Hotkey settings tab
fn create_hotkey_tab(
    config: &Config,
    config_state: Arc<RwLock<Config>>,
    event_tx: Sender<AppEvent>,
) -> GtkBox {
    let vbox = GtkBox::new(Orientation::Vertical, 12);
    vbox.set_margin_top(20);
    vbox.set_margin_bottom(20);
    vbox.set_margin_start(20);
    vbox.set_margin_end(20);

    let title = Label::new(Some("Hotkey Configuration"));
    title.add_css_class("title-2");
    vbox.append(&title);

    vbox.append(&Separator::new(Orientation::Horizontal));

    let desc = Label::new(Some(
        "Pick modifier keys and an optional trigger key.\nHold the full combination to record, release to transcribe.",
    ));
    desc.set_wrap(true);
    vbox.append(&desc);

    let grid = Grid::new();
    grid.set_row_spacing(8);
    grid.set_column_spacing(16);
    grid.set_margin_top(12);

    let ctrl_check = CheckButton::with_label("Ctrl");
    ctrl_check.set_active(config.hotkey.ctrl);
    grid.attach(&ctrl_check, 0, 0, 1, 1);

    let alt_check = CheckButton::with_label("Alt");
    alt_check.set_active(config.hotkey.alt);
    grid.attach(&alt_check, 1, 0, 1, 1);

    let shift_check = CheckButton::with_label("Shift");
    shift_check.set_active(config.hotkey.shift);
    grid.attach(&shift_check, 0, 1, 1, 1);

    let super_check = CheckButton::with_label("Super (Windows key)");
    super_check.set_active(config.hotkey.super_key);
    grid.attach(&super_check, 1, 1, 1, 1);

    vbox.append(&grid);

    let trigger_box = GtkBox::new(Orientation::Horizontal, 8);
    trigger_box.set_margin_top(8);
    trigger_box.append(&Label::new(Some("Trigger key:")));

    let trigger_combo = ComboBoxText::new();
    for (label, id) in hotkey_trigger_options() {
        trigger_combo.append(Some(id), label);
    }
    trigger_combo.set_active_id(config.hotkey.trigger_key.as_deref().or(Some("none")));
    trigger_box.append(&trigger_combo);
    vbox.append(&trigger_box);

    let preview_box = GtkBox::new(Orientation::Horizontal, 8);
    preview_box.set_margin_top(12);
    preview_box.append(&Label::new(Some("Current hotkey:")));
    let preview_label = Label::new(Some(&format_hotkey(&config.hotkey)));
    preview_label.add_css_class("title-3");
    preview_box.append(&preview_label);
    vbox.append(&preview_box);

    let preview_ref = preview_label.clone();
    let update_preview: Rc<dyn Fn()> = Rc::new({
        let ctrl = ctrl_check.clone();
        let alt = alt_check.clone();
        let shift = shift_check.clone();
        let super_key = super_check.clone();
        let trigger = trigger_combo.clone();
        move || {
            let hotkey = HotkeyConfig {
                ctrl: ctrl.is_active(),
                alt: alt.is_active(),
                shift: shift.is_active(),
                super_key: super_key.is_active(),
                trigger_key: selected_trigger_key(&trigger),
            };
            preview_ref.set_text(&format_hotkey(&hotkey));
        }
    });

    let update_clone = update_preview.clone();
    ctrl_check.connect_toggled(move |_| update_clone());
    let update_clone = update_preview.clone();
    alt_check.connect_toggled(move |_| update_clone());
    let update_clone = update_preview.clone();
    shift_check.connect_toggled(move |_| update_clone());
    let update_clone = update_preview.clone();
    super_check.connect_toggled(move |_| update_clone());
    let update_clone = update_preview.clone();
    trigger_combo.connect_changed(move |_| update_clone());

    let spacer = GtkBox::new(Orientation::Vertical, 0);
    spacer.set_vexpand(true);
    vbox.append(&spacer);

    let warning = Label::new(Some("Hotkey changes take effect immediately."));
    warning.add_css_class("dim-label");
    vbox.append(&warning);

    let save_btn = Button::with_label("Apply Hotkey");
    save_btn.add_css_class("suggested-action");

    let config_state_clone = config_state.clone();
    let event_tx_clone = event_tx.clone();
    save_btn.connect_clicked(move |_| {
        let mut new_config = config_state_clone.read().unwrap().clone();
        new_config.hotkey = HotkeyConfig {
            ctrl: ctrl_check.is_active(),
            alt: alt_check.is_active(),
            shift: shift_check.is_active(),
            super_key: super_check.is_active(),
            trigger_key: selected_trigger_key(&trigger_combo),
        };

        if !hotkey_has_activation_key(&new_config.hotkey) {
            error!("Hotkey rejected: choose at least one modifier or trigger key");
            return;
        }

        if let Err(e) = save_config(&new_config) {
            error!("Failed to save hotkey config: {}", e);
        } else {
            if let Ok(mut guard) = config_state_clone.write() {
                *guard = new_config.clone();
            }
            info!("Hotkey updated: {}", format_hotkey(&new_config.hotkey));
            let _ = event_tx_clone.send(AppEvent::ReloadConfig);
        }
    });
    vbox.append(&save_btn);

    vbox
}

/// Create Models settings tab
fn create_models_tab(
    config: &Config,
    config_state: Arc<RwLock<Config>>,
    event_tx: Sender<AppEvent>,
) -> GtkBox {
    let vbox = GtkBox::new(Orientation::Vertical, 12);
    vbox.set_margin_top(20);
    vbox.set_margin_bottom(20);
    vbox.set_margin_start(20);
    vbox.set_margin_end(20);

    let title = Label::new(Some("Whisper Models"));
    title.add_css_class("title-2");
    vbox.append(&title);

    vbox.append(&Separator::new(Orientation::Horizontal));

    let current_box = GtkBox::new(Orientation::Horizontal, 8);
    current_box.append(&Label::new(Some("Active model:")));
    let current_model = config
        .model_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Unknown".to_string());
    let current_label = Label::new(Some(&current_model));
    current_label.add_css_class("title-4");
    current_box.append(&current_label);
    vbox.append(&current_box);

    let profile_text = models::estimate_model_benchmark(&config.model_path)
        .map(|b| model_benchmark_text(&b))
        .unwrap_or_else(|_| "No benchmark available for active model".to_string());
    let profile_label = Label::new(Some(&profile_text));
    profile_label.add_css_class("dim-label");
    profile_label.set_wrap(true);
    profile_label.set_xalign(0.0);
    vbox.append(&profile_label);

    vbox.append(&Separator::new(Orientation::Horizontal));

    let import_title = Label::new(Some("Import local model file (.bin / .gguf)"));
    import_title.set_xalign(0.0);
    import_title.add_css_class("heading");
    vbox.append(&import_title);

    let import_row = GtkBox::new(Orientation::Horizontal, 8);
    let import_entry = Entry::new();
    import_entry.set_hexpand(true);
    import_entry.set_placeholder_text(Some("/absolute/path/to/model.bin"));
    import_row.append(&import_entry);

    let import_btn = Button::with_label("Import + Activate");
    let config_state_clone = config_state.clone();
    let event_tx_clone = event_tx.clone();
    import_btn.connect_clicked(move |_| {
        let raw = import_entry.text();
        let source = raw.trim();
        if source.is_empty() {
            error!("No model path provided for import");
            return;
        }

        match models::import_model_from_path(Path::new(source)) {
            Ok(imported_path) => {
                let mut new_config = config_state_clone.read().unwrap().clone();
                new_config.model_path = imported_path.clone();

                if let Err(e) = save_config(&new_config) {
                    error!("Failed to save imported model selection: {}", e);
                    return;
                }

                if let Ok(mut guard) = config_state_clone.write() {
                    *guard = new_config;
                }
                info!("Imported and activated model: {:?}", imported_path);
                let _ = event_tx_clone.send(AppEvent::ReloadConfig);
            }
            Err(e) => error!("Failed to import model from '{}': {}", source, e),
        }
    });
    import_row.append(&import_btn);
    vbox.append(&import_row);

    vbox.append(&Separator::new(Orientation::Horizontal));

    let bundled_label = Label::new(Some("Bundled downloads"));
    bundled_label.set_xalign(0.0);
    bundled_label.add_css_class("heading");
    vbox.append(&bundled_label);

    let scroll = ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_min_content_height(180);

    let models_box = GtkBox::new(Orientation::Vertical, 8);
    for model in AVAILABLE_MODELS {
        let row = create_model_row(model, config, config_state.clone(), event_tx.clone());
        models_box.append(&row);
    }
    scroll.set_child(Some(&models_box));
    vbox.append(&scroll);

    let custom_models = models::list_custom_models();
    if !custom_models.is_empty() {
        vbox.append(&Separator::new(Orientation::Horizontal));
        let custom_label = Label::new(Some("Custom local models"));
        custom_label.set_xalign(0.0);
        custom_label.add_css_class("heading");
        vbox.append(&custom_label);

        for model_path in custom_models {
            let row = create_custom_model_row(
                &model_path,
                config,
                config_state.clone(),
                event_tx.clone(),
            );
            vbox.append(&row);
        }
    }

    vbox
}

fn create_custom_model_row(
    model_path: &PathBuf,
    config: &Config,
    config_state: Arc<RwLock<Config>>,
    event_tx: Sender<AppEvent>,
) -> GtkBox {
    let row = GtkBox::new(Orientation::Horizontal, 12);
    row.set_margin_top(2);
    row.set_margin_bottom(2);

    let info_box = GtkBox::new(Orientation::Vertical, 2);
    info_box.set_hexpand(true);

    let file_name = model_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "custom-model".to_string());
    let name_label = Label::new(Some(&file_name));
    name_label.set_halign(gtk4::Align::Start);
    name_label.add_css_class("heading");
    info_box.append(&name_label);

    let bench = models::estimate_model_benchmark(model_path)
        .map(|b| model_benchmark_text(&b))
        .unwrap_or_else(|_| "Custom local model".to_string());
    let desc_label = Label::new(Some(&bench));
    desc_label.set_halign(gtk4::Align::Start);
    desc_label.add_css_class("dim-label");
    info_box.append(&desc_label);

    row.append(&info_box);

    let is_active = config.model_path == *model_path;
    if is_active {
        let label = Label::new(Some("✓ Active"));
        label.add_css_class("success");
        row.append(&label);
    } else {
        let use_btn = Button::with_label("Use");
        let config_state_clone = config_state.clone();
        let model_path_clone = model_path.clone();
        use_btn.connect_clicked(move |_| {
            let mut new_config = config_state_clone.read().unwrap().clone();
            new_config.model_path = model_path_clone.clone();
            if let Err(e) = save_config(&new_config) {
                error!("Failed to set active custom model: {}", e);
            } else {
                if let Ok(mut guard) = config_state_clone.write() {
                    *guard = new_config;
                }
                info!(
                    "Active model changed to custom path: {:?}",
                    model_path_clone
                );
                let _ = event_tx.send(AppEvent::ReloadConfig);
            }
        });
        row.append(&use_btn);
    }

    row
}

/// Create a row for a model in the list
fn create_model_row(
    model: &'static ModelInfo,
    config: &Config,
    config_state: Arc<RwLock<Config>>,
    event_tx: Sender<AppEvent>,
) -> GtkBox {
    let row = GtkBox::new(Orientation::Horizontal, 12);
    row.set_margin_top(4);
    row.set_margin_bottom(4);

    let info_box = GtkBox::new(Orientation::Vertical, 2);
    info_box.set_hexpand(true);

    let name_label = Label::new(Some(model.name));
    name_label.set_halign(gtk4::Align::Start);
    name_label.add_css_class("heading");
    info_box.append(&name_label);

    let desc_label = Label::new(Some(&format!(
        "{} • {}MB",
        model.description, model.size_mb
    )));
    desc_label.set_halign(gtk4::Align::Start);
    desc_label.add_css_class("dim-label");
    info_box.append(&desc_label);

    row.append(&info_box);

    let is_installed = models::is_model_installed(model);
    let is_active = config
        .model_path
        .file_name()
        .map(|n| n.to_string_lossy() == model.filename)
        .unwrap_or(false);

    if is_active {
        let label = Label::new(Some("✓ Active"));
        label.add_css_class("success");
        row.append(&label);
    } else if is_installed {
        let use_btn = Button::with_label("Use");
        let config_state_clone = config_state.clone();
        let model_filename = model.filename;
        use_btn.connect_clicked(move |_| {
            let mut new_config = config_state_clone.read().unwrap().clone();
            new_config.model_path = models::get_model_path(&ModelInfo {
                name: "",
                filename: model_filename,
                size_mb: 0,
                url: "",
                description: "",
            });
            if let Err(e) = save_config(&new_config) {
                error!("Failed to set active model: {}", e);
            } else {
                if let Ok(mut guard) = config_state_clone.write() {
                    *guard = new_config.clone();
                }
                info!("Active model changed to: {}", model_filename);
                let _ = event_tx.send(AppEvent::ReloadConfig);
            }
        });
        row.append(&use_btn);
    } else {
        let download_btn = Button::with_label(&format!("Download ({}MB)", model.size_mb));
        download_btn.connect_clicked(move |btn| {
            btn.set_sensitive(false);
            btn.set_label("Downloading...");

            let model_clone = model;
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    match models::download_model(model_clone, |downloaded, total| {
                        let percent = (downloaded as f64 / total as f64 * 100.0) as u32;
                        if percent % 10 == 0 {
                            info!("Download progress: {}%", percent);
                        }
                    })
                    .await
                    {
                        Ok(path) => info!("Model downloaded to {:?}", path),
                        Err(e) => error!("Download failed: {}", e),
                    }
                });
            });
        });
        row.append(&download_btn);
    }

    row
}

fn create_backend_tab(
    config: &Config,
    config_state: Arc<RwLock<Config>>,
    event_tx: Sender<AppEvent>,
) -> GtkBox {
    let vbox = GtkBox::new(Orientation::Vertical, 12);
    vbox.set_margin_top(20);
    vbox.set_margin_bottom(20);
    vbox.set_margin_start(20);
    vbox.set_margin_end(20);

    let title = Label::new(Some("Backend Settings"));
    title.add_css_class("title-2");
    vbox.append(&title);

    vbox.append(&Separator::new(Orientation::Horizontal));

    let privacy = Label::new(Some(
        "Local-first by default. Remote VPS mode is optional and only used when enabled.",
    ));
    privacy.set_wrap(true);
    privacy.set_xalign(0.0);
    vbox.append(&privacy);

    let mode_box = GtkBox::new(Orientation::Horizontal, 8);
    mode_box.append(&Label::new(Some("Backend mode:")));
    let backend_combo = ComboBoxText::new();
    backend_combo.append(Some("local"), "Local (Whisper.cpp)");
    backend_combo.append(Some("remote"), "Remote VPS (OpenAI-compatible)");
    match config.backend {
        TranscriptionBackend::Local => backend_combo.set_active_id(Some("local")),
        TranscriptionBackend::Remote => backend_combo.set_active_id(Some("remote")),
    };
    mode_box.append(&backend_combo);
    vbox.append(&mode_box);

    let endpoint_box = GtkBox::new(Orientation::Horizontal, 8);
    endpoint_box.append(&Label::new(Some("Endpoint URL:")));
    let endpoint_entry = Entry::new();
    endpoint_entry.set_hexpand(true);
    endpoint_entry
        .set_placeholder_text(Some("https://your-vps.example.com/v1/audio/transcriptions"));
    endpoint_entry.set_text(&config.remote_backend.endpoint);
    endpoint_box.append(&endpoint_entry);
    vbox.append(&endpoint_box);

    let model_box = GtkBox::new(Orientation::Horizontal, 8);
    model_box.append(&Label::new(Some("Remote model:")));
    let model_entry = Entry::new();
    model_entry.set_hexpand(true);
    model_entry.set_placeholder_text(Some("whisper-1"));
    model_entry.set_text(&config.remote_backend.model);
    model_box.append(&model_entry);
    vbox.append(&model_box);

    let timeout_box = GtkBox::new(Orientation::Horizontal, 8);
    timeout_box.append(&Label::new(Some("Timeout (ms):")));
    let timeout_spin = SpinButton::with_range(1_000.0, 120_000.0, 500.0);
    timeout_spin.set_value(config.remote_backend.timeout_ms as f64);
    timeout_box.append(&timeout_spin);
    vbox.append(&timeout_box);

    let insecure_check = CheckButton::with_label("Allow insecure HTTP endpoint (not recommended)");
    insecure_check.set_active(config.remote_backend.allow_insecure_http);
    vbox.append(&insecure_check);

    let env_box = GtkBox::new(Orientation::Horizontal, 8);
    env_box.append(&Label::new(Some("API key env var (optional):")));
    let env_entry = Entry::new();
    env_entry.set_hexpand(true);
    env_entry.set_placeholder_text(Some("OSWISPA_REMOTE_API_KEY"));
    env_entry.set_text(config.remote_backend.api_key_env.as_deref().unwrap_or(""));
    env_box.append(&env_entry);
    vbox.append(&env_box);

    let api_box = GtkBox::new(Orientation::Horizontal, 8);
    api_box.append(&Label::new(Some("Store API key:")));
    let api_key_entry = Entry::new();
    api_key_entry.set_hexpand(true);
    api_key_entry.set_visibility(false);
    api_key_entry.set_placeholder_text(Some("Leave empty to keep existing"));
    api_box.append(&api_key_entry);
    vbox.append(&api_box);

    let token_present = get_remote_api_key().is_some();
    let token_status = Label::new(Some(if token_present {
        "A remote API key is currently stored securely."
    } else {
        "No stored remote API key."
    }));
    token_status.set_xalign(0.0);
    token_status.add_css_class("dim-label");
    vbox.append(&token_status);

    let clear_key_check = CheckButton::with_label("Clear stored API key");
    clear_key_check.set_active(false);
    vbox.append(&clear_key_check);

    let spacer = GtkBox::new(Orientation::Vertical, 0);
    spacer.set_vexpand(true);
    vbox.append(&spacer);

    let save_btn = Button::with_label("Save Backend Settings");
    save_btn.add_css_class("suggested-action");

    let config_state_clone = config_state.clone();
    let event_tx_clone = event_tx.clone();
    save_btn.connect_clicked(move |_| {
        let mut new_config = config_state_clone.read().unwrap().clone();
        new_config.backend = match backend_combo.active_id().as_deref() {
            Some("remote") => TranscriptionBackend::Remote,
            _ => TranscriptionBackend::Local,
        };
        new_config.remote_backend.endpoint = endpoint_entry.text().trim().to_string();
        new_config.remote_backend.model = {
            let model = model_entry.text().trim().to_string();
            if model.is_empty() {
                "whisper-1".to_string()
            } else {
                model
            }
        };
        new_config.remote_backend.timeout_ms = timeout_spin.value() as u64;
        new_config.remote_backend.allow_insecure_http = insecure_check.is_active();
        new_config.remote_backend.api_key_env = {
            let env = env_entry.text().trim().to_string();
            if env.is_empty() {
                None
            } else {
                Some(env)
            }
        };

        if new_config.backend == TranscriptionBackend::Remote
            && new_config.remote_backend.endpoint.trim().is_empty()
        {
            error!("Remote backend selected but endpoint URL is empty");
            return;
        }

        if new_config.backend == TranscriptionBackend::Remote
            && new_config.remote_backend.endpoint.starts_with("http://")
            && !new_config.remote_backend.allow_insecure_http
        {
            error!("Remote endpoint uses HTTP; enable insecure HTTP only if you explicitly trust the network");
            return;
        }

        if clear_key_check.is_active() {
            if let Err(e) = clear_remote_api_key() {
                warn!("Failed to clear stored API key: {}", e);
            }
        }

        let typed_api_key = api_key_entry.text().trim().to_string();
        if !typed_api_key.is_empty() {
            if let Err(e) = set_remote_api_key(&typed_api_key) {
                warn!("Failed to store API key securely: {}", e);
            }
        }

        if let Err(e) = save_config(&new_config) {
            error!("Failed to save backend config: {}", e);
        } else {
            if let Ok(mut guard) = config_state_clone.write() {
                *guard = new_config;
            }
            info!("Backend settings updated");
            let _ = event_tx_clone.send(AppEvent::ReloadConfig);
        }
    });
    vbox.append(&save_btn);

    vbox
}

fn model_benchmark_text(benchmark: &ModelBenchmark) -> String {
    format!(
        "Size: {:.0}MB • Speed: {} • Accuracy: {}",
        benchmark.size_mb, benchmark.speed_tier, benchmark.accuracy_tier
    )
}

/// Format hotkey for display
fn format_hotkey(hotkey: &HotkeyConfig) -> String {
    let mut parts = Vec::new();
    if hotkey.ctrl {
        parts.push("Ctrl".to_string());
    }
    if hotkey.alt {
        parts.push("Alt".to_string());
    }
    if hotkey.shift {
        parts.push("Shift".to_string());
    }
    if hotkey.super_key {
        parts.push("Super".to_string());
    }
    if let Some(trigger) = hotkey
        .trigger_key
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        let pretty = match trigger {
            "space" => "Space".to_string(),
            "tab" => "Tab".to_string(),
            "enter" => "Enter".to_string(),
            "backspace" => "Backspace".to_string(),
            "capslock" => "CapsLock".to_string(),
            "grave" => "`".to_string(),
            other => other.to_ascii_uppercase(),
        };
        parts.push(pretty);
    }

    if parts.is_empty() {
        "None (disabled)".to_string()
    } else {
        parts.join(" + ")
    }
}

/// Save configuration to disk
fn save_config(config: &Config) -> anyhow::Result<()> {
    let config_path = get_config_dir().join("config.json");
    std::fs::create_dir_all(get_config_dir())?;
    let json = serde_json::to_string_pretty(config)?;
    std::fs::write(&config_path, json)?;
    info!("Configuration saved to {:?}", config_path);
    Ok(())
}
