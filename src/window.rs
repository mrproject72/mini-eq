//! Main application window for mini-eq.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;

use glib::ControlFlow;

use crate::appearance::{AppearancePreference, apply_appearance_preference};
use crate::pipewire_backend::PipeWireBackend;
use crate::style;
use crate::window_band_editor::{BandEditor, BandEditorCallbacks};
use crate::window_layout;
use crate::window_state;
use crate::window_utility::UtilityPane;
use crate::window_utils;

/// Snapshot the live fader state as `EqBand`s (the shape the DSP/peak math uses).
///
/// When `smooth` is set the Smooth override is applied on the way out:
/// every non-`Off` band reports as the internal `Sin` bell. `spread_bands`
/// is the spread in bands (Gaussian sigma); the bell Q is derived from it
/// so the bump always bridges whatever bands participate. The faders' own
/// type/Q are never mutated, so switching Smooth off restores them exactly.
fn fader_band_snapshot(
    registry: &Rc<RefCell<Vec<Rc<RefCell<crate::band_fader::EqBandFader>>>>>,
    smooth: bool,
    spread_bands: f64,
) -> Vec<crate::core::EqBand> {
    let bands: Vec<crate::core::EqBand> = registry
        .borrow()
        .iter()
        .map(|f| {
            let fader = f.borrow();
            crate::core::EqBand {
                index: fader.index,
                frequency: fader.frequency,
                gain_db: fader.gain_db,
                q: fader.q_value,
                filter_type: fader.filter_type,
                mute: fader.muted,
                solo: fader.soloed,
                coefficients: crate::core::BiquadCoefficients::identity(),
            }
        })
        .collect();
    if smooth {
        let spacing = crate::core::band_spacing_oct(&bands);
        let q = crate::core::smooth_bell_q(spread_bands, spacing);
        return bands
            .iter()
            .map(|b| crate::core::smooth_effective_band(b, q))
            .collect();
    }
    bands
}

/// Apply `edit` to the fader at `index`, redrawing it when it exists.
fn edit_fader(
    registry: &Rc<RefCell<Vec<Rc<RefCell<crate::band_fader::EqBandFader>>>>>,
    index: usize,
    edit: impl FnOnce(&mut crate::band_fader::EqBandFader),
) {
    let Some(fader) = registry.borrow().get(index).cloned() else {
        return;
    };
    edit(&mut fader.borrow_mut());
    fader.borrow().drawing_area.queue_draw();
}

/// Mirror `solo_active` onto every fader: upstream computes it once from the
/// whole band list (`bands_have_solo`) and hands it to each `set_band_state`.
fn recompute_solo_active(faders: &[Rc<RefCell<crate::band_fader::EqBandFader>>]) {
    let solo_active = faders.iter().any(|fader| fader.borrow().soloed);
    for fader in faders.iter() {
        let mut fader = fader.borrow_mut();
        if fader.solo_active != solo_active {
            fader.solo_active = solo_active;
            fader.drawing_area.queue_draw();
        }
    }
}

/// Main application window.
pub struct MiniEqWindow {
    pub window: adw::ApplicationWindow,
    pub toolbar_view: adw::ToolbarView,
    pub utility: UtilityPane,
    pub split_view: Rc<RefCell<adw::OverlaySplitView>>,
    pub band_scrolled: gtk4::ScrolledWindow,
    pub band_faders: Vec<Rc<RefCell<crate::band_fader::EqBandFader>>>,
}

