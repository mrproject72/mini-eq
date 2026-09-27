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

        // Build header bar
        let header_bar = adw::HeaderBar::new();
        let window_title = adw::WindowTitle::new("Mini EQ", "");
        header_bar.set_title_widget(Some(&window_title));

        // EQ output dropdown
        let output_label = gtk4::Label::new(Some("Output"));
        output_label.set_css_classes(&["heading"]);
        header_bar.pack_start(&output_label);

        let output_list = gtk4::StringList::new(&["System Output", "Virtual Sink"]);
        let output_dropdown = gtk4::DropDown::new(Some(output_list), None::<gtk4::Expression>);
        // Keep the header narrow so the window can shrink to the 640px
        // minimum. The dropdown ellipsizes long device names.
        output_dropdown.set_size_request(160, -1);
        output_dropdown.set_hexpand(true);
        header_bar.pack_start(&output_dropdown);

        // Main menu button
        let menu_button = gtk4::MenuButton::new();
        menu_button.set_icon_name("open-menu-symbolic");
        let menu_model = create_menu_model();
        menu_button.set_menu_model(Some(&menu_model));
        header_bar.pack_end(&menu_button);

        // System-wide EQ toggle
        let route_switch = gtk4::Switch::new();
        route_switch.set_tooltip_text(Some("System-wide EQ"));
        header_bar.pack_end(&route_switch);

        let route_label = gtk4::Label::new(Some("System EQ"));
        header_bar.pack_end(&route_label);

        // Inspector pane toggle
        let inspector_button = gtk4::ToggleButton::new();
        inspector_button.set_icon_name("sidebar-show-symbolic");
        inspector_button.set_tooltip_text(Some("Toggle side panel (F9)"));
        // Panel is collapsed (hidden) by default; the toggle reveals it as an
        // overlay over the full-width main content.
        inspector_button.set_active(false);
        header_bar.pack_end(&inspector_button);

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
                let registry = fader_registry.clone();
                let refresh = refresh_editor.clone();
                Box::new(move |index, gain_db| {
                    edit_fader(&registry, index, |fader| {
                        fader.gain_db =
                            gain_db.clamp(crate::core::EQ_GAIN_MIN_DB, crate::core::EQ_GAIN_MAX_DB);
                    });
                    refresh();
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
            selection_callback,
        );
        *fader_registry.borrow_mut() = band_faders.clone();
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

        // Inspector pane toggle mirrors the F9 binding. The split view is kept
        // in overlay mode (collapsed=TRUE) so the main content is ALWAYS full
        // width; the toggle drives `show-sidebar`, which slides the panel
        // OVER the content instead of resizing it.
        {
            let split_view_for_toggle = split_view.clone();
            inspector_button.connect_toggled(move |button| {
                split_view_for_toggle
                    .borrow_mut()
                    .set_show_sidebar(button.is_active());
            });
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
            // height compaction.
            band_scrolled_for_narrow.set_min_content_height(150);
        });
        let band_scrolled_for_narrow = band_scrolled.clone();
        narrow_bp.connect_unapply(move |_| {
            band_scrolled_for_narrow.set_min_content_height(200);
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
            band_scrolled_for_compact.set_min_content_height(150);
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
            band_scrolled_for_compact.set_min_content_height(200);
        });

        window.add_breakpoint(narrow_bp);
        window.add_breakpoint(compact_bp);

        // Bind window state
        window_state::bind_window_state(&window);

        // Center window
        window_utils::center_window();

        // F9 binding to toggle the side panel. Drives the inspector button's
        // active state, which in turn sets `show-sidebar` (overlay reveal).
        let inspector_for_f9 = inspector_button.clone();
        let key_controller = gtk4::EventControllerKey::new();
        key_controller.connect_key_pressed(move |_, key, _, _| {
            if key == gtk4::gdk::Key::F9 {
                inspector_for_f9.set_active(!inspector_for_f9.is_active());
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        window.add_controller(key_controller);

        // Start real-time update loop for graph + headroom + backend push
        {
            let graph = utility.graph.clone();
            let headroom = utility.headroom.clone();
            let band_faders = band_faders.clone();
            let backend = backend.clone();
            let engine_sink = engine_sink.clone();
            // Debounce state: only reload the filter-chain when the effective
            // state actually changed and at most every 400 ms, so fader drags
            // do not thrash the module.
            let last_pushed_sig = Rc::new(RefCell::new(String::new()));
            let last_push = Rc::new(RefCell::new(
                std::time::Instant::now() - std::time::Duration::from_millis(500),
            ));
            glib::timeout_add_local(std::time::Duration::from_millis(33), move || {
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
                // Auto-Safe: continuously clamp the preamp so the curve peak
                // stays under the target. Runs before reading `preamp_db` so
                // the graph, the meter and the backend push all see the
                // adjusted value. Sliding the EQ up auto-lowers the preamp;
                // sliding down lets it rise back toward 0.
                if headroom.borrow().auto_safe_enabled() {
                    let raw_peak = crate::core::estimate_response_peak_db(
                        &bands,
                        0.0,
                        crate::core::SAMPLE_RATE,
                    );
                    let desired = crate::window_headroom::auto_safe_preamp_db(
                        raw_peak,
                        crate::window_headroom::AUTO_SAFE_TARGET_DBFS,
                    );
                    if (desired - headroom.borrow().preamp_value()).abs() > 0.05 {
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
                    g.update(preamp_db, sel_freq, sel_q, sel_type, &bands, &[]);
                }
                headroom.borrow_mut().update_curve_peak(&bands, preamp_db);

                // Push UI state to the PipeWire engine (debounced).
                let sig = crate::core::preset_payload_state_signature(
                    &crate::core::preset_payload(&bands, preamp_db),
                );
                if sig != *last_pushed_sig.borrow()
                    && last_push.borrow().elapsed() >= std::time::Duration::from_millis(400)
                {
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
                ControlFlow::Continue
            });
        }

        // Pump the PipeWire main loop from the GTK main loop so registry
        // events, sync roundtrips and module callbacks are dispatched without
        // running a second OS thread.
        {
            let backend = backend.clone();
            glib::timeout_add_local(std::time::Duration::from_millis(10), move || {
                if let Some(be) = backend.borrow().as_ref() {
                    be.pump();
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

        // "Set Safe" lowers the preamp so the estimated curve peak clears 0 dBFS
        // with 1 dB of margin, mirroring upstream `on_set_safe_preamp_clicked`.
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
