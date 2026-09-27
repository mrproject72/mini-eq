//! Preset management panel.
//!
//! Clean layout: a header (title + saved/modified state chip), a compact
//! toolbar (Add / Remove / Import / Export), and a single ListBox that is
//! the sole preset selector. Built-in factory presets (Neutral, Bass Boost,
//! Treble Boost) always appear at the top and cannot be removed; user
//! presets (added or imported) appear below and can be removed. Clicking a
//! row loads that preset.
//!
//! The per-output-device auto-preset features (Fallback / Link to Output)
//! live in the Headroom panel's "Output Controls" section, not here.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use gtk4::prelude::*;

use crate::autoeq::parse_apo_file;
use crate::core::{
    BUILTIN_PRESET_NAMES, default_bands, is_builtin_preset, load_preset_from_file,
    preset_path_for_name, sanitize_preset_name, save_preset_to_file,
};

/// Preset management widget.
pub struct PresetPanel {
    pub container: gtk4::Box,
    pub list_box: gtk4::ListBox,
    pub add_button: gtk4::Button,
    pub remove_button: gtk4::Button,
    pub import_button: gtk4::Button,
    pub export_button: gtk4::Button,
    pub state_chip: gtk4::Label,
    current_bands: Vec<crate::core::EqBand>,
    current_preamp_db: f64,
    apply_bands_callback: Option<Box<dyn Fn(Vec<crate::core::EqBand>, f64)>>,
    reset_callback: Option<Box<dyn Fn()>>,
    get_signature_callback: Option<Box<dyn Fn() -> String>>,
    current_preset_name: Option<String>,
    saved_signature: Option<String>,
    revert_baseline_label: Option<String>,
    revert_baseline_signature: Option<String>,
    revert_baseline_payload: Option<serde_json::Value>,
    default_signature: Option<String>,
    file_monitor: Option<glib::SignalHandlerId>,
}

