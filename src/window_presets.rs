//! Preset management panel widgets.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use gtk4::prelude::*;

use crate::autoeq::parse_apo_file;
use crate::core::{
    default_bands, ensure_preset_storage_dir, format_frequency, load_preset_from_file, PRESET_FILE_SUFFIX,
    preset_path_for_name, sanitize_preset_name, save_preset_to_file,
};

/// Preset management widget.
pub struct PresetPanel {
    pub container: gtk4::Box,
    pub list_box: gtk4::ListBox,
    pub add_button: gtk4::Button,
    pub delete_button: gtk4::Button,
    pub save_button: gtk4::Button,
    pub import_button: gtk4::Button,
    pub export_button: gtk4::Button,
    pub revert_button: gtk4::Button,
    pub reset_button: gtk4::Button,
    pub current_curve_label: gtk4::Label,
    pub state_chip: gtk4::Label,
    pub preset_combo: gtk4::ComboBoxText,
    pub fallback_button: gtk4::Button,
    pub fallback_label: gtk4::Label,
    pub link_button: gtk4::Button,
    pub link_label: gtk4::Label,
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
        add_button.set_tooltip_text(Some("Add preset"));
        let delete_button = gtk4::Button::from_icon_name("list-remove-symbolic");
        delete_button.set_tooltip_text(Some("Delete preset"));
        let save_button = gtk4::Button::from_icon_name("document-save-symbolic");
        save_button.set_tooltip_text(Some("Save preset"));
        let import_button = gtk4::Button::from_icon_name("document-open-symbolic");
        import_button.set_tooltip_text(Some("Import APO preset..."));
        let export_button = gtk4::Button::from_icon_name("document-save-as-symbolic");
        export_button.set_tooltip_text(Some("Export preset..."));

        let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        let title = gtk4::Label::new(Some("Presets"));
        title.set_css_classes(&["heading"]);
        header.append(&title);
        header.set_hexpand(true);
        header.append(&add_button);
        header.append(&delete_button);
        header.append(&save_button);
        header.append(&import_button);
        header.append(&export_button);

        let list_box = gtk4::ListBox::new();
        list_box.set_selection_mode(gtk4::SelectionMode::Single);
        list_box.set_vexpand(true);

        refresh_preset_list(&list_box);

