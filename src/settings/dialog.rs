//! GTK4 Settings Dialog Implementation
//!
//! Full settings UI with tabs for General, Hotkey, and Models configuration.

use crate::models::{self, ModelInfo, AVAILABLE_MODELS};
use crate::{AppEvent, Config, HotkeyConfig, get_config_dir};
use crossbeam_channel::Sender;
use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box as GtkBox, Button, CheckButton, ComboBoxText,
    Grid, Label, Notebook, Orientation, ScrolledWindow, Separator,
};
use std::sync::Arc;
use tracing::{error, info};

/// Show the settings dialog
pub fn show_settings_dialog(config: &Arc<Config>, event_tx: Sender<AppEvent>) {
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

        let config_clone = config.clone();
        let event_tx_clone = event_tx.clone();
        app.connect_activate(move |app| {
            build_settings_window(app, &config_clone, event_tx_clone.clone());
        });

        app.run_with_args::<String>(&[]);
    });
}

fn build_settings_window(app: &Application, config: &Config, event_tx: Sender<AppEvent>) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("OSWispa Settings")
        .default_width(500)
        .default_height(450)
        .build();

    let notebook = Notebook::new();
    
    // Create tabs
    notebook.append_page(
        &create_general_tab(config, event_tx.clone()),
        Some(&Label::new(Some("General"))),
    );
    notebook.append_page(
        &create_hotkey_tab(config, event_tx.clone()),
        Some(&Label::new(Some("Hotkey"))),
    );
    notebook.append_page(
        &create_models_tab(config, event_tx.clone()),
        Some(&Label::new(Some("Models"))),
    );

    window.set_child(Some(&notebook));
    window.present();
}

/// Create General settings tab
fn create_general_tab(config: &Config, event_tx: Sender<AppEvent>) -> GtkBox {
    let vbox = GtkBox::new(Orientation::Vertical, 12);
    vbox.set_margin_top(20);
    vbox.set_margin_bottom(20);
    vbox.set_margin_start(20);
    vbox.set_margin_end(20);

    let title = Label::new(Some("General Settings"));
    title.add_css_class("title-2");
    vbox.append(&title);

    vbox.append(&Separator::new(Orientation::Horizontal));

    // Audio feedback toggle
    let audio_check = CheckButton::with_label("Audio feedback (tones for recording events)");
    audio_check.set_active(config.audio_feedback);
    vbox.append(&audio_check);

    // Auto-paste toggle
    let paste_check = CheckButton::with_label("Auto-paste transcribed text");
    paste_check.set_active(config.auto_paste);
    vbox.append(&paste_check);

    // Notifications toggle
    let notify_check = CheckButton::with_label("Show desktop notifications");
    notify_check.set_active(config.notification_enabled);
    vbox.append(&notify_check);

    // Punctuation commands toggle
    let punct_check = CheckButton::with_label("Punctuation commands (say \"period\" for \".\")");
    punct_check.set_active(config.punctuation_commands);
    vbox.append(&punct_check);

    // Language selector
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

    // Translate to English toggle
    let translate_check = CheckButton::with_label("Translate to English (for non-English speech)");
    translate_check.set_active(config.translate_to_english);
    vbox.append(&translate_check);

    // Spacer
    let spacer = GtkBox::new(Orientation::Vertical, 0);
    spacer.set_vexpand(true);
    vbox.append(&spacer);

    // Save button
    let save_btn = Button::with_label("Save Settings");
    save_btn.add_css_class("suggested-action");
    
    let config_clone = config.clone();
    let event_tx_clone = event_tx.clone();
    save_btn.connect_clicked(move |_| {
        let mut new_config = config_clone.clone();
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
            info!("Settings saved successfully");
            let _ = event_tx_clone.send(AppEvent::ReloadConfig);
        }
    });
    vbox.append(&save_btn);

    vbox
}

