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
///
/// `gain_changed` is the owner-authoritative gain request handed to every
/// fader: it returns the gain the band may actually use. It MUST be threaded
/// all the way down — a stub here would silently disable peak safety for all
/// fader gestures (drag/scroll/keys) while leaving the editor spin clamped,
/// which is exactly how stacked shelves escaped the limit.
pub fn build_band_faders(
    visible_bands: usize,
    gain_changed: Rc<dyn Fn(usize, f64) -> f64>,
    selection_changed_callback: Rc<dyn Fn(usize)>,
) -> (
    gtk4::ScrolledWindow,
    Vec<Rc<RefCell<crate::band_fader::EqBandFader>>>,
) {
    let scrolled = gtk4::ScrolledWindow::new();
    scrolled.set_hexpand(true);
    scrolled.set_vexpand(true);
    // Horizontal: NEVER scroll. The fader row is homogeneous + hexpand, so it
    // is forced to the viewport width and shrinks to fit. This prevents the
    // row from keeping its natural (72*10=738px) width and overflowing the
    // window — which was pushing the overlay side panel outside the viewport.
    scrolled.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
    scrolled.set_min_content_height(200);

    let band_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 2);
    band_box.set_css_classes(&["band-fader-container"]);
    band_box.set_hexpand(true);
    band_box.set_vexpand(true);
    // Equal-width faders that share the available horizontal space, so the
    // row shrinks to fit a narrow window instead of overflowing it.
    band_box.set_homogeneous(true);

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
            gain_changed.clone(),
            selection_changed_callback.clone(),
        );
        faders.push(band.fader.clone());
        band_box.append(band.widget());
    }

    scrolled.set_child(Some(&band_box));
    (scrolled, faders)
}

/// The main-window output control row: Auto-Safe, A/B compare, preamp,
/// live peak meter and the Set Safe button.
///
/// Every control is reparented from the sidebar panels, so the sidebar can
/// hold only output-device settings. `Set Safe` is shown only when the
/// curve is at risk (see `HeadroomPanel::update_peak`), and it — not the
/// header icon — carries the clipping alert.
/// Below this row width the captions are dropped and each control falls
/// back to its tooltip, so the row stays compact instead of being cut.
const OUTPUT_ROW_COMPACT_WIDTH: i32 = 1000;

/// Fixed cell widths for the output row (see `fixed_cell`).
// Sized so the whole row fits MIN_WINDOW_WIDTH (640) WITHOUT wrapping.
// It previously overflowed by ~16px, which pushed Set Safe onto a second
// line that the row height could not show -- that is why it looked like the
// button was missing rather than merely insensitive.
const CELL_W_SMOOTH: i32 = 100;
const CELL_W_AUTO_SAFE: i32 = 128;
const CELL_W_PREAMP: i32 = 130;
const CELL_W_STATUS: i32 = 112;
const CELL_W_SET_SAFE: i32 = 100;

fn build_output_control_row(utility: &UtilityPane) -> adw::WrapBox {
    // WrapBox rather than a plain Box: at narrow widths items wrap onto a
    // second line instead of being clipped, so nothing is ever cut off.
    let row = adw::WrapBox::builder()
        .child_spacing(12)
        .line_spacing(10)
        .build();
    row.set_orientation(gtk4::Orientation::Horizontal);
    row.set_css_classes(&["output-control-row"]);
    row.set_halign(gtk4::Align::Center);
    row.set_hexpand(true);
    row.set_valign(gtk4::Align::Center);

    let headroom = utility.headroom.borrow();
    let mut labels: Vec<gtk4::Label> = Vec::new();

    // Every cell has a FIXED width. Previously the cells sized to their
    // content, so hiding the preamp or the width control slid everything
    // else sideways. Fixed cells mean a control appearing or vanishing never
    // moves its neighbours, and the wrap points are deterministic too.
    let auto_safe_item = fixed_cell(
        CELL_W_AUTO_SAFE,
        Some((
            "Auto-Safe",
            "Let the output preamp follow the peak automatically",
        )),
        &headroom.auto_safe_switch,
        &mut labels,
    );
    // One cell for Smooth: the switch and its width spin live inside the
    // menu popover, so the row spends a single cell on them.
    let smooth_item = fixed_cell(CELL_W_SMOOTH, None, &headroom.smooth_menu, &mut labels);
    let preamp_item = fixed_cell(
        CELL_W_PREAMP,
        Some(("Preamp", "Output preamp trim (dB)")),
        &headroom.preamp_scale,
        &mut labels,
    );

    // Smooth first, then Auto-Safe: the dropdown is the control that
    // changes how dragging behaves, so it leads the row.
    row.append(&smooth_item);
    row.append(&auto_safe_item);
    row.append(&preamp_item);

    // Status cell: LED + numeric peak, grouped so they never separate on wrap.
    let status_cell = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    status_cell.set_size_request(CELL_W_STATUS, -1);
    status_cell.set_halign(gtk4::Align::Start);
    status_cell.set_valign(gtk4::Align::Center);
    status_cell.append(&headroom.led_area);
    headroom.peak_label.set_valign(gtk4::Align::Center);
    status_cell.append(&headroom.peak_label);
    row.append(&status_cell);

    headroom.set_safe_button.set_valign(gtk4::Align::Center);
    let set_safe_cell = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    set_safe_cell.set_size_request(CELL_W_SET_SAFE, -1);
    set_safe_cell.set_halign(gtk4::Align::Start);
    set_safe_cell.set_valign(gtk4::Align::Center);
    set_safe_cell.append(&headroom.set_safe_button);
    row.append(&set_safe_cell);

    // The preamp control is hidden while Auto-Safe owns it. The CELL stays
    // its fixed width, so nothing shifts.
    {
        let preamp = headroom.preamp_scale.clone();
        headroom.auto_safe_switch.connect_state_set(move |_sw, on| {
            preamp.set_visible(!on);
            glib::Propagation::Proceed
        });
    }
    preamp_item.set_visible(true);
    headroom.preamp_scale.set_visible(!headroom.auto_safe.get());

    // Drop the captions when the row gets tight. Every control carries its
    // own tooltip, so the names are still reachable.
    {
        let labels: Rc<Vec<gtk4::Label>> = Rc::new(labels);
        let current = Rc::new(std::cell::Cell::new(false));
        row.connect_notify_local(Some("width"), move |r, _| {
            let compact = r.width() > 0 && r.width() < OUTPUT_ROW_COMPACT_WIDTH;
            if current.get() == compact {
                return;
            }
            current.set(compact);
            for label in labels.iter() {
                label.set_visible(!compact);
            }
        });
    }

    row
}

