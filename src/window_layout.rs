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

fn build_output_control_row(utility: &UtilityPane) -> adw::WrapBox {
    // WrapBox rather than a plain Box: at narrow widths items wrap onto a
    // second line instead of being clipped, so nothing is ever cut off.
    // Explicit child/line spacing: the WrapBox default is tight enough that
    // the switches read as one merged blob once the row wraps.
    let row = adw::WrapBox::builder()
        .child_spacing(14)
        .line_spacing(12)
        .build();
    row.set_orientation(gtk4::Orientation::Horizontal);
    row.set_css_classes(&["output-control-row"]);
    row.set_halign(gtk4::Align::Center);
    row.set_hexpand(true);
    row.set_valign(gtk4::Align::Center);

    let headroom = utility.headroom.borrow();
    let mut labels: Vec<gtk4::Label> = Vec::new();

    let auto_safe_item = control(
        "Auto-Safe",
        "Let the output preamp follow the peak automatically",
        &headroom.auto_safe_switch,
        &mut labels,
    );
    let smooth_item = control(
        "Smooth",
        "Dragging one band drags its neighbours so the curve stays smooth",
        &headroom.smooth_switch,
        &mut labels,
    );
    let smooth_width_item = control(
        "Width",
        "How many bands move when you drag one",
        &headroom.smooth_width_scale,
        &mut labels,
    );
    let preamp_item = control(
        "Preamp",
        "Output preamp trim (dB)",
        &headroom.preamp_scale,
        &mut labels,
    );

    row.append(&auto_safe_item);
    row.append(&smooth_item);
    // Only meaningful while Smooth is on; hidden otherwise (see below).
    row.append(&smooth_width_item);
    // Hidden while Auto-Safe owns the preamp.
    row.append(&preamp_item);

    // Status LED: 18px instead of the old ~90px bar meter, which was what
    // forced the row onto a second line with every switch active.
    row.append(&headroom.led_area);

    headroom.peak_label.set_valign(gtk4::Align::Center);
    row.append(&headroom.peak_label);

    headroom.set_safe_button.set_valign(gtk4::Align::Center);
    row.append(&headroom.set_safe_button);

    // --- visibility rules (self-contained) -------------------------------
    // Width only applies while Smooth is on; the preamp control is hidden
    // while Auto-Safe owns it, rather than sitting there greyed out.
    {
        let width_item = smooth_width_item.clone();
        headroom.smooth_switch.connect_state_set(move |_sw, on| {
            width_item.set_visible(on);
            glib::Propagation::Proceed
        });
    }
    {
        let preamp_item = preamp_item.clone();
        headroom.auto_safe_switch.connect_state_set(move |_sw, on| {
            preamp_item.set_visible(!on);
            glib::Propagation::Proceed
        });
    }
    smooth_width_item.set_visible(headroom.smooth.get());
    preamp_item.set_visible(!headroom.auto_safe.get());

    // Drop the captions when the row gets tight. Every control carries its
    // own tooltip, so the names are still reachable.
    {
        let labels: Rc<Vec<gtk4::Label>> = Rc::new(labels);
        // Guard so the visibility change we trigger does not re-enter this
        // handler and thrash the layout.
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

/// A captioned control in the output row. The tooltip is set on the whole
/// item so it still identifies the control once the caption is dropped.
fn control(
    caption: &str,
    tooltip: &str,
    widget: &impl IsA<gtk4::Widget>,
    labels: &mut Vec<gtk4::Label>,
) -> gtk4::Box {
    let box_ = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    box_.set_tooltip_text(Some(tooltip));
    let label = gtk4::Label::new(Some(caption));
    label.set_valign(gtk4::Align::Center);
    label.set_css_classes(&["metric-title"]);
    box_.append(&label);
    box_.append(widget);
    labels.push(label);
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
