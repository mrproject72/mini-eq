//! Main layout for the mini-eq application window.

use gtk4::prelude::*;

use crate::core::{DEFAULT_ACTIVE_BANDS, MAX_BANDS};
use crate::window_band_fader::WindowBandFader;
use crate::window_utility::UtilityPane;

use std::cell::RefCell;
use std::rc::Rc;

/// Build the band fader row layout.
pub fn build_band_faders(
    visible_bands: usize,
) -> (
    gtk4::ScrolledWindow,
    Vec<Rc<RefCell<crate::band_fader::EqBandFader>>>,
) {
    let scrolled = gtk4::ScrolledWindow::new();
    scrolled.set_hexpand(true);
    scrolled.set_vexpand(true);
    scrolled.set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Automatic);
    scrolled.set_min_content_height(200);

    let band_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 2);
    band_box.set_css_classes(&["band-fader-container"]);
    band_box.set_hexpand(true);
    band_box.set_vexpand(true);

    let mut faders = Vec::new();
    let count = visible_bands.clamp(1, MAX_BANDS);
    // Upstream `compute_log_spaced_band_defaults` derives both frequency and Q
    // from the band count, so the row spans 20 Hz..20 kHz across `count` bands.
    let defaults = crate::core::compute_log_spaced_band_defaults(count);
    for (i, (frequency, q)) in defaults.into_iter().enumerate() {
        let band = WindowBandFader::new(
            i,
            frequency,
            0.0,
            q,
            crate::core::FilterType::Off,
            i < DEFAULT_ACTIVE_BANDS,
        );
        faders.push(band.fader.clone());
        band_box.append(band.widget());
    }

    scrolled.set_child(Some(&band_box));
    (scrolled, faders)
}

/// Build the shared selected-band editor.
pub fn build_band_editor() -> gtk4::Box {
    let editor = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    editor.set_css_classes(&["band-editor"]);

    let mute_toggle = gtk4::ToggleButton::with_label("Mute");
    let solo_toggle = gtk4::ToggleButton::with_label("Solo");

    let type_label = gtk4::Label::new(Some("Type"));
    let type_combo = gtk4::ComboBoxText::new();
    for ft in crate::core::SELECTABLE_FILTER_TYPES.iter() {
        type_combo.append_text(crate::band_fader::filter_type_short_label(*ft));
    }

    let freq_adj = gtk4::Adjustment::new(
        1000.0,
        crate::core::EQ_FREQUENCY_MIN_HZ,
        crate::core::EQ_FREQUENCY_MAX_HZ,
        10.0,
        100.0,
        0.0,
    );
    let freq_spin = gtk4::SpinButton::new(Some(&freq_adj), 10.0, 0);
    freq_spin.set_digits(0);
    freq_spin.set_width_chars(7);

    let q_adj = gtk4::Adjustment::new(1.0, 0.1, 10.0, 0.1, 0.5, 0.0);
    let q_spin = gtk4::SpinButton::new(Some(&q_adj), 0.1, 1);
    q_spin.set_digits(2);
    q_spin.set_width_chars(5);

    let gain_adj = gtk4::Adjustment::new(
        0.0,
        crate::core::EQ_GAIN_MIN_DB,
        crate::core::EQ_GAIN_MAX_DB,
        0.5,
        1.0,
        0.0,
    );
    let gain_spin = gtk4::SpinButton::new(Some(&gain_adj), 0.5, 1);
    gain_spin.set_digits(1);
    gain_spin.set_width_chars(6);

    editor.append(&mute_toggle);
    editor.append(&solo_toggle);
    editor.append(&type_label);
    editor.append(&type_combo);
    editor.append(&freq_spin);
    editor.append(&q_spin);
    editor.append(&gain_spin);

    editor
}

/// Build the main content layout with left panel (band faders) and right panel (utility).
pub fn build_main_layout(
    utility: &UtilityPane,
    visible_bands: usize,
) -> (
    adw::OverlaySplitView,
    gtk4::ScrolledWindow,
    Vec<Rc<RefCell<crate::band_fader::EqBandFader>>>,
) {
    let main_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);

    main_box.append(utility.graph.borrow().widget());

    let (band_scrolled, faders) = build_band_faders(visible_bands);
    main_box.append(&band_scrolled);

    let band_editor = build_band_editor();
    main_box.append(&band_editor);

    let split_view = adw::OverlaySplitView::new();
    split_view.set_content(Some(&main_box));
    split_view.set_sidebar(Some(&utility.container));
    split_view.set_pin_sidebar(true);
    split_view.set_collapsed(false);
    split_view.set_sidebar_width_fraction(0.24);
    split_view.set_min_sidebar_width(268.0);
    split_view.set_max_sidebar_width(320.0);

    (split_view, band_scrolled, faders)
}
