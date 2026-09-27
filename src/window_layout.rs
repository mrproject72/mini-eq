//! Main layout for the mini-eq application window.

use gtk4::prelude::*;

use crate::core::{DEFAULT_ACTIVE_BANDS, MAX_BANDS};
use crate::window_band_fader::WindowBandFader;
use crate::window_utility::UtilityPane;

use std::cell::RefCell;
use std::rc::Rc;

/// Build the band fader row layout.
///
/// `selection_changed_callback` receives the index of the band the user just
/// selected; the owner is responsible for clearing the other faders (a fader
/// cannot do that itself without holding borrows on its siblings).
pub fn build_band_faders(
    visible_bands: usize,
    selection_changed_callback: Rc<dyn Fn(usize)>,
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
        // Active bands must default to Bell (matching `core::default_bands()`).
        // With `Off` the wet/dry mix is 0, so dragging the fader changes gain
        // on a bypassed biquad and produces no audible effect.
        let filter_type = if i < DEFAULT_ACTIVE_BANDS {
            crate::core::FilterType::Bell
        } else {
            crate::core::FilterType::Off
        };
        let band = WindowBandFader::new(
            i,
            frequency,
            0.0,
            q,
            filter_type,
            i < DEFAULT_ACTIVE_BANDS,
            selection_changed_callback.clone(),
        );
        faders.push(band.fader.clone());
        band_box.append(band.widget());
    }

    scrolled.set_child(Some(&band_box));
    (scrolled, faders)
}

/// Build the main content layout with left panel (band faders) and right panel (utility).
pub fn build_main_layout(
    utility: &UtilityPane,
    editor: &crate::window_band_editor::BandEditor,
    visible_bands: usize,
    selection_changed_callback: Rc<dyn Fn(usize)>,
) -> (
    adw::OverlaySplitView,
    gtk4::ScrolledWindow,
    Vec<Rc<RefCell<crate::band_fader::EqBandFader>>>,
) {
    let main_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);

    main_box.append(utility.graph.borrow().widget());

    let (band_scrolled, faders) = build_band_faders(visible_bands, selection_changed_callback);
    main_box.append(&band_scrolled);

    main_box.append(editor.widget());

    let split_view = adw::OverlaySplitView::new();
    split_view.set_content(Some(&main_box));
    split_view.set_sidebar(Some(&utility.container));
    // Overlay mode: the main content keeps the FULL width and the utility
    // panel slides OVER it from the right, instead of squeezing the content
    // inline. Gives the main window maximum space.
    split_view.set_pin_sidebar(false);
    // Hidden by default; revealed via the toggle (F9 / header button) or the
    // right-edge handle.
    split_view.set_collapsed(true);
    // Keep the utility panel on the RIGHT at every window size.
    split_view.set_sidebar_position(gtk4::PackType::End);
    split_view.set_sidebar_width_fraction(0.32);
    split_view.set_min_sidebar_width(300.0);
    split_view.set_max_sidebar_width(440.0);

    (split_view, band_scrolled, faders)
}
