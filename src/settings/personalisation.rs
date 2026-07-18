//! Linux GTK management surface for the local personal dictionary.

use crate::personalisation::{
    export_personalisation, import_personalisation, save_personalisation, DictionaryEntry,
    Personalisation,
};
use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Button, CheckButton, Entry, FileChooserAction, FileChooserNative, Grid, Label,
    Orientation, ResponseType, ScrolledWindow, Separator,
};
use std::sync::{Arc, RwLock};

pub(super) fn create_personalisation_tab(state: Arc<RwLock<Personalisation>>) -> GtkBox {
    let page = GtkBox::new(Orientation::Vertical, 12);
    page.set_margin_top(20);
    page.set_margin_bottom(20);
    page.set_margin_start(20);
    page.set_margin_end(20);

    let title = Label::new(Some("Personal Dictionary"));
    title.add_css_class("title-2");
    title.set_xalign(0.0);
    page.append(&title);

    let description = Label::new(Some(
        "Replace phrases locally before punctuation is applied. Entries are explicit, private, and never learned from other apps.",
    ));
    description.set_wrap(true);
    description.set_xalign(0.0);
    page.append(&description);
    page.append(&Separator::new(Orientation::Horizontal));

    let add_grid = Grid::new();
    add_grid.set_column_spacing(8);
    add_grid.set_row_spacing(8);
    let spoken = Entry::new();
    spoken.set_placeholder_text(Some("What Whisper hears"));
    spoken.set_hexpand(true);
    let written = Entry::new();
    written.set_placeholder_text(Some("Preferred spelling"));
    written.set_hexpand(true);
    let case_sensitive = CheckButton::with_label("Case-sensitive");
    let add = Button::with_label("Add Entry");
    add.add_css_class("suggested-action");
    add_grid.attach(&spoken, 0, 0, 1, 1);
    add_grid.attach(&written, 1, 0, 1, 1);
    add_grid.attach(&case_sensitive, 2, 0, 1, 1);
    add_grid.attach(&add, 3, 0, 1, 1);
    page.append(&add_grid);

    let status = Label::new(None);
    status.set_xalign(0.0);
    status.set_wrap(true);
    status.add_css_class("dim-label");
    page.append(&status);

    let entries = GtkBox::new(Orientation::Vertical, 8);
    let scroller = ScrolledWindow::new();
    scroller.set_vexpand(true);
    scroller.set_child(Some(&entries));
    page.append(&scroller);

    refresh_rows(&entries, &state, &status);

    let state_for_add = state.clone();
    let entries_for_add = entries.clone();
    let status_for_add = status.clone();
    let spoken_for_add = spoken.clone();
    let written_for_add = written.clone();
    add.connect_clicked(move |_| {
        let entry = DictionaryEntry {
            spoken: spoken_for_add.text().to_string(),
            written: written_for_add.text().to_string(),
            enabled: true,
            case_sensitive: case_sensitive.is_active(),
        };
        let mut dictionary = state_for_add.read().unwrap().dictionary().to_vec();
        dictionary.push(entry);
        match replace_and_save(&state_for_add, dictionary) {
            Ok(()) => {
                spoken_for_add.set_text("");
                written_for_add.set_text("");
                status_for_add.set_text("Dictionary entry added.");
                refresh_rows(&entries_for_add, &state_for_add, &status_for_add);
            }
            Err(error) => status_for_add.set_text(&format!("Could not add entry: {error}")),
        }
    });

    let transfer_box = GtkBox::new(Orientation::Horizontal, 8);
    let import = Button::with_label("Import JSON…");
    let export = Button::with_label("Export JSON…");
    transfer_box.append(&import);
    transfer_box.append(&export);
    page.append(&transfer_box);

    let state_for_import = state.clone();
    let entries_for_import = entries.clone();
    let status_for_import = status.clone();
    import.connect_clicked(move |_| {
        let chooser = FileChooserNative::new(
            Some("Import MorpheOS Voice dictionary"),
            None::<&gtk4::Window>,
            FileChooserAction::Open,
            Some("Import"),
            Some("Cancel"),
        );
        let state = state_for_import.clone();
        let entries = entries_for_import.clone();
        let status = status_for_import.clone();
        chooser.connect_response(move |chooser, response| {
            if response != ResponseType::Accept {
                return;
            }
            let Some(path) = chooser.file().and_then(|file| file.path()) else {
                status.set_text("The selected import is not a local file.");
                return;
            };
            match import_personalisation(&path)
                .and_then(|candidate| save_personalisation(&candidate).map(|_| candidate))
            {
                Ok(candidate) => {
                    *state.write().unwrap() = candidate;
                    status.set_text("Dictionary imported and applied.");
                    refresh_rows(&entries, &state, &status);
                }
                Err(error) => status.set_text(&format!("Could not import dictionary: {error}")),
            }
        });
        chooser.show();
    });

    let state_for_export = state.clone();
    let status_for_export = status.clone();
    export.connect_clicked(move |_| {
        let chooser = FileChooserNative::new(
            Some("Export MorpheOS Voice dictionary"),
            None::<&gtk4::Window>,
            FileChooserAction::Save,
            Some("Export"),
            Some("Cancel"),
        );
        // Retain the legacy export filename for compatibility with existing
        // documentation and user backups during the transition release.
        chooser.set_current_name("oswispa-personalisation.json");
        let state = state_for_export.clone();
        let status = status_for_export.clone();
        chooser.connect_response(move |chooser, response| {
            if response != ResponseType::Accept {
                return;
            }
            let Some(path) = chooser.file().and_then(|file| file.path()) else {
                status.set_text("The selected export is not a local file.");
                return;
            };
            match state
                .read()
                .map_err(|_| anyhow::anyhow!("dictionary state is unavailable"))
                .and_then(|snapshot| export_personalisation(&snapshot, &path))
            {
                Ok(()) => status.set_text("Dictionary exported."),
                Err(error) => status.set_text(&format!("Could not export dictionary: {error}")),
            }
        });
        chooser.show();
    });

    page
}