impl MiniEqWindow {
    pub fn new(
        app: &adw::Application,
        backend: Rc<RefCell<Option<PipeWireBackend>>>,
        engine_sink: String,
    ) -> Self {
        let window = adw::ApplicationWindow::new(app);
        let (default_width, default_height) = window_state::initial_window_default_size();
        window.set_default_size(default_width, default_height);
        // Enforce a minimum so components never get cut, while staying small
        // enough not to dominate a low-res screen.
        window.set_size_request(
            window_state::MIN_WINDOW_WIDTH,
            window_state::MIN_WINDOW_HEIGHT,
        );
        window.set_title(Some("Mini EQ"));

        // Load CSS styling
        style::load_style();

        // Apply appearance
        let settings = crate::appearance::AppearanceSettings::load();
        apply_appearance_preference(AppearancePreference::parse(&settings.preference));

        // Build utility pane
        let utility = UtilityPane::new();

        // A/B compare belongs with the other output toggles at the top of
        // the graph, not on the crowded output control row.
        {
            let ab_item = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
            ab_item.set_tooltip_text(Some("Bypass the EQ to compare with/without"));
            let ab_label = gtk4::Label::new(Some("A/B"));
            ab_label.set_valign(gtk4::Align::Center);
            ab_label.set_css_classes(&["monitor-toggle-label"]);
            ab_item.append(&ab_label);
            ab_item.append(&utility.bypass_switch);
            utility.graph.borrow().add_top_control(&ab_item);
        }

        // Build header bar
        let header_bar = adw::HeaderBar::new();
        let window_title = adw::WindowTitle::new("Mini EQ", "");
        header_bar.set_title_widget(Some(&window_title));

        // The output-device selector now lives in the Headroom panel's
        // "Output Controls" section (see window_utility.rs), not the header.

        // Main menu button
        let menu_button = gtk4::MenuButton::new();
        menu_button.set_icon_name("open-menu-symbolic");
        let menu_model = create_menu_model();
        menu_button.set_menu_model(Some(&menu_model));
        header_bar.pack_end(&menu_button);

        // System-wide EQ toggle with an explicit ON/OFF readout. The bare
        // switch left the routing state ambiguous at a glance, and the
        // tooltip is only reachable with the pointer.
        let route_switch = gtk4::Switch::new();
        route_switch.set_tooltip_text(Some("System-wide EQ"));
        let route_state_label = gtk4::Label::new(Some("Off"));
        route_state_label.set_css_classes(&["metric-title"]);
        route_state_label.set_valign(gtk4::Align::Center);
        route_state_label.set_width_chars(3);
        route_state_label.set_xalign(1.0);
        {
            let lbl = route_state_label.clone();
            // notify::active rather than state-set so programmatic changes
            // (restore-on-startup, menu actions) update the readout too.
            route_switch.connect_notify_local(Some("active"), move |sw, _| {
                let on = sw.is_active();
                lbl.set_label(if on { "On" } else { "Off" });
            });
        }
        let route_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
        route_box.set_valign(gtk4::Align::Center);
        route_box.append(&route_state_label);
        route_box.append(&route_switch);
        header_bar.pack_end(&route_box);

        // Panel switch buttons: each swaps the sidebar to a different panel
        // (Preset / Signal Analyzer / Headroom) and opens it. Mutually
        // exclusive; clicking the active one closes the sidebar. Replaces the
        // old single inspector toggle. Wired to the stack after the layout is
        // built (they need the split_view + utility pane handles).
        let panel_switch_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        panel_switch_box.set_css_classes(&["linked", "panel-switch-group"]);
        let preset_btn = gtk4::ToggleButton::new();
        preset_btn.set_icon_name("view-list-symbolic");
        preset_btn.set_tooltip_text(Some("Presets (F9)"));
        let analyzer_btn = gtk4::ToggleButton::new();
        analyzer_btn.set_icon_name("audio-x-generic-symbolic");
        analyzer_btn.set_tooltip_text(Some("Signal Analyzer"));
        let headroom_btn = gtk4::ToggleButton::new();
        headroom_btn.set_icon_name("audio-volume-high-symbolic");
        headroom_btn.set_tooltip_text(Some("Output / Device"));
        panel_switch_box.append(&preset_btn);
        panel_switch_box.append(&analyzer_btn);
        panel_switch_box.append(&headroom_btn);
        header_bar.pack_start(&panel_switch_box);

        // Build main layout with utility pane.
        //
        // Selection is coordinated here rather than inside a fader: the clicked
        // fader reports the index and the owner clears its siblings and mirrors
        // the choice into the response graph. The registry is filled in after
        // the layout is built, so the closure captures an empty cell for now.
        let fader_registry: Rc<RefCell<Vec<Rc<RefCell<crate::band_fader::EqBandFader>>>>> =
            Rc::new(RefCell::new(Vec::new()));
        // Filled once the editor exists; the callbacks below are created first
        // so they can be handed to the editor's constructor.
        let editor_cell: Rc<RefCell<Option<Rc<BandEditor>>>> = Rc::new(RefCell::new(None));
        let refresh_editor: Rc<dyn Fn()> = {
            let editor_cell = editor_cell.clone();
            let registry = fader_registry.clone();
            Rc::new(move || {
                let Some(editor) = editor_cell.borrow().clone() else {
                    return;
                };
                let selected = registry
                    .borrow()
                    .iter()
                    .find(|fader| fader.borrow().selected)
                    .cloned();
                match selected {
                    Some(fader) => editor.refresh(Some(&fader.borrow())),
                    None => editor.refresh(None),
                }
            })
        };
        // Owner-authoritative gain request shared by EVERY gain source: the
        // fader drag / scroll / keyboard / zero gestures AND the band editor
        // spin. It returns the gain the band may actually use after peak
        // safety, so callers apply the returned value instead of what they
        // asked for.
        //
        // Why this matters: the preamp floor (-24 dB) is the only headroom
        // lever. Two maxed Hi-Shelves stack their plateaus and measured
        // ~+59 dB raw, so the preamp would need ~-60 dB to stay safe. The
        // cap keeps the raw peak inside what the preamp can always absorb.
        let gain_request: Rc<dyn Fn(usize, f64) -> f64> = {
            let registry = fader_registry.clone();
            let refresh = refresh_editor.clone();
            let smooth = utility.headroom.borrow().smooth.clone();
            let spread_bands = utility.headroom.borrow().smooth_spread_bands.clone();
            Rc::new(move |index, gain_db| {
                let bands = fader_band_snapshot(&registry, smooth.get(), spread_bands.get());
                let clamped = crate::core::clamp_gain_for_peak(
                    &bands,
                    index,
                    gain_db,
                    crate::core::SAMPLE_RATE,
                    crate::core::MAX_SAFE_RAW_PEAK_DB,
                );
                edit_fader(&registry, index, |fader| {
                    fader.gain_db = clamped;
                });

                // Smooth mode: the dragged band's EFFECTIVE delta (post-cap)
                // drags the neighbours through a Gaussian kernel, so the
                // summed response is one broad, smooth curve instead of an
                // isolated hump. The slider's width sets BOTH the bell width
                // and how many bands participate (sigma = width / spacing);
                // they must move together or the bells cannot bridge the
                // participating region and ripple appears on the bump.
                // Neighbours are clamped one at a time against a fresh
                // snapshot so the peak cap always sees the latest state.
                if smooth.get() && index < bands.len() {
                    let delta = clamped - bands[index].gain_db;
                    if delta != 0.0 {
                        // The slider value IS the Gaussian sigma (in bands).
                        let sigma = spread_bands.get();
                        for (other, weighted) in
                            crate::core::smooth_kernel_weights(index, delta, sigma, bands.len())
                        {
                            let cur =
                                fader_band_snapshot(&registry, smooth.get(), spread_bands.get());
                            let Some(cur_band) = cur.get(other) else {
                                continue;
                            };
                            let want = cur_band.gain_db + weighted;
                            let c = crate::core::clamp_gain_for_peak(
                                &cur,
                                other,
                                want,
                                crate::core::SAMPLE_RATE,
                                crate::core::MAX_SAFE_RAW_PEAK_DB,
                            );
                            edit_fader(&registry, other, |fader| {
                                fader.gain_db = c;
                            });
                        }
                    }
                }
                refresh();
                clamped
            })
        };

        let selection_callback = {
            let registry = fader_registry.clone();
            let graph = utility.graph.clone();
            let refresh = refresh_editor.clone();
            Rc::new(move |index: usize| {
                // Toggle: clicking the already-selected fader deselects it so
                // the editor disappears again. Clicking any other fader moves
                // the selection to it.
                let already_selected = registry
                    .borrow()
                    .iter()
                    .any(|f| f.borrow().index == index && f.borrow().selected);
                if already_selected {
                    for fader in registry.borrow().iter() {
                        let mut f = fader.borrow_mut();
                        if f.selected {
                            f.selected = false;
                            f.drawing_area.queue_draw();
                        }
                    }
                    graph.borrow_mut().set_selected_band(None);
                } else {
                    for fader in registry.borrow().iter() {
                        let mut f = fader.borrow_mut();
                        let should_select = f.index == index;
                        if f.selected != should_select {
                            f.selected = should_select;
                            f.drawing_area.queue_draw();
                        }
                    }
                    graph.borrow_mut().set_selected_band(Some(index));
                }
                refresh();
            }) as Rc<dyn Fn(usize)>
        };

        let band_editor = Rc::new(BandEditor::new(BandEditorCallbacks {
            frequency_changed: {
                let registry = fader_registry.clone();
                let refresh = refresh_editor.clone();
                Box::new(move |index, frequency| {
                    edit_fader(&registry, index, |fader| {
                        let clamped = frequency.clamp(
                            crate::core::EQ_FREQUENCY_MIN_HZ,
                            crate::core::EQ_FREQUENCY_MAX_HZ,
                        );
                        fader.frequency = clamped;
                        fader.frequency_label =
                            crate::window_band_fader::format_frequency_label(clamped);
                    });
                    refresh();
                })
            },
            q_changed: {
                let registry = fader_registry.clone();
                let refresh = refresh_editor.clone();
                Box::new(move |index, q| {
                    edit_fader(&registry, index, |fader| {
                        let clamped = q.clamp(crate::core::EQ_Q_MIN, crate::core::EQ_Q_MAX);
                        fader.q_value = clamped;
                        fader.q_label = crate::window_band_fader::format_q_label(clamped);
                    });
                    refresh();
                })
            },
            gain_changed: {
                let gain_request = gain_request.clone();
                Box::new(move |index, gain_db| {
                    // Route through the shared request so the editor spin and
                    // the faders obey the same peak cap. `refresh()` inside
                    // pulls the clamped value back into the spin.
                    gain_request(index, gain_db);
                })
            },
            filter_type_changed: {
                let registry = fader_registry.clone();
                let refresh = refresh_editor.clone();
                Box::new(move |index, filter_type| {
                    edit_fader(&registry, index, |fader| {
                        fader.filter_type = filter_type;
                        fader.filter_type_label =
                            crate::band_fader::filter_type_short_label(filter_type).into();
                        // Upstream `update_band_fader` derives `active` from the
                        // filter type, so selecting `Off` dims the fader.
                        fader.active = filter_type != crate::core::FilterType::Off;
                    });
                    refresh();
                })
            },
            mute_changed: {
                let registry = fader_registry.clone();
                let refresh = refresh_editor.clone();
                Box::new(move |index, muted| {
                    edit_fader(&registry, index, |fader| fader.muted = muted);
                    refresh();
                })
            },
            solo_changed: {
                let registry = fader_registry.clone();
                let refresh = refresh_editor.clone();
                Box::new(move |index, soloed| {
                    edit_fader(&registry, index, |fader| fader.soloed = soloed);
                    recompute_solo_active(&registry.borrow());
                    refresh();
                })
            },
        }));
        *editor_cell.borrow_mut() = Some(band_editor.clone());

        let (split_view, band_scrolled, band_faders) = window_layout::build_main_layout(
            &utility,
            &band_editor,
            crate::core::DEFAULT_ACTIVE_BANDS,
            gain_request.clone(),
            selection_callback,
        );
        *fader_registry.borrow_mut() = band_faders.clone();

        // Smooth override wiring. The switch already drives the shared
        // `smooth` cell (read by the DSP snapshot + gain path); this handler
        // reflects it onto the UI: every fader shows "Sin" and the type/Q
        // controls lock, because the override pins them. The bands' own
        // type/Q are never mutated, so switching off restores them exactly.
        {
            let registry = fader_registry.clone();
            let editor = band_editor.clone();
            let width_spin = utility.headroom.borrow().smooth_width_spin.clone();
            utility
                .headroom
                .borrow()
                .smooth_switch
                .connect_state_set(move |_sw, on| {
                    for fader in registry.borrow().iter() {
                        fader.borrow_mut().smooth_override = on;
                        fader.borrow().drawing_area.queue_draw();
                    }
                    editor.set_type_and_q_enabled(!on);
                    // The width control only means anything while Smooth is on.
                    width_spin.set_sensitive(on);
                    glib::Propagation::Proceed
                });
        }
        refresh_editor();
        let split_view = Rc::new(RefCell::new(split_view));

        // Build toolbar view
        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&header_bar);