impl PresetPanel {
    pub fn new() -> Rc<RefCell<Self>> {
        let add_button = gtk4::Button::from_icon_name("list-add-symbolic");
        add_button.set_tooltip_text(Some("Add a new preset from the current EQ"));
        let remove_button = gtk4::Button::from_icon_name("list-remove-symbolic");
        remove_button.set_tooltip_text(Some("Remove the selected custom preset"));
        let import_button = gtk4::Button::from_icon_name("document-open-symbolic");
        import_button.set_tooltip_text(Some("Import an APO (.apo/.txt) preset..."));
        let export_button = gtk4::Button::from_icon_name("document-save-as-symbolic");
        export_button.set_tooltip_text(Some("Export the selected preset to a file..."));

        // --- Header: title + state chip + toolbar.
        let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        let title = gtk4::Label::new(Some("Presets"));
        title.set_css_classes(&["heading"]);
        header.append(&title);

        let state_chip = gtk4::Label::new(Some("Neutral"));
        state_chip.set_css_classes(&["preset-state-chip", "preset-state-chip-neutral"]);
        state_chip.set_valign(gtk4::Align::Center);
        header.append(&state_chip);

        let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        header.append(&spacer);

        header.append(&add_button);
        header.append(&remove_button);
        header.append(&import_button);
        header.append(&export_button);

        // --- Preset list (the sole selector).
        let list_box = gtk4::ListBox::new();
        list_box.set_selection_mode(gtk4::SelectionMode::Single);
        list_box.set_vexpand(true);
        list_box.set_css_classes(&["preset-list"]);

        let scrolled = gtk4::ScrolledWindow::new();
        scrolled.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
        scrolled.set_vexpand(true);
        scrolled.set_child(Some(&list_box));

        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
        container.set_css_classes(&["utility-section"]);
        container.set_margin_top(8);
        container.set_margin_bottom(8);
        container.set_margin_start(8);
        container.set_margin_end(8);
        container.append(&header);
        container.append(&scrolled);

        let panel = Rc::new(RefCell::new(Self {
            container,
            list_box,
            add_button,
            remove_button,
            import_button,
            export_button,
            state_chip,
            current_bands: default_bands(),
            current_preamp_db: 0.0,
            apply_bands_callback: None,
            reset_callback: None,
            get_signature_callback: None,
            current_preset_name: None,
            saved_signature: None,
            revert_baseline_label: None,
            revert_baseline_signature: None,
            revert_baseline_payload: None,
            default_signature: None,
            file_monitor: None,
        }));

        refresh_preset_list(&panel.borrow().list_box);

        // --- Add: create a new custom preset from the current EQ state.
        let panel_clone = panel.clone();
        panel.borrow().add_button.connect_clicked(move |_| {
            let list_box = panel_clone.borrow().list_box.clone();
            let count = count_custom_children(&list_box);
            let name = format!("preset_{}", count + 1);
            let sanitized = sanitize_preset_name(&name);
            let path = preset_path_for_name(&sanitized);
            let (bands, preamp) = {
                let p = panel_clone.borrow();
                (p.current_bands.clone(), p.current_preamp_db)
            };
            let _ = save_preset_to_file(&path, &bands, preamp);
            refresh_preset_list(&list_box);
        });

        // --- Remove: delete the selected CUSTOM preset (built-ins are safe).
        let panel_clone = panel.clone();
        panel.borrow().remove_button.connect_clicked(move |_| {
            let list_box = panel_clone.borrow().list_box.clone();
            let Some(row) = list_box.selected_row() else {
                return;
            };
            let Some(name) = row_name(&row) else { return };
            if is_builtin_preset(&name) {
                // Built-in presets cannot be removed; ignore.
                return;
            }
            let _ = crate::core::delete_preset_file(&name);
            refresh_preset_list(&list_box);
        });

        // --- Import: APO file -> new custom preset.
        let panel_clone = panel.clone();
        panel.borrow().import_button.connect_clicked(move |_| {
            let dialog = gtk4::FileChooserDialog::new(
                Some("Import APO Preset"),
                gtk4::Window::NONE,
                gtk4::FileChooserAction::Open,
                &[
                    ("Cancel", gtk4::ResponseType::Cancel),
                    ("Import", gtk4::ResponseType::Accept),
                ],
            );
            dialog.set_modal(true);

            let filter = gtk4::FileFilter::new();
            filter.add_pattern("*.apo");
            filter.add_pattern("*.txt");
            filter.set_name(Some("APO Presets"));
            dialog.add_filter(&filter);

            let list_box_for_import = panel_clone.borrow().list_box.clone();
            dialog.connect_response(move |d, response| {
                if response == gtk4::ResponseType::Accept {
                    if let Some(file) = d.file() {
                        if let Some(path) = file.path() {
                            if let Ok((_preamp, bands)) = parse_apo_file(&path) {
                                let count = count_custom_children(&list_box_for_import);
                                let name = format!("imported_{}", count + 1);
                                let sanitized = sanitize_preset_name(&name);
                                let dest = preset_path_for_name(&sanitized);
                                let _ = save_preset_to_file(&dest, &bands, _preamp);
                                refresh_preset_list(&list_box_for_import);
                            }
                        }
                    }
                }
                d.close();
            });

            dialog.show();
        });

        // --- Export: selected preset -> file.
        let panel_clone = panel.clone();
        panel.borrow().export_button.connect_clicked(move |_| {
            let list_box = panel_clone.borrow().list_box.clone();
            let Some(row) = list_box.selected_row() else {
                return;
            };
            let Some(name) = row_name(&row) else { return };
            let (preamp, bands) = match load_preset_by_name(&name) {
                Ok(v) => v,
                Err(_) => return,
            };
            let dialog = gtk4::FileChooserDialog::new(
                Some("Export Preset"),
                gtk4::Window::NONE,
                gtk4::FileChooserAction::Save,
                &[
                    ("Cancel", gtk4::ResponseType::Cancel),
                    ("Export", gtk4::ResponseType::Accept),
                ],
            );
            dialog.set_modal(true);
            dialog.set_current_name(&format!("{name}.json"));
            dialog.connect_response(move |d, response| {
                if response == gtk4::ResponseType::Accept {
                    if let Some(file) = d.file() {
                        if let Some(path) = file.path() {
                            let _ = save_preset_to_file(&path, &bands, preamp);
                        }
                    }
                }
                d.close();
            });
            dialog.show();
        });

        // --- Select a row -> load that preset.
        let panel_clone = panel.clone();
        panel.borrow().list_box.connect_row_selected(move |_, row| {
            let Some(row) = row else { return };
            let Some(name) = row_name(row) else { return };
            {
                let mut panel_mut = panel_clone.borrow_mut();
                if let Ok((preamp, bands)) = load_preset_by_name(&name) {
                    if let Some(ref apply) = panel_mut.apply_bands_callback {
                        apply(bands.clone(), preamp);
                    }
                    panel_mut.current_bands = bands;
                    panel_mut.current_preamp_db = preamp;
                    panel_mut.current_preset_name = Some(name.clone());
                    panel_mut.saved_signature = Some(
                        panel_mut
                            .get_signature_callback
                            .as_ref()
                            .map(|f| f())
                            .unwrap_or_default(),
                    );
                    panel_mut.set_curve_revert_baseline(Some(name));
                    panel_mut.update_state_chip();
                }
            }
        });

        panel
    }