/// A fixed-width cell in the output row. The width is reserved whether or
/// not the caption is showing and whether or not the inner control is
/// visible, which is what keeps the row from reflowing.
fn fixed_cell(
    width: i32,
    caption: Option<(&str, &str)>,
    widget: &impl IsA<gtk4::Widget>,
    labels: &mut Vec<gtk4::Label>,
) -> gtk4::Box {
    let box_ = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    box_.set_size_request(width, -1);
    box_.set_halign(gtk4::Align::Start);
    box_.set_valign(gtk4::Align::Center);
    if let Some((text, tooltip)) = caption {
        box_.set_tooltip_text(Some(tooltip));
        let label = gtk4::Label::new(Some(text));
        label.set_valign(gtk4::Align::Center);
        label.set_css_classes(&["metric-title"]);
        box_.append(&label);
        labels.push(label);
    }
    box_.append(widget);
    box_
}

/// Build the main content layout with left panel (band faders) and right panel (utility).
pub fn build_main_layout(
    utility: &UtilityPane,
    editor: &crate::window_band_editor::BandEditor,
    visible_bands: usize,
    gain_changed: Rc<dyn Fn(usize, f64) -> f64>,
    selection_changed_callback: Rc<dyn Fn(usize)>,
) -> (
    adw::OverlaySplitView,
    gtk4::ScrolledWindow,
    Vec<Rc<RefCell<crate::band_fader::EqBandFader>>>,
) {
    let main_box = gtk4::Box::new(gtk4::Orientation::Vertical, 4);

    main_box.append(utility.graph.borrow().widget());

    // Output control row: the headroom/bypass controls the user reaches for
    // constantly, on ONE row between the spectrum and the faders. These
    // widgets stay owned by HeadroomPanel/UtilityPane and are reparented
    // here, which frees the sidebar to be a pure Output (device) panel.
    let control_row = build_output_control_row(utility);
    main_box.append(&control_row);

    let (band_scrolled, faders) =
        build_band_faders(visible_bands, gain_changed, selection_changed_callback);
    main_box.append(&band_scrolled);

    main_box.append(editor.widget());

    let split_view = adw::OverlaySplitView::new();
    split_view.set_content(Some(&main_box));
    split_view.set_sidebar(Some(&utility.container));
    // Overlay mode: collapsed=TRUE means the sidebar is shown as an OVERLAY
    // above the content, so the main content is ALWAYS full width. The panel
    // never squeezes the content. Visibility is driven by `show-sidebar`.
    split_view.set_collapsed(true);
    // We control visibility explicitly (via the toggle / F9); don't let the
    // split view change it on its own.
    split_view.set_pin_sidebar(true);
    // Hidden by default: main content uses the full window width.
    split_view.set_show_sidebar(false);
    // Keep the utility panel on the RIGHT at every window size.
    split_view.set_sidebar_position(gtk4::PackType::End);
    split_view.set_sidebar_width_fraction(0.32);
    split_view.set_min_sidebar_width(300.0);
    split_view.set_max_sidebar_width(440.0);

    (split_view, band_scrolled, faders)
}