        // ToastOverlay inside ToolbarView, Clamp inside ToastOverlay (matches upstream)
        let toast_overlay = adw::ToastOverlay::new();
        let clamp = adw::Clamp::new();
        clamp.set_orientation(gtk4::Orientation::Horizontal);
        clamp.set_maximum_size(1480);
        clamp.set_tightening_threshold(1320);
        clamp.set_hexpand(true);
        clamp.set_vexpand(true);
        clamp.set_child(Some(&*split_view.borrow()));
        toast_overlay.set_child(Some(&clamp));
        toolbar_view.set_content(Some(&toast_overlay));

        // Disable split-view swipe gestures so fader drags don't hide the sidebar
        split_view.borrow_mut().set_enable_show_gesture(false);
        split_view.borrow_mut().set_enable_hide_gesture(false);

        window.set_content(Some(&toolbar_view));

        // Panel switch buttons: mutually exclusive. Each shows its sidebar
        // page and opens the sidebar (as an OVERLAY over the full-width main
        // content, since the split view is collapsed=TRUE). Clicking the
        // already-active button closes the sidebar. The stack handle is cloned
        // because `utility` is moved into Self at the end.
        {
            let stack = utility.container.clone();
            let split_view_for_panel = split_view.clone();
            let guard = Rc::new(std::cell::Cell::new(false));
            let all_buttons: Vec<gtk4::ToggleButton> = vec![
                preset_btn.clone(),
                analyzer_btn.clone(),
                headroom_btn.clone(),
            ];
            let pages = [
                crate::window_utility::PAGE_PRESET,
                crate::window_utility::PAGE_ANALYZER,
                crate::window_utility::PAGE_OUTPUT,
            ];
            for (btn, page) in all_buttons.iter().zip(pages.iter()) {
                let stack = stack.clone();
                let split_view_for_panel = split_view_for_panel.clone();
                let guard = guard.clone();
                let all_buttons = all_buttons.clone();
                let page = *page;
                btn.connect_toggled(move |b| {
                    if guard.get() {
                        return;
                    }
                    guard.set(true);
                    if b.is_active() {
                        for other in all_buttons.iter() {
                            if other != b {
                                other.set_active(false);
                            }
                        }
                        stack.set_visible_child_name(page);
                        split_view_for_panel.borrow_mut().set_show_sidebar(true);
                    } else {
                        // The active button was toggled off -> close the sidebar.
                        split_view_for_panel.borrow_mut().set_show_sidebar(false);
                    }
                    guard.set(false);
                });
            }
            // Default the sidebar to the Preset page, but keep it CLOSED on
            // startup: the user opens it explicitly. (Activating the button
            // here would auto-open the sidebar via the toggled handler.)
            stack.set_visible_child_name(crate::window_utility::PAGE_PRESET);
        }