    pub fn set_callbacks(
        &mut self,
        apply_bands: Option<Box<dyn Fn(Vec<crate::core::EqBand>, f64)>>,
        reset: Option<Box<dyn Fn()>>,
        get_signature: Option<Box<dyn Fn() -> String>>,
    ) {
        self.apply_bands_callback = apply_bands;
        self.reset_callback = reset;
        self.get_signature_callback = get_signature;
    }

    pub fn set_default_signature(&mut self, signature: String) {
        self.default_signature = Some(signature);
    }

    /// The currently-loaded preset name, if any (used by the Output Controls
    /// Fallback / Link-to-Output actions).
    pub fn current_preset_name(&self) -> Option<String> {
        self.current_preset_name.clone()
    }

    pub fn start_file_monitoring(&mut self) {
        let dir = crate::core::ensure_preset_storage_dir();
        let file = gio::File::for_path(&dir);
        if let Ok(monitor) =
            file.monitor_directory(gio::FileMonitorFlags::NONE, gio::Cancellable::NONE)
        {
            let list_box = self.list_box.clone();
            let handler = monitor.connect_changed(move |_, _, _, _| {
                refresh_preset_list(&list_box);
            });
            self.file_monitor = Some(handler);
        }
    }

    pub fn refresh_list(&self) {
        refresh_preset_list(&self.list_box);
    }

    pub fn update_state_chip(&mut self) {
        let signature = self
            .get_signature_callback
            .as_ref()
            .map(|f| f())
            .unwrap_or_default();
        let current_name = self.current_preset_name.as_deref();
        let saved_sig = self.saved_signature.as_deref();

        if current_name.is_some() && saved_sig == Some(signature.as_str()) {
            self.state_chip.set_text("Saved");
            self.state_chip
                .set_css_classes(&["preset-state-chip", "preset-state-chip-saved"]);
        } else if current_name.is_some() {
            self.state_chip.set_text("Modified");
            self.state_chip
                .set_css_classes(&["preset-state-chip", "preset-state-chip-modified"]);
        } else if self.default_signature.as_deref() == Some(signature.as_str()) {
            self.state_chip.set_text("Neutral");
            self.state_chip
                .set_css_classes(&["preset-state-chip", "preset-state-chip-neutral"]);
        } else {
            self.state_chip.set_text("Unsaved");
            self.state_chip
                .set_css_classes(&["preset-state-chip", "preset-state-chip-unsaved"]);
        }
    }

    pub fn load_library_preset(&mut self, name: &str) -> anyhow::Result<()> {
        let preset_name = sanitize_preset_name(name);
        if preset_name.is_empty() {
            anyhow::bail!("preset name is empty");
        }
        let (preamp, bands) = load_preset_by_name(&preset_name)?;
        if let Some(ref apply) = self.apply_bands_callback {
            apply(bands.clone(), preamp);
        }
        self.current_bands = bands;
        self.current_preamp_db = preamp;
        self.current_preset_name = Some(preset_name.clone());
        self.saved_signature = Some(
            self.get_signature_callback
                .as_ref()
                .map(|f| f())
                .unwrap_or_default(),
        );
        self.set_curve_revert_baseline(Some(preset_name));
        self.update_state_chip();
        Ok(())
    }