fn refresh_rows(container: &GtkBox, state: &Arc<RwLock<Personalisation>>, status: &Label) {
    while let Some(child) = container.first_child() {
        container.remove(&child);
    }

    let snapshot = state.read().unwrap().dictionary().to_vec();
    if snapshot.is_empty() {
        let empty = Label::new(Some(
            "No entries yet. Add a phrase above or import a JSON file.",
        ));
        empty.set_xalign(0.0);
        empty.add_css_class("dim-label");
        container.append(&empty);
        return;
    }

    for (index, item) in snapshot.into_iter().enumerate() {
        let row = Grid::new();
        row.set_column_spacing(8);
        row.set_row_spacing(4);

        let enabled = CheckButton::with_label("Enabled");
        enabled.set_active(item.enabled);
        let spoken = Entry::new();
        spoken.set_text(&item.spoken);
        spoken.set_hexpand(true);
        let written = Entry::new();
        written.set_text(&item.written);
        written.set_hexpand(true);
        let case_sensitive = CheckButton::with_label("Case-sensitive");
        case_sensitive.set_active(item.case_sensitive);
        let save = Button::with_label("Save");
        let delete = Button::with_label("Delete");
        delete.add_css_class("destructive-action");

        row.attach(&enabled, 0, 0, 1, 1);
        row.attach(&spoken, 1, 0, 1, 1);
        row.attach(&written, 2, 0, 1, 1);
        row.attach(&case_sensitive, 3, 0, 1, 1);
        row.attach(&save, 4, 0, 1, 1);
        row.attach(&delete, 5, 0, 1, 1);
        container.append(&row);

        let state_for_save = state.clone();
        let container_for_save = container.clone();
        let status_for_save = status.clone();
        save.connect_clicked(move |_| {
            let mut dictionary = state_for_save.read().unwrap().dictionary().to_vec();
            if let Some(entry) = dictionary.get_mut(index) {
                entry.spoken = spoken.text().to_string();
                entry.written = written.text().to_string();
                entry.enabled = enabled.is_active();
                entry.case_sensitive = case_sensitive.is_active();
            }
            match replace_and_save(&state_for_save, dictionary) {
                Ok(()) => {
                    status_for_save.set_text("Dictionary entry saved.");
                    refresh_rows(&container_for_save, &state_for_save, &status_for_save);
                }
                Err(error) => status_for_save.set_text(&format!("Could not save entry: {error}")),
            }
        });

        let state_for_delete = state.clone();
        let container_for_delete = container.clone();
        let status_for_delete = status.clone();
        delete.connect_clicked(move |_| {
            let mut dictionary = state_for_delete.read().unwrap().dictionary().to_vec();
            if index < dictionary.len() {
                dictionary.remove(index);
            }
            match replace_and_save(&state_for_delete, dictionary) {
                Ok(()) => {
                    status_for_delete.set_text("Dictionary entry deleted.");
                    refresh_rows(&container_for_delete, &state_for_delete, &status_for_delete);
                }
                Err(error) => {
                    status_for_delete.set_text(&format!("Could not delete entry: {error}"))
                }
            }
        });
    }
}

fn replace_and_save(
    state: &Arc<RwLock<Personalisation>>,
    dictionary: Vec<DictionaryEntry>,
) -> anyhow::Result<()> {
    let candidate = Personalisation::from_dictionary(dictionary)?;
    save_personalisation(&candidate)?;
    *state.write().unwrap() = candidate;
    Ok(())
}
