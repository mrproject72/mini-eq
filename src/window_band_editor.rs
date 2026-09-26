//! Selected-band editor.
//!
//! Mirrors upstream `window_layout.py` (`band_editor`) and the handler pair in
//! `window_graph.py` (`update_selected_band_editor` + `on_selected_band_*`).
//! The editor is a view over whichever band the fader strip currently has
//! selected; it owns no band state of its own.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;

use crate::band_fader::EqBandFader;
use crate::core::{
    EQ_FREQUENCY_MAX_HZ, EQ_FREQUENCY_MIN_HZ, EQ_GAIN_MAX_DB, EQ_GAIN_MIN_DB, EQ_Q_MAX, EQ_Q_MIN,
    FilterType, SELECTABLE_FILTER_TYPES, filter_type_combo_index, filter_type_from_combo_index,
};

/// Upstream `SELECTED_BAND_PLACEHOLDER_FREQUENCY_HZ`: shown while nothing is
/// selected so the spin buttons keep a plausible value.
const PLACEHOLDER_FREQUENCY_HZ: f64 = 1000.0;

/// Edits requested by the user, forwarded to the window that owns band state.
pub struct BandEditorCallbacks {
    pub frequency_changed: Box<dyn Fn(usize, f64)>,
    pub q_changed: Box<dyn Fn(usize, f64)>,
    pub gain_changed: Box<dyn Fn(usize, f64)>,
    pub filter_type_changed: Box<dyn Fn(usize, FilterType)>,
    pub mute_changed: Box<dyn Fn(usize, bool)>,
    pub solo_changed: Box<dyn Fn(usize, bool)>,
}

/// The selected-band editor row.
pub struct BandEditor {
    pub container: gtk4::Box,
    title_label: gtk4::Label,
    mute_button: gtk4::ToggleButton,
    solo_button: gtk4::ToggleButton,
    type_combo: gtk4::DropDown,
    frequency_spin: gtk4::SpinButton,
    q_spin: gtk4::SpinButton,
    gain_spin: gtk4::SpinButton,
    /// Index of the band currently shown, or `None` when nothing is selected.
    selected_index: Rc<RefCell<Option<usize>>>,
    /// Set while the editor is being repopulated, so refreshing the controls
    /// does not echo back through the change handlers as a user edit.
    updating: Rc<RefCell<bool>>,
}

impl BandEditor {
    pub fn new(callbacks: BandEditorCallbacks) -> Self {
        // Upstream uses `Adw.WrapBox` here, which needs libadwaita 1.7; the
        // project targets 1.4/1.5 (Ubuntu 24.04 ships 1.5), so a plain
        // horizontal box carries the same controls without the newer binding.
        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        container.set_css_classes(&["band-editor"]);
        container.set_hexpand(true);
        container.set_valign(gtk4::Align::Start);

        // ── Band title ───────────────────────────────────────────────────────
        let title_box = gtk4::Box::new(gtk4::Orientation::Vertical, 1);
        title_box.set_css_classes(&["band-editor-selected"]);
        title_box.set_size_request(88, -1);
        let title_label = gtk4::Label::new(Some("No Band"));
        title_label.set_css_classes(&["band-editor-title"]);
        title_label.set_xalign(0.0);
        title_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        title_label.set_max_width_chars(8);
        title_box.append(&title_label);
        container.append(&title_box);

        // ── Mute / Solo ──────────────────────────────────────────────────────
        let state_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        state_box.set_css_classes(&["band-editor-state"]);
        state_box.set_valign(gtk4::Align::Center);

        let mute_button = gtk4::ToggleButton::with_label("M");
        mute_button.set_css_classes(&["band-editor-toggle"]);
        mute_button.set_tooltip_text(Some("Mute Selected Band"));
        state_box.append(&mute_button);

        let solo_button = gtk4::ToggleButton::with_label("S");
        solo_button.set_css_classes(&["band-editor-toggle"]);
        solo_button.set_tooltip_text(Some("Solo Selected Band"));
        state_box.append(&solo_button);
        container.append(&state_box);

        // ── Filter type ──────────────────────────────────────────────────────
        let type_labels: Vec<&str> = SELECTABLE_FILTER_TYPES
            .iter()
            .map(|filter_type| filter_type.name())
            .collect();
        let type_combo = gtk4::DropDown::new(
            Some(gtk4::StringList::new(&type_labels)),
            None::<gtk4::Expression>,
        );
        type_combo.set_size_request(118, -1);
        type_combo.set_css_classes(&["band-editor-input"]);
        container.append(&field("Type", &type_combo));

        // ── Frequency ────────────────────────────────────────────────────────
        let frequency_spin =
            gtk4::SpinButton::with_range(EQ_FREQUENCY_MIN_HZ, EQ_FREQUENCY_MAX_HZ, 0.1);
        frequency_spin.set_digits(1);
        frequency_spin.set_size_request(110, -1);
        frequency_spin.set_css_classes(&["band-editor-input"]);
        container.append(&field("Frequency", &frequency_spin));

        // ── Q ────────────────────────────────────────────────────────────────
        let q_spin = gtk4::SpinButton::with_range(EQ_Q_MIN, EQ_Q_MAX, 0.001);
        q_spin.set_digits(3);
        q_spin.set_size_request(82, -1);
        q_spin.set_css_classes(&["band-editor-input"]);
        container.append(&field("Q", &q_spin));

        // ── Gain ─────────────────────────────────────────────────────────────
        let gain_spin = gtk4::SpinButton::with_range(EQ_GAIN_MIN_DB, EQ_GAIN_MAX_DB, 0.1);
        gain_spin.set_digits(1);
        gain_spin.set_size_request(96, -1);
        gain_spin.set_css_classes(&["band-editor-input"]);
        container.append(&field("Gain", &gain_spin));

        let selected_index = Rc::new(RefCell::new(Option::<usize>::None));
        let updating = Rc::new(RefCell::new(false));

        {
            let index = selected_index.clone();
            let updating = updating.clone();
            let callback = callbacks.filter_type_changed;
            type_combo.connect_selected_notify(move |combo| {
                if *updating.borrow() {
                    return;
                }
                let Some(index) = *index.borrow() else {
                    return;
                };
                callback(
                    index,
                    filter_type_from_combo_index(combo.selected() as usize),
                );
            });
        }
        {
            let index = selected_index.clone();
            let updating = updating.clone();
            let callback = callbacks.frequency_changed;
            frequency_spin.connect_value_changed(move |spin| {
                if *updating.borrow() {
                    return;
                }
                let Some(index) = *index.borrow() else {
                    return;
                };
                callback(index, spin.value());
            });
        }
        {
            let index = selected_index.clone();
            let updating = updating.clone();
            let callback = callbacks.q_changed;
            q_spin.connect_value_changed(move |spin| {
                if *updating.borrow() {
                    return;
                }
                let Some(index) = *index.borrow() else {
                    return;
                };
                callback(index, spin.value());
            });
        }
        {
            let index = selected_index.clone();
            let updating = updating.clone();
            let callback = callbacks.gain_changed;
            gain_spin.connect_value_changed(move |spin| {
                if *updating.borrow() {
                    return;
                }
                let Some(index) = *index.borrow() else {
                    return;
                };
                callback(index, spin.value());
            });
        }
        {
            let index = selected_index.clone();
            let updating = updating.clone();
            let callback = callbacks.mute_changed;
            mute_button.connect_toggled(move |button| {
                if *updating.borrow() {
                    return;
                }
                let Some(index) = *index.borrow() else {
                    return;
                };
                callback(index, button.is_active());
            });
        }
        {
            let index = selected_index.clone();
            let updating = updating.clone();
            let callback = callbacks.solo_changed;
            solo_button.connect_toggled(move |button| {
                if *updating.borrow() {
                    return;
                }
                let Some(index) = *index.borrow() else {
                    return;
                };
                callback(index, button.is_active());
            });
        }

        Self {
            container,
            title_label,
            mute_button,
            solo_button,
            type_combo,
            frequency_spin,
            q_spin,
            gain_spin,
            selected_index,
            updating,
        }
    }