        let scrolled = gtk4::ScrolledWindow::new();
        scrolled.set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Automatic);
        scrolled.set_child(Some(&list_box));

        let revert_button = gtk4::Button::with_label("Revert");
        revert_button.set_tooltip_text(Some("Restore saved preset"));
        let reset_button = gtk4::Button::with_label("Reset");
        reset_button.set_tooltip_text(Some("Reset to neutral"));
        let preset_combo = gtk4::ComboBoxText::new();
        let fallback_button = gtk4::Button::with_label("Set Fallback");
        fallback_button.set_tooltip_text(Some("Use this preset for unmatched outputs"));
        let fallback_label = gtk4::Label::new(Some("Bypass"));
        let link_button = gtk4::Button::with_label("Link to Output");
        link_button.set_tooltip_text(Some("Auto-load this preset for current output"));
        let link_label = gtk4::Label::new(Some("No link"));

        let current_curve_label = gtk4::Label::new(Some("Neutral"));
        let state_chip = gtk4::Label::new(Some("Saved"));

        let current_curve_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        current_curve_row.set_halign(gtk4::Align::Fill);
        let curve_label = gtk4::Label::new(Some("Current Curve"));
        curve_label.set_css_classes(&["metric-title"]);
        curve_label.set_hexpand(true);
        current_curve_row.append(&curve_label);
        current_curve_row.append(&current_curve_label);
        current_curve_row.append(&state_chip);
        current_curve_row.append(&revert_button);
        current_curve_row.append(&reset_button);

        let preset_combo_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        preset_combo_row.set_halign(gtk4::Align::Fill);
        let combo_label = gtk4::Label::new(Some("Load Preset"));
        preset_combo_row.append(&combo_label);
        preset_combo_row.append(&preset_combo);

        let fallback_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        fallback_row.set_halign(gtk4::Align::Fill);
        let fallback_title = gtk4::Label::new(Some("Fallback"));
        fallback_title.set_css_classes(&["metric-title"]);
        fallback_title.set_hexpand(true);
        fallback_row.append(&fallback_title);
        fallback_row.append(&fallback_button);
        fallback_row.append(&fallback_label);

        let link_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        link_row.set_halign(gtk4::Align::Fill);
        let link_title = gtk4::Label::new(Some("Output Link"));
        link_title.set_css_classes(&["metric-title"]);
        link_title.set_hexpand(true);
        link_row.append(&link_title);
        link_row.append(&link_button);
        link_row.append(&link_label);

        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
        container.set_css_classes(&["utility-section"]);
        container.set_margin_bottom(8);
        container.append(&header);
        container.append(&current_curve_row);
        container.append(&preset_combo_row);
        container.append(&fallback_row);
        container.append(&link_row);
        container.append(&scrolled);

        let panel = Rc::new(RefCell::new(Self {
            container,
            list_box,
            add_button,
            delete_button,
            save_button,
            import_button,
            export_button,
            revert_button,
            reset_button,
            current_curve_label,
            state_chip,
            preset_combo,
            fallback_button,
            fallback_label,
            link_button,
            link_label,
            current_bands: Vec::new(),
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

        let panel_clone = panel.clone();
        panel.borrow().add_button.connect_clicked(move |_| {
            let list_box = &panel_clone.borrow().list_box;
            let count = list_box
                .first_child()
                .map_or(0, |_| count_children(list_box));
            let name = format!("preset_{}", count + 1);
            let sanitized = sanitize_preset_name(&name);
            let path = preset_path_for_name(&sanitized);
            let bands = default_bands();
            let _ = save_preset_to_file(&path, &bands, 0.0);
            refresh_preset_list(list_box);
        });

        let panel_clone = panel.clone();
        panel.borrow().delete_button.connect_clicked(move |_| {
            let list_box = &panel_clone.borrow().list_box;
            if let Some(row) = list_box.selected_row() {
                if let Some(child) = row.child() {
                    if let Some(label) = child.downcast_ref::<gtk4::Label>() {
                        let name = label.label();
                        let path = preset_path_for_name(&name);
                        let _ = std::fs::remove_file(&path);
                    }
                }
                refresh_preset_list(list_box);
            }
        });

        let panel_clone = panel.clone();
        panel.borrow().save_button.connect_clicked(move |_| {
            let panel_ref = panel_clone.borrow();
            let list_box = &panel_ref.list_box;
            if let Some(row) = list_box.selected_row() {
                if let Some(child) = row.child() {
                    if let Some(label) = child.downcast_ref::<gtk4::Label>() {
                        let name = label.label();
                        let path = preset_path_for_name(&name);
                        let bands = panel_ref.current_bands.clone();
                        let preamp = panel_ref.current_preamp_db;
                        let _ = save_preset_to_file(&path, &bands, preamp);
                    }
                }
            }
        });

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
                        let path = file.path();
                        if let Some(path) = path {
                            if let Ok((_preamp, bands)) = parse_apo_file(&path) {
                                let count = list_box_for_import
                                    .first_child()
                                    .map_or(0, |_| count_children(&list_box_for_import));
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

        let panel_clone = panel.clone();
        panel.borrow().export_button.connect_clicked(move |_| {
            let panel_ref = panel_clone.borrow();
            let list_box = &panel_ref.list_box;
            if let Some(row) = list_box.selected_row() {
                if let Some(child) = row.child() {
                    if let Some(label) = child.downcast_ref::<gtk4::Label>() {
                        let name = label.label();
                        let path = preset_path_for_name(&name);
                        if let Ok((preamp, bands)) = load_preset_from_file(&path) {
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
                            dialog.set_current_name(&format!("{}.{}", name, PRESET_FILE_SUFFIX));

                            let filter = gtk4::FileFilter::new();
                            filter.add_pattern(&format!("*.{}", PRESET_FILE_SUFFIX));
                            filter.set_name(Some("Preset Files"));
                            dialog.add_filter(&filter);

                            let bands_for_export = bands.clone();
                            let preamp_for_export = preamp;
                            dialog.connect_response(move |d, response| {
                                if response == gtk4::ResponseType::Accept {
                                    if let Some(file) = d.file() {
                                        let dest = file.path();
                                        if let Some(dest) = dest {
                                            let _ = save_preset_to_file(&dest, &bands_for_export, preamp_for_export);
                                        }
                                    }
                                }
                                d.close();
                            });

                             dialog.show();
                        }
                    }
                }
            }
        });

        let panel_clone = panel.clone();
        panel.borrow().revert_button.connect_clicked(move |_| {
            let mut panel_ref = panel_clone.borrow_mut();
            panel_ref.revert_to_baseline();
            panel_ref.refresh_list();
        });

        let panel_clone = panel.clone();
        panel.borrow().reset_button.connect_clicked(move |_| {
            let mut panel_ref = panel_clone.borrow_mut();
            panel_ref.reset_to_neutral();
            panel_ref.refresh_list();
        });

        let panel_clone = panel.clone();
        panel.borrow().preset_combo.connect_changed(move |combo| {
            let selected = combo.active();
            if selected != Some(0) {
                return;
            }
            if let Some(name) = combo.active_text() {
                let mut panel_ref = panel_clone.borrow_mut();
                if let Err(e) = panel_ref.load_library_preset(&name) {
                    eprintln!("Failed to load preset: {}", e);
                }
                panel_ref.refresh_list();
            }
        });

        let panel_clone = panel.clone();
        panel.borrow().list_box.connect_row_selected(move |_, row| {
            if let Some(row) = row {
                if let Some(child) = row.child() {
                    if let Some(label) = child.downcast_ref::<gtk4::Label>() {
                        let name = label.label();
                        let mut panel_mut = panel_clone.borrow_mut();
                        if let Ok((preamp, bands)) = load_preset_from_file(&preset_path_for_name(&name)) {
                            panel_mut.current_bands = bands;
                            panel_mut.current_preamp_db = preamp;
                            panel_mut.current_preset_name = Some(name.to_string());
                            panel_mut.update_state_chip();
                        }
                    }
                }
            }
        });

        let panel_clone = panel.clone();
        panel.borrow().fallback_button.connect_clicked(move |_| {
            let panel_ref = panel_clone.borrow();
            if let Some(name) = panel_ref.current_preset_name.as_deref() {
                if let Err(e) = crate::core::set_output_preset_fallback_name(name) {
                    eprintln!("Failed to set fallback preset: {}", e);
                } else {
                    panel_clone.borrow().fallback_label.set_text(name);
                }
            }
        });

        let panel_clone = panel.clone();
        panel.borrow().link_button.connect_clicked(move |_| {
            let panel_ref = panel_clone.borrow();
            if let Some(name) = panel_ref.current_preset_name.as_deref() {
                if let Err(e) = crate::core::set_output_preset_link("default", name) {
                    eprintln!("Failed to link output preset: {}", e);
                } else {
                    panel_clone.borrow().link_label.set_text(name);
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

    pub fn start_file_monitoring(&mut self) {
        let dir = crate::core::ensure_preset_storage_dir();
        let file = gio::File::for_path(&dir);
        if let Ok(monitor) = file.monitor_directory(gio::FileMonitorFlags::NONE, gio::Cancellable::NONE) {
            let list_box = self.list_box.clone();
            let handler = monitor.connect_changed(move |_, _, _, _| {
                refresh_preset_list(&list_box);
            });
            self.file_monitor = Some(handler);
        }
    }

    pub fn refresh_list(&self) {
        refresh_preset_list(&self.list_box);
        self.refresh_preset_combo();
    }

    fn refresh_preset_combo(&self) {
        let names = list_preset_names();
        self.preset_combo.remove_all();
        self.preset_combo.append(Some("-- Select Preset --"), "");
        for name in &names {
            self.preset_combo.append(Some(name.as_str()), name.as_str());
        }
        self.preset_combo.set_active(Some(0));
    }

    pub fn set_bands_and_preamp(&mut self, bands: &[crate::core::EqBand], preamp_db: f64) {
        self.current_bands = bands.to_vec();
        self.current_preamp_db = preamp_db;
        let mut labels = Vec::new();
        for band in bands.iter().take(10) {
            if band.enabled && band.filter_type != crate::core::FilterType::Off {
                labels.push(format!(
                    "{} {}dB",
                    format_frequency(band.frequency),
                    band.gain_db.round() as i64
                ));
            }
        }
        let text = if labels.is_empty() {
            "Neutral".to_string()
        } else {
            labels.join(" / ")
        };
        self.current_curve_label.set_text(&text);
    }

    pub fn update_state_chip(&mut self) {
        let signature = self.get_signature_callback.as_ref().map(|f| f()).unwrap_or_default();
        let current_name = self.current_preset_name.as_deref();
        let saved_sig = self.saved_signature.as_deref();

        if current_name.is_some() && signature == saved_sig.unwrap_or(&signature) {
            self.state_chip.set_text("Saved");
            self.state_chip.set_css_classes(&["preset-state-saved"]);
        } else if current_name.is_some() {
            self.state_chip.set_text("Modified");
            self.state_chip.set_css_classes(&["preset-state-modified"]);
        } else if signature == self.default_signature.as_deref().unwrap_or(&signature) {
            self.state_chip.set_text("Neutral");
            self.state_chip.set_css_classes(&["preset-state-neutral"]);
        } else {
            self.state_chip.set_text("Unsaved");
            self.state_chip.set_css_classes(&["preset-state-unsaved"]);
        }
    }

    pub fn load_library_preset(&mut self, name: &str) -> anyhow::Result<()> {
        let preset_name = sanitize_preset_name(name);
        if preset_name.is_empty() {
            anyhow::bail!("preset name is empty");
        }
        let path = preset_path_for_name(&preset_name);
        let (preamp, bands) = load_preset_from_file(&path)?;
        if let Some(ref apply) = self.apply_bands_callback {
            apply(bands.clone(), preamp);
        }
        self.current_bands = bands;
        self.current_preamp_db = preamp;
        self.current_preset_name = Some(preset_name.clone());
        self.saved_signature = Some(self.get_signature_callback.as_ref().map(|f| f()).unwrap_or_default());
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

fn count_children(list_box: &gtk4::ListBox) -> usize {
    let mut n = 0;
    let mut child = list_box.first_child();
    while let Some(c) = child {
        n += 1;
        child = c.next_sibling();
    }
    n
}

fn refresh_preset_list(list_box: &gtk4::ListBox) {
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }
    for name in list_preset_names() {
        let row = gtk4::ListBoxRow::new();
        let label = gtk4::Label::new(Some(&name));
        label.set_halign(gtk4::Align::Start);
        row.set_child(Some(&label));
        list_box.append(&row);
    }
}

/// Get the preset storage directory.
pub fn preset_storage_dir() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".config/mini-eq/presets")
}

/// List all preset names (sorted).
pub fn list_preset_names() -> Vec<String> {
    let dir = ensure_preset_storage_dir();
    let mut names = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == PRESET_FILE_SUFFIX.strip_prefix('.').unwrap_or("json"))
                && let Some(stem) = path.file_stem()
            {
                names.push(stem.to_string_lossy().to_string());
            }
        }
    }
    names.sort();
    names
}


