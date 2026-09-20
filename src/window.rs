//! Main application window for mini-eq.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;

use glib::ControlFlow;

use crate::appearance::{AppearancePreference, apply_appearance_preference};
use crate::style;
use crate::window_layout;
use crate::window_state;
use crate::window_utility::UtilityPane;
use crate::window_utils;

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
    pub fn new(app: &adw::Application) -> Self {
        let window = adw::ApplicationWindow::new(app);
        let (default_width, default_height) = window_state::initial_window_default_size();
        window.set_default_size(default_width, default_height);
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
        output_dropdown.set_size_request(300, -1);
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
        inspector_button.set_icon_name("dialog-information-symbolic");
        inspector_button.set_tooltip_text(Some("Inspector Pane"));
        header_bar.pack_end(&inspector_button);

        // Build main layout with utility pane
        let (split_view, band_scrolled, band_faders) =
            window_layout::build_main_layout(&utility, crate::core::DEFAULT_ACTIVE_BANDS);
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

        let split_view_for_bp = split_view.clone();
        let band_scrolled_for_narrow = band_scrolled.clone();
        narrow_bp.connect_apply(move |_| {
            split_view_for_bp.borrow_mut().set_collapsed(true);
            split_view_for_bp
                .borrow_mut()
                .set_sidebar_position(gtk4::PackType::End);
            band_scrolled_for_narrow.set_min_content_height(150);
        });
        let split_view_for_bp = split_view.clone();
        let band_scrolled_for_narrow = band_scrolled.clone();
        narrow_bp.connect_unapply(move |_| {
            split_view_for_bp.borrow_mut().set_collapsed(false);
            split_view_for_bp
                .borrow_mut()
                .set_sidebar_position(gtk4::PackType::Start);
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

        // F9 binding to toggle sidebar
        let split_view_clone = split_view.clone();
        let key_controller = gtk4::EventControllerKey::new();
        key_controller.connect_key_pressed(move |_, key, _, _| {
            if key == gtk4::gdk::Key::F9 {
                split_view_clone
                    .borrow_mut()
                    .set_collapsed(!split_view_clone.borrow().is_collapsed());
                return glib::Propagation::Stop;
            }
            glib::Propagation::Proceed
        });
        window.add_controller(key_controller);

        // Start real-time update loop for graph
        {
            let graph = utility.graph.clone();
            let band_faders = band_faders.clone();
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
                            enabled: fader.active,
                            solo: fader.soloed,
                            coefficients: crate::core::BiquadCoefficients::identity(),
                        }
                    })
                    .collect();
                let mut g = graph.borrow_mut();
                g.update(0.0, 1000.0, 1.0, crate::core::FilterType::Bell, &bands, &[]);
                ControlFlow::Continue
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
            let reset_band_faders = band_faders.clone();
            let sig_band_faders = band_faders.clone();
            presets.borrow_mut().set_callbacks(
                Some(Box::new(move |bands, _preamp| {
                    for (i, band) in bands.iter().enumerate() {
                        if let Some(fader) = apply_band_faders.get(i) {
                            let mut f = fader.borrow_mut();
                            f.gain_db = band
                                .gain_db
                                .clamp(crate::core::EQ_GAIN_MIN_DB, crate::core::EQ_GAIN_MAX_DB);
                            f.frequency = band.frequency.clamp(
                                crate::core::EQ_FREQUENCY_MIN_HZ,
                                crate::core::EQ_FREQUENCY_MAX_HZ,
                            );
                            f.q_value = band.q.clamp(crate::core::EQ_Q_MIN, crate::core::EQ_Q_MAX);
                            f.filter_type = band.filter_type;
                            f.active = band.enabled;
                            f.drawing_area.queue_draw();
                        }
                    }
                })),
                Some(Box::new(move || {
                    for (i, fader) in reset_band_faders.iter().enumerate() {
                        let mut f = fader.borrow_mut();
                        f.gain_db = 0.0;
                        f.filter_type = crate::core::FilterType::Off;
                        f.active = i < crate::core::DEFAULT_ACTIVE_BANDS;
                        f.drawing_area.queue_draw();
                    }
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
                                enabled: fader.active,
                                solo: fader.soloed,
                                coefficients: crate::core::BiquadCoefficients::identity(),
                            }
                        })
                        .collect();
                    crate::core::preset_payload_state_signature(&crate::core::preset_payload(
                        &bands, 0.0,
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