    /// Repopulate the editor from the selected fader, or blank it out when
    /// nothing is selected. Mirrors upstream `update_selected_band_editor`.
    pub fn refresh(&self, fader: Option<&EqBandFader>) {
        *self.updating.borrow_mut() = true;
        *self.selected_index.borrow_mut() = fader.map(|f| f.index);

        match fader {
            None => {
                self.title_label.set_text("No Band");
                self.title_label.set_tooltip_text(Some("No band selected"));
                self.type_combo
                    .set_selected(filter_type_combo_index(FilterType::Off) as u32);
                self.frequency_spin.set_value(PLACEHOLDER_FREQUENCY_HZ);
                self.q_spin.set_value(crate::core::DEFAULT_BAND_Q);
                self.gain_spin.set_value(0.0);
                self.mute_button.set_active(false);
                self.solo_button.set_active(false);
                self.set_controls_sensitive(false);
            }
            Some(fader) => {
                let band_title = format!("Band {}", fader.index + 1);
                self.title_label.set_text(&band_title);
                self.title_label.set_tooltip_text(Some(&format!(
                    "{} • {} • {:.1} Hz • Q {:.3} • {:+.1} dB",
                    band_title,
                    fader.filter_type.name(),
                    fader.frequency,
                    fader.q_value,
                    fader.gain_db
                )));
                self.type_combo
                    .set_selected(filter_type_combo_index(fader.filter_type) as u32);
                self.frequency_spin.set_value(fader.frequency);
                self.q_spin.set_value(fader.q_value);
                self.gain_spin.set_value(fader.gain_db);
                self.mute_button.set_active(fader.muted);
                self.solo_button.set_active(fader.soloed);
                self.set_controls_sensitive(true);
            }
        }

        *self.updating.borrow_mut() = false;
    }

    fn set_controls_sensitive(&self, sensitive: bool) {
        self.type_combo.set_sensitive(sensitive);
        self.frequency_spin.set_sensitive(sensitive);
        self.q_spin.set_sensitive(sensitive);
        self.gain_spin.set_sensitive(sensitive);
        self.mute_button.set_sensitive(sensitive);
        self.solo_button.set_sensitive(sensitive);
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }
}

/// Wrap a labelled control in the upstream `band-editor-field` shell.
fn field(label: &str, control: &impl IsA<gtk4::Widget>) -> gtk4::Box {
    let shell = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    shell.set_css_classes(&["band-editor-field"]);
    shell.set_valign(gtk4::Align::Center);

    let label = gtk4::Label::new(Some(label));
    label.set_css_classes(&["metric-title"]);
    label.set_xalign(0.0);
    label.set_mnemonic_widget(Some(control));
    shell.append(&label);
    shell.append(control);
    shell
}