    pub fn reset_to_neutral(&mut self) {
        if let Some(ref reset) = self.reset_callback {
            reset();
        }
        self.current_preset_name = None;
        self.saved_signature = None;
        self.set_curve_revert_baseline(None);
        self.update_state_chip();
    }

    pub fn revert_to_baseline(&mut self) {
        if let Some(ref payload) = self.revert_baseline_payload {
            if let (Some(preamp), Ok(bands)) = (
                payload.get("preamp_db").and_then(|v| v.as_f64()),
                crate::core::preset_payload_bands(payload),
            ) {
                if let Some(ref apply) = self.apply_bands_callback {
                    apply(bands.clone(), preamp);
                }
                self.current_bands = bands;
                self.current_preamp_db = preamp;
                self.current_preset_name = self.revert_baseline_label.clone();
                self.saved_signature = self.revert_baseline_signature.clone();
                self.update_state_chip();
            }
        }
    }

    fn set_curve_revert_baseline(&mut self, label: Option<String>) {
        self.revert_baseline_label = label;
        self.revert_baseline_signature = self.get_signature_callback.as_ref().map(|f| f());
        let bands = self.current_bands.clone();
        let preamp = self.current_preamp_db;
        self.revert_baseline_payload = Some(crate::core::preset_payload(&bands, preamp));
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }
}

/// Load a preset by name: built-in presets come from code, custom presets
/// from the preset directory.
fn load_preset_by_name(name: &str) -> anyhow::Result<(f64, Vec<crate::core::EqBand>)> {
    if is_builtin_preset(name) {
        if let Some((bands, preamp)) = crate::core::builtin_preset_bands(name) {
            return Ok((preamp, bands));
        }
    }
    let path = preset_path_for_name(name);
    load_preset_from_file(&path)
}

/// Extract the preset name stored on a list row (set by `refresh_preset_list`).
fn row_name(row: &gtk4::ListBoxRow) -> Option<String> {
    row.child()
        .and_downcast::<gtk4::Label>()
        .map(|l| l.label().to_string())
}

/// Count only the CUSTOM (non-built-in) preset rows, so new custom presets
/// get sequential names that don't collide with built-ins.
fn count_custom_children(list_box: &gtk4::ListBox) -> usize {
    let mut n = 0;
    let mut child = list_box.first_child();
    while let Some(c) = child {
        if let Some(row) = c.downcast_ref::<gtk4::ListBoxRow>()
            && let Some(name) = row_name(row)
            && !is_builtin_preset(&name)
        {
            n += 1;
        }
        child = c.next_sibling();
    }
    n
}

/// Rebuild the preset ListBox: built-in presets first (marked), then custom
/// presets. The row's label text IS the preset name (used by `row_name`).
fn refresh_preset_list(list_box: &gtk4::ListBox) {
    let selected = list_box.selected_row().and_then(|r| row_name(&r));
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }

    let mut rows: Vec<(String, bool)> = Vec::new();
    for name in BUILTIN_PRESET_NAMES {
        rows.push(((*name).to_string(), true));
    }
    for name in list_preset_names() {
        if !is_builtin_preset(&name) {
            rows.push((name, false));
        }
    }

    for (name, builtin) in rows {
        let row = gtk4::ListBoxRow::new();
        let label = gtk4::Label::new(Some(&name));
        label.set_halign(gtk4::Align::Start);
        if builtin {
            label.set_css_classes(&["preset-builtin-label"]);
            label.set_tooltip_text(Some("Built-in preset (cannot be removed)"));
        }
        row.set_child(Some(&label));
        list_box.append(&row);
        if selected.as_deref() == Some(name.as_str()) {
            list_box.select_row(Some(&row));
        }
    }
}

/// Get the preset storage directory.
pub fn preset_storage_dir() -> PathBuf {
    crate::core::preset_storage_dir()
}

/// List all custom preset names (de-duplicated, case-insensitively sorted).
pub fn list_preset_names() -> Vec<String> {
    crate::core::list_preset_names()
}