/// Create Hotkey settings tab
fn create_hotkey_tab(config: &Config, event_tx: Sender<AppEvent>) -> GtkBox {
    let vbox = GtkBox::new(Orientation::Vertical, 12);
    vbox.set_margin_top(20);
    vbox.set_margin_bottom(20);
    vbox.set_margin_start(20);
    vbox.set_margin_end(20);

    let title = Label::new(Some("Hotkey Configuration"));
    title.add_css_class("title-2");
    vbox.append(&title);

    vbox.append(&Separator::new(Orientation::Horizontal));

    let desc = Label::new(Some("Select modifier keys for recording hotkey.\nHold the combination to record, release to transcribe."));
    desc.set_wrap(true);
    vbox.append(&desc);

    // Modifier checkboxes
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

    // Preview label
    let preview_box = GtkBox::new(Orientation::Horizontal, 8);
    preview_box.set_margin_top(16);
    preview_box.append(&Label::new(Some("Current hotkey:")));
    
    let preview_label = Label::new(Some(&format_hotkey(&config.hotkey)));
    preview_label.add_css_class("title-3");
    preview_box.append(&preview_label);
    vbox.append(&preview_box);

    // Update preview on checkbox changes
    let preview_ref = preview_label.clone();
    let update_preview = {
        let ctrl = ctrl_check.clone();
        let alt = alt_check.clone();
        let shift = shift_check.clone();
        let super_key = super_check.clone();
        move || {
            let hotkey = HotkeyConfig {
                ctrl: ctrl.is_active(),
                alt: alt.is_active(),
                shift: shift.is_active(),
                super_key: super_key.is_active(),
            };
            preview_ref.set_text(&format_hotkey(&hotkey));
        }
    };

    let update_clone = update_preview.clone();
    ctrl_check.connect_toggled(move |_| update_clone());
    let update_clone = update_preview.clone();
    alt_check.connect_toggled(move |_| update_clone());
    let update_clone = update_preview.clone();
    shift_check.connect_toggled(move |_| update_clone());
    let update_clone = update_preview.clone();
    super_check.connect_toggled(move |_| update_clone());

    // Spacer
    let spacer = GtkBox::new(Orientation::Vertical, 0);
    spacer.set_vexpand(true);
    vbox.append(&spacer);

    // Warning about restart
    let warning = Label::new(Some("Note: Hotkey changes take effect immediately."));
    warning.add_css_class("dim-label");
    vbox.append(&warning);

    // Save button
    let save_btn = Button::with_label("Apply Hotkey");
    save_btn.add_css_class("suggested-action");
    
    let config_clone = config.clone();
    let event_tx_clone = event_tx.clone();
    save_btn.connect_clicked(move |_| {
        let mut new_config = config_clone.clone();
        new_config.hotkey = HotkeyConfig {
            ctrl: ctrl_check.is_active(),
            alt: alt_check.is_active(),
            shift: shift_check.is_active(),
            super_key: super_check.is_active(),
        };
        
        if let Err(e) = save_config(&new_config) {
            error!("Failed to save hotkey config: {}", e);
        } else {
            info!("Hotkey updated: {}", format_hotkey(&new_config.hotkey));
            // Notify main loop to reload config
            let _ = event_tx_clone.send(AppEvent::ReloadConfig);
        }
    });
    vbox.append(&save_btn);

    vbox
}

/// Create Models settings tab
fn create_models_tab(config: &Config, event_tx: Sender<AppEvent>) -> GtkBox {
    let vbox = GtkBox::new(Orientation::Vertical, 12);
    vbox.set_margin_top(20);
    vbox.set_margin_bottom(20);
    vbox.set_margin_start(20);
    vbox.set_margin_end(20);

    let title = Label::new(Some("Whisper Models"));
    title.add_css_class("title-2");
    vbox.append(&title);

    vbox.append(&Separator::new(Orientation::Horizontal));

    // Current model display
    let current_box = GtkBox::new(Orientation::Horizontal, 8);
    current_box.append(&Label::new(Some("Active model:")));
    let current_model = config.model_path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "Unknown".to_string());
    let current_label = Label::new(Some(&current_model));
    current_label.add_css_class("title-4");
    current_box.append(&current_label);
    vbox.append(&current_box);

    vbox.append(&Separator::new(Orientation::Horizontal));

    // Scrollable model list
    let scroll = ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_min_content_height(200);

    let models_box = GtkBox::new(Orientation::Vertical, 8);

    for model in AVAILABLE_MODELS {
        let row = create_model_row(model, config, event_tx.clone());
        models_box.append(&row);
    }

    scroll.set_child(Some(&models_box));
    vbox.append(&scroll);

    vbox
}

/// Create a row for a model in the list
fn create_model_row(model: &'static ModelInfo, config: &Config, event_tx: Sender<AppEvent>) -> GtkBox {
    let row = GtkBox::new(Orientation::Horizontal, 12);
    row.set_margin_top(4);
    row.set_margin_bottom(4);

    // Model info
    let info_box = GtkBox::new(Orientation::Vertical, 2);
    info_box.set_hexpand(true);

    let name_label = Label::new(Some(model.name));
    name_label.set_halign(gtk4::Align::Start);
    name_label.add_css_class("heading");
    info_box.append(&name_label);

    let desc_label = Label::new(Some(&format!("{} • {}MB", model.description, model.size_mb)));
    desc_label.set_halign(gtk4::Align::Start);
    desc_label.add_css_class("dim-label");
    info_box.append(&desc_label);

    row.append(&info_box);

    // Status/action
    let is_installed = models::is_model_installed(model);
    let is_active = config.model_path.file_name()
        .map(|n| n.to_string_lossy() == model.filename)
        .unwrap_or(false);

    if is_active {
        let label = Label::new(Some("✓ Active"));
        label.add_css_class("success");
        row.append(&label);
    } else if is_installed {
        let use_btn = Button::with_label("Use");
        let config_clone = config.clone();
        let model_filename = model.filename;
        use_btn.connect_clicked(move |_| {
            let mut new_config = config_clone.clone();
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
            
            // Start async download
            let model_clone = model;
            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    match models::download_model(model_clone, |downloaded, total| {
                        let percent = (downloaded as f64 / total as f64 * 100.0) as u32;
                        if percent % 10 == 0 {
                            info!("Download progress: {}%", percent);
                        }
                    }).await {
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

/// Format hotkey for display
fn format_hotkey(hotkey: &HotkeyConfig) -> String {
    let mut parts = Vec::new();
    if hotkey.ctrl { parts.push("Ctrl"); }
    if hotkey.alt { parts.push("Alt"); }
    if hotkey.shift { parts.push("Shift"); }
    if hotkey.super_key { parts.push("Super"); }
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