        // Breakpoints: 1320sp collapses sidebar/pins END, 1080sp compacts toolbar/faders
        let narrow_bp = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            1320.0,
            adw::LengthUnit::Sp,
        ));
        let compact_bp = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
            adw::BreakpointConditionLengthType::MaxWidth,
            1080.0,
            adw::LengthUnit::Sp,
        ));

        let band_scrolled_for_narrow = band_scrolled.clone();
        narrow_bp.connect_apply(move |_| {
            // NOTE: do NOT set_collapsed(true) or swap sidebar position here.
            // The utility panel stays pinned to the End (right) at all sizes;
            // swapping it made the panel jump sides on resize. Keep only the
            // height compaction. Must stay >= the fader height or the fader's
            // bottom (Q label + border) gets clipped. In this width range the
            // faders are 182 (initial) or 208 (after a compact->wide cycle),
            // so use 208 to cover the tallest case.
            band_scrolled_for_narrow.set_min_content_height(208);
        });
        let band_scrolled_for_narrow = band_scrolled.clone();
        narrow_bp.connect_unapply(move |_| {
            band_scrolled_for_narrow.set_min_content_height(208);
        });

        let band_faders_for_compact = band_faders.clone();
        let analyzer_for_compact = utility.analyzer.clone();
        let graph_for_compact = utility.graph.clone();
        let band_scrolled_for_compact = band_scrolled.clone();
        compact_bp.connect_apply(move |_| {
            for fader in band_faders_for_compact.iter() {
                fader.borrow().set_height(164);
            }
            analyzer_for_compact.borrow_mut().set_height(80);
            graph_for_compact
                .borrow_mut()
                .set_mode(crate::window_graph::GraphMode::Compact);
            band_scrolled_for_compact.set_min_content_height(164);
        });
        let band_faders_for_compact = band_faders.clone();
        let analyzer_for_compact = utility.analyzer.clone();
        let graph_for_compact = utility.graph.clone();
        let band_scrolled_for_compact = band_scrolled.clone();
        compact_bp.connect_unapply(move |_| {
            for fader in band_faders_for_compact.iter() {
                fader.borrow().set_height(208);
            }
            analyzer_for_compact.borrow_mut().set_height(120);
            graph_for_compact
                .borrow_mut()
                .set_mode(crate::window_graph::GraphMode::Default);
            band_scrolled_for_compact.set_min_content_height(208);
        });

        window.add_breakpoint(narrow_bp);
        window.add_breakpoint(compact_bp);

        // Bind window state
        window_state::bind_window_state(&window);

        // Center window
        window_utils::center_window();

        // F9 toggles the side panel. When opening, ensure a panel button is
        // active (Preset if none); when closing, deactivate all. The button
        // handlers also drive `show-sidebar`, so we set it directly here too
        // to keep the two paths consistent.
        let split_for_f9 = split_view.clone();
        let f9_buttons = [
            preset_btn.clone(),
            analyzer_btn.clone(),
            headroom_btn.clone(),
        ];
        let key_controller = gtk4::EventControllerKey::new();
        key_controller.connect_key_pressed(move |_, key, _, _| {
            if key == gtk4::gdk::Key::F9 {
                let open = !split_for_f9.borrow().shows_sidebar();
                split_for_f9.borrow_mut().set_show_sidebar(open);
                if open {
                    if !f9_buttons.iter().any(|b| b.is_active()) {
                        f9_buttons[0].set_active(true);
                    }
                } else {
                    for b in f9_buttons.iter() {
                        b.set_active(false);
                    }
                }
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        window.add_controller(key_controller);

        // Start real-time update loop for graph + headroom + backend push
        // Shared flag: true when the EQ curve peak exceeds the -1 dBFS
        // target. Drives the blinking alert on the Set Safe button.
        let headroom_warning = Rc::new(std::cell::Cell::new(false));
        {
            let graph = utility.graph.clone();
            let headroom = utility.headroom.clone();
            let band_faders = band_faders.clone();
            let backend = backend.clone();
            let engine_sink = engine_sink.clone();
            let monitor_loudness_value = utility.monitor_loudness_value.clone();
            let monitor_summary = utility.monitor_summary.clone();
            let headroom_warning = headroom_warning.clone();
            // Debounce state: only reload the filter-chain when the effective
            // state actually changed and at most every 400 ms, so fader drags
            // do not thrash the module.
            let last_pushed_sig = Rc::new(RefCell::new(String::new()));
            let last_push = Rc::new(RefCell::new(
                std::time::Instant::now() - std::time::Duration::from_millis(500),
            ));
            glib::timeout_add_local(std::time::Duration::from_millis(33), move || {
                // Smooth override: the graph, the peak estimate and the
                // backend push must all see the SAME effective bands, or the
                // displayed curve would disagree with the audio.
                let smooth_on = headroom.borrow().smooth.get();
                let spread = headroom.borrow().smooth_spread_bands.get();
                let bands: Vec<crate::core::EqBand> = band_faders
                    .iter()
                    .map(|f| {
                        let fader = f.borrow();
                        crate::core::EqBand {
                            index: fader.index,
                            frequency: fader.frequency,
                            gain_db: fader.gain_db,
                            q: fader.q_value,
                            filter_type: fader.filter_type,
                            mute: fader.muted,
                            solo: fader.soloed,
                            coefficients: crate::core::BiquadCoefficients::identity(),
                        }
                    })
                    .collect();
                let bands = if smooth_on {
                    let spacing = crate::core::band_spacing_oct(&bands);
                    let q = crate::core::smooth_bell_q(spread, spacing);
                    bands
                        .iter()
                        .map(|b| crate::core::smooth_effective_band(b, q))
                        .collect()
                } else {
                    bands
                };
                // Auto-Safe: continuously clamp the preamp so the curve peak
                // stays under the target. Runs before reading `preamp_db` so
                // the graph, the meter and the backend push all see the
                // adjusted value. Sliding the EQ up auto-lowers the preamp;
                // sliding down lets it rise back toward 0.
                // Read the live monitor peak ONCE per tick: the analyzer's
                // take_window_peak() consumes the value, so a second read in
                // the same tick would come back empty.
                let live_peak = backend
                    .borrow()
                    .as_ref()
                    .and_then(|be| be.monitor_peak_dbfs());

                if headroom.borrow().auto_safe_enabled() {
                    let raw_peak = crate::core::estimate_response_peak_db(
                        &bands,
                        0.0,
                        crate::core::SAMPLE_RATE,
                    );
                    // Feed-forward ONLY, deliberately. A live-driven
                    // preamp was tried and rejected: this tick runs at
                    // 33 ms and there is no hard limiter in the PipeWire
                    // filter chain, so program transients (1-10 ms) pass
                    // and clip before any feedback loop can react. The
                    // curve peak is known in advance, which is the only
                    // thing here that can prevent clipping.
                    //
                    // The live peak is still used -- for the numeric label
                    // and the LED -- just never to drive the preamp.
                    let desired = crate::window_headroom::auto_safe_preamp_db(
                        raw_peak,
                        crate::window_headroom::AUTO_SAFE_TARGET_DBFS,
                    );
                    if (desired - headroom.borrow().preamp_value()).abs() > 0.01 {
                        headroom.borrow_mut().set_preamp_value(desired);
                    }
                }
                let preamp_db = headroom.borrow().preamp_value();
                {
                    // Overlay reflects the *selected* band's actual controls,
                    // not a hardcoded Bell. Previously the type was pinned to
                    // Bell so switching filter types never changed the curve.
                    let selected = band_faders.iter().find(|f| f.borrow().selected);
                    let (sel_freq, sel_q, sel_type) = match selected {
                        Some(f) => {
                            let f = f.borrow();
                            (f.frequency, f.q_value, f.filter_type)
                        }
                        None => (1000.0, 1.0, crate::core::FilterType::Bell),
                    };
                    let mut g = graph.borrow_mut();
                    // Feed live spectrum from the output monitor (empty when
                    // the monitor is off, so the overlay draws nothing).
                    let analyzer_levels: Vec<f64> = backend
                        .borrow()
                        .as_ref()
                        .map(|be| be.monitor_levels())
                        .unwrap_or_default();
                    g.update(
                        preamp_db,
                        sel_freq,
                        sel_q,
                        sel_type,
                        &bands,
                        &analyzer_levels,
                    );
                }
                // Live loudness readout in the monitor strip (short-term LUFS).
                if let Some(be) = backend.borrow().as_ref() {
                    if be.monitor_enabled() {
                        if let Some(loud) = be.monitor_loudness() {
                            let lufs = loud.shortterm_lufs;
                            if lufs.is_finite() {
                                monitor_loudness_value.set_text(&format!("{lufs:.1} LUFS"));
                                monitor_summary.set_text(&format!("On \u{00b7} {lufs:.1} LUFS"));
                            }
                        }
                    }
                }
                headroom.borrow_mut().update_curve_peak(&bands, preamp_db);

                // Warn (blink the Headroom icon) when the output rides over
                // the -1 dBFS target. Prefer the LIVE monitor peak (actual
                // output level). When the monitor is off, fall back to the
                // estimated curve peak but only warn on an actual boost
                // (peak > 0 dB), so a flat/neutral EQ never false-triggers.
                // The curve peak above is a property of the EQ settings,
                // not a measurement, so it must not be presented as a
                // level. Show the real output peak whenever the monitor is
                // delivering one; otherwise label the estimate explicitly.
                headroom.borrow_mut().apply_live_peak(live_peak);
                let warn = match live_peak {
                    Some(db) => db > crate::window_headroom::AUTO_SAFE_TARGET_DBFS,
                    None => {
                        let est = *headroom.borrow().peak_value.borrow();
                        est > 0.0
                    }
                };
                headroom_warning.set(warn);

                // Push UI state to the PipeWire engine (debounced).
                let sig = crate::core::preset_payload_state_signature(
                    &crate::core::preset_payload(&bands, preamp_db),
                );
                if sig != *last_pushed_sig.borrow()
                    && last_push.borrow().elapsed() >= std::time::Duration::from_millis(400)
                {
                    // Startup grace: the live node proxy is captured
                    // asynchronously after the module load. Pushing before it
                    // exists used to fall through to a full module
                    // unload+reload, cutting the audio a SECOND time just
                    // after startup. Leave the signature unpushed so the next
                    // tick retries once the proxy has arrived.
                    let live_ready = backend
                        .borrow()
                        .as_ref()
                        .map(|be| be.has_live_node())
                        .unwrap_or(false);
                    if live_ready {
                        if let Some(be) = backend.borrow_mut().as_mut() {
                            if !engine_sink.is_empty() {
                                let _ = be.set_preamp(preamp_db);
                                *be.get_bands_mut() = bands.clone();
                                match be.update_state_live_or_reload(&engine_sink) {
                                    Ok(()) => {
                                        log::debug!("Backend state applied");
                                    }
                                    Err(e) => log::warn!("Failed to apply backend state: {}", e),
                                }
                            }
                            // Mark pushed either way so a failing state is not
                            // retried every tick; further edits change the sig.
                            *last_pushed_sig.borrow_mut() = sig;
                            *last_push.borrow_mut() = std::time::Instant::now();
                        }
                    }
                }
                ControlFlow::Continue
            });
        }

        // Blink the Headroom header icon while the EQ curve peak is over the
        // -1 dBFS target. GTK4 CSS has no @keyframes, so we toggle a class
        // on a 500 ms timer; a CSS color transition smooths it into a pulse.
        // Blink the clipping alert on the **Set Safe** button while the EQ
        // curve peak is over the -1 dBFS target. The header icon no longer
        // flashes: the alert belongs on the control that fixes it. GTK4 CSS
        // has no @keyframes, so we toggle a class on a 500 ms timer; a CSS
        // color transition smooths it into a pulse.
        {
            let set_safe = utility.headroom.borrow().set_safe_button.clone();
            let headroom_warning = headroom_warning.clone();
            let blink_on = Rc::new(std::cell::Cell::new(false));
            glib::timeout_add_local(std::time::Duration::from_millis(500), move || {
                if headroom_warning.get() {
                    blink_on.set(!blink_on.get());
                    if blink_on.get() {
                        set_safe.add_css_class("headroom-warning");
                    } else {
                        set_safe.remove_css_class("headroom-warning");
                    }
                } else if blink_on.get() || set_safe.has_css_class("headroom-warning") {
                    blink_on.set(false);
                    set_safe.remove_css_class("headroom-warning");
                }
                ControlFlow::Continue
            });
        }

        // events, sync roundtrips and module callbacks are dispatched without
        // running a second OS thread.
        {
            let backend = backend.clone();
            glib::timeout_add_local(std::time::Duration::from_millis(10), move || {
                if let Some(be) = backend.borrow_mut().as_mut() {
                    be.pump();
                    // Advance the pending monitor port-linking (non-blocking,
                    // bounded work per tick).
                    be.pump_monitor_link();
                }
                ControlFlow::Continue
            });
        }

        // System-wide EQ switch: route all app playback streams into the
        // virtual EQ sink (on) / log that unrouting is not yet implemented
        // (off).
        {
            let backend_for_switch = backend.clone();
            route_switch.connect_state_set(move |_switch, on| {
                if let Some(be) = backend_for_switch.borrow_mut().as_mut() {
                    if on {
                        if let Err(e) = be.auto_route_to_sink(crate::core::VIRTUAL_SINK_BASE) {
                            log::warn!("System EQ: auto-route failed: {}", e);
                        }
                    } else {
                        if let Err(e) = be.unroute_all() {
                            log::warn!("System EQ off: unroute failed: {}", e);
                        }
                    }
                }
                glib::Propagation::Proceed
            });
        }

        // Monitor switch: start/stop the output spectrum capture. Mirrors
        // upstream `set_analyzer_enabled` -> `ensure_output_analyzer`, where
        // the analyzer captures the controller's output sink (the sink the
        // EQ outputs to, i.e. what you hear) via a monitor tap. Our capture
        // is a separate stream, so (unlike upstream) enabling it does not
        // require restarting the filter-chain engine.
        {
            let backend_for_monitor = backend.clone();
            let monitor_target = engine_sink.clone();
            let summary = utility.monitor_summary.clone();
            utility
                .graph
                .borrow()
                .monitor_switch
                .connect_state_set(move |_switch, on| {
                    if let Some(be) = backend_for_monitor.borrow_mut().as_mut() {
                        if on {
                            let target = if monitor_target.is_empty() {
                                be.default_output_sink().unwrap_or_default()
                            } else {
                                monitor_target.clone()
                            };
                            if target.is_empty() {
                                log::warn!("Monitor: no output sink to capture");
                                summary.set_text("Off · no sink");
                                return glib::Propagation::Stop;
                            }
                            match be.start_monitor(&target) {
                                Ok(()) => {
                                    log::info!("Monitor enabled on {target}");
                                    summary.set_text("On · Live");
                                }
                                Err(e) => {
                                    log::warn!("Monitor start failed: {e}");
                                    summary.set_text("Off");
                                    return glib::Propagation::Stop;
                                }
                            }
                        } else {
                            be.stop_monitor();
                            summary.set_text("Off");
                        }
                    }
                    glib::Propagation::Proceed
                });
        }

        // Output Controls (Headroom panel): per-output-device auto-preset.
        // Fallback = default preset for unmatched outputs; Link to Output =
        // auto-load the current preset for the active output device. Both
        // act on the currently-selected preset (from the Preset panel).
        {
            let presets = utility.presets.clone();
            let fallback_label = utility.fallback_label.clone();
            utility.fallback_button.connect_clicked(move |_| {
                match presets.borrow().current_preset_name() {
                    Some(name) => {
                        if let Err(e) = crate::core::set_output_preset_fallback_name(&name) {
                            log::warn!("Set fallback preset failed: {e}");
                        } else {
                            fallback_label.set_text(&name);
                            log::info!("Fallback preset set to {name}");
                        }
                    }
                    None => log::info!("No preset selected to set as fallback"),
                }
            });

            let presets = utility.presets.clone();
            let link_label = utility.link_label.clone();
            utility.link_button.connect_clicked(move |_| {
                match presets.borrow().current_preset_name() {
                    Some(name) => {
                        if let Err(e) = crate::core::set_output_preset_link("default", &name) {
                            log::warn!("Link preset to output failed: {e}");
                        } else {
                            link_label.set_text(&name);
                            log::info!("Linked preset {name} to output");
                        }
                    }
                    None => log::info!("No preset selected to link to output"),
                }
            });
        }

        {
            let headroom = utility.headroom.clone();
            let band_faders = band_faders.clone();
            utility
                .headroom
                .borrow()
                .set_safe_button
                .connect_clicked(move |_| {
                    let bands: Vec<crate::core::EqBand> = band_faders
                        .iter()
                        .map(|f| {
                            let fader = f.borrow();
                            crate::core::EqBand {
                                index: fader.index,
                                frequency: fader.frequency,
                                gain_db: fader.gain_db,
                                q: fader.q_value,
                                filter_type: fader.filter_type,
                                mute: fader.muted,
                                solo: fader.soloed,
                                coefficients: crate::core::BiquadCoefficients::identity(),
                            }
                        })
                        .collect();
                    let panel = headroom.borrow();
                    let peak = crate::core::estimate_response_peak_db(
                        &bands,
                        panel.preamp_value(),
                        crate::core::SAMPLE_RATE,
                    );
                    if peak <= 0.5 {
                        return;
                    }
                    panel.set_preamp_value(panel.preamp_value() - peak - 1.0);
                });
        }

        // Setup preset panel callbacks
        {
            let presets = utility.presets.clone();
            let band_faders = band_faders.clone();
            let default_sig = crate::core::preset_payload_state_signature(
                &crate::core::preset_payload(&crate::core::default_bands(), 0.0),
            );
            let apply_band_faders = band_faders.clone();
            let apply_headroom = utility.headroom.clone();
            let apply_refresh = refresh_editor.clone();
            let reset_band_faders = band_faders.clone();
            let reset_headroom = utility.headroom.clone();
            let reset_refresh = refresh_editor.clone();
            let sig_band_faders = band_faders.clone();
            let sig_headroom = utility.headroom.clone();
            presets.borrow_mut().set_callbacks(
                Some(Box::new(move |bands, preamp| {
                    // Upstream re-syncs every fader from the loaded bands
                    // (`update_band_fader`), so mute/solo come from the preset
                    // rather than from the pre-load UI state.
                    let solo_active = crate::core::bands_have_solo(&bands);
                    apply_headroom.borrow().set_preamp_value(preamp);
                    for (i, band) in bands.iter().enumerate() {
                        if let Some(fader) = apply_band_faders.get(i) {
                            let frequency = band.frequency.clamp(
                                crate::core::EQ_FREQUENCY_MIN_HZ,
                                crate::core::EQ_FREQUENCY_MAX_HZ,
                            );
                            let q = band.q.clamp(crate::core::EQ_Q_MIN, crate::core::EQ_Q_MAX);
                            let mut f = fader.borrow_mut();
                            let selected = f.selected;
                            f.set_band_state(
                                band.gain_db,
                                frequency,
                                crate::window_band_fader::format_frequency_label(frequency),
                                q,
                                crate::window_band_fader::format_q_label(q),
                                band.filter_type,
                                crate::band_fader::filter_type_short_label(band.filter_type).into(),
                                selected,
                                band.filter_type != crate::core::FilterType::Off,
                                band.mute,
                                band.solo,
                                solo_active,
                            );
                            f.drawing_area.queue_draw();
                        }
                    }
                    recompute_solo_active(&apply_band_faders);
                    apply_refresh();
                })),
                Some(Box::new(move || {
                    // Upstream `reset_state` restores `default_bands()` (the
                    // first DEFAULT_ACTIVE_BANDS as neutral *Bell*s), NOT all
                    // Off. Setting Off left the bands inert so moving a fader
                    // produced no curve change ("EQ stays off").
                    let default_bands = crate::core::default_bands();
                    // Upstream `reset_state` also zeroes the preamp.
                    reset_headroom.borrow().set_preamp_value(0.0);
                    for (i, fader) in reset_band_faders.iter().enumerate() {
                        let mut f = fader.borrow_mut();
                        let band =
                            default_bands
                                .get(i)
                                .cloned()
                                .unwrap_or_else(|| crate::core::EqBand {
                                    index: i,
                                    frequency: 1000.0,
                                    gain_db: 0.0,
                                    q: 1.0,
                                    filter_type: crate::core::FilterType::Off,
                                    mute: false,
                                    solo: false,
                                    coefficients: crate::core::BiquadCoefficients::identity(),
                                });
                        let frequency = band.frequency;
                        let q = band.q;
                        let filter_type = band.filter_type;
                        let selected = f.selected;
                        f.set_band_state(
                            0.0,
                            frequency,
                            crate::window_band_fader::format_frequency_label(frequency),
                            q,
                            crate::window_band_fader::format_q_label(q),
                            filter_type,
                            crate::band_fader::filter_type_short_label(filter_type).into(),
                            selected,
                            i < crate::core::DEFAULT_ACTIVE_BANDS,
                            false,
                            false,
                            false,
                        );
                        f.drawing_area.queue_draw();
                    }
                    reset_refresh();
                })),
                Some(Box::new(move || {
                    let bands: Vec<crate::core::EqBand> = sig_band_faders
                        .iter()
                        .map(|f| {
                            let fader = f.borrow();
                            crate::core::EqBand {
                                index: fader.index,
                                frequency: fader.frequency,
                                gain_db: fader.gain_db,
                                q: fader.q_value,
                                filter_type: fader.filter_type,
                                mute: fader.muted,
                                solo: fader.soloed,
                                coefficients: crate::core::BiquadCoefficients::identity(),
                            }
                        })
                        .collect();
                    crate::core::preset_payload_state_signature(&crate::core::preset_payload(
                        &bands,
                        sig_headroom.borrow().preamp_value(),
                    ))
                })),
            );
            presets.borrow_mut().set_default_signature(default_sig);
        }

        Self {
            window,
            toolbar_view,
            utility,
            split_view,
            band_scrolled,
            band_faders,
        }
    }

    pub fn present(&self) {
        self.window.present();
    }
}

fn create_menu_model() -> gio::Menu {
    let menu = gio::Menu::new();

    let file_section = gio::Menu::new();
    file_section.append(Some("Preferences"), Some("app.preferences"));
    file_section.append(Some("Quit"), Some("app.quit"));
    menu.append_section(None, &file_section);

    let help_section = gio::Menu::new();
    help_section.append(Some("About Mini EQ"), Some("app.about"));
    menu.append_section(None, &help_section);

    menu
}
