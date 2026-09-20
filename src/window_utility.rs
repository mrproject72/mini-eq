//! Utility pane (right sidebar sections).

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;

use crate::window_analyzer;
use crate::window_graph;
use crate::window_headroom;
use crate::window_presets;

/// Utility pane widget containing preset section and system section.
pub struct UtilityPane {
    pub container: gtk4::ScrolledWindow,
    pub preset_section: gtk4::Box,
    pub system_section: gtk4::Box,
    pub analyzer: Rc<RefCell<window_analyzer::AnalyzerPanel>>,
    pub headroom: Rc<RefCell<window_headroom::HeadroomPanel>>,
    pub graph: Rc<RefCell<window_graph::EqGraph>>,
    pub presets: Rc<RefCell<window_presets::PresetPanel>>,
}

impl UtilityPane {
    pub fn new() -> Self {
        let analyzer = Rc::new(RefCell::new(window_analyzer::AnalyzerPanel::new()));
        let headroom = Rc::new(RefCell::new(window_headroom::HeadroomPanel::new()));
        let graph = Rc::new(RefCell::new(window_graph::EqGraph::new()));
        let presets = window_presets::PresetPanel::new();
        presets.borrow_mut().start_file_monitoring();

        let container = gtk4::ScrolledWindow::new();
        container.set_policy(gtk4::PolicyType::Automatic, gtk4::PolicyType::Automatic);
        container.set_vexpand(true);
        container.set_size_request(310, -1);

        let main_box = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        main_box.set_margin_top(8);
        main_box.set_margin_bottom(8);
        main_box.set_margin_start(8);
        main_box.set_margin_end(8);

        let preset_section = Self::build_preset_section(&presets);
        main_box.append(&preset_section);

        let system_section = Self::build_system_section(&analyzer, &headroom);
        main_box.append(&system_section);

        container.set_child(Some(&main_box));

        Self {
            container,
            preset_section,
            system_section,
            analyzer,
            headroom,
            graph,
            presets,
        }
    }

    fn build_preset_section(presets: &Rc<RefCell<window_presets::PresetPanel>>) -> gtk4::Box {
        let section = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
        section.set_css_classes(&["utility-section"]);

        section.append(presets.borrow().widget());

        section
    }

    fn build_system_section(
        analyzer: &Rc<RefCell<window_analyzer::AnalyzerPanel>>,
        headroom: &Rc<RefCell<window_headroom::HeadroomPanel>>,
    ) -> gtk4::Box {
        let section = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
        section.set_css_classes(&["utility-section"]);

        let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        let title = gtk4::Label::new(Some("Signal"));
        title.set_css_classes(&["heading"]);
        header.append(&title);

        let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        header.append(&spacer);

        let state_chip = gtk4::Label::new(Some("Active"));
        state_chip.set_css_classes(&["system-state-chip"]);
        state_chip.set_width_chars(11);
        header.append(&state_chip);
        section.append(&header);

        let compare_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        compare_row.set_css_classes(&["compare-row"]);
        let compare_label = gtk4::Label::new(Some("A/B"));
        compare_label.set_css_classes(&["metric-title"]);
        compare_row.append(&compare_label);

        let compare_spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        compare_spacer.set_hexpand(true);
        compare_row.append(&compare_spacer);

        let bypass_switch = gtk4::Switch::new();
        bypass_switch.set_valign(gtk4::Align::Center);
        bypass_switch.set_tooltip_text(Some("A/B Compare"));
        compare_row.append(&bypass_switch);
        section.append(&compare_row);

        section.append(analyzer.borrow().widget());

        let headroom_panel = headroom.borrow();
        section.append(headroom_panel.widget());

        let monitor_panel = Self::build_monitor_panel();
        section.append(&monitor_panel);

        section
    }

    fn build_monitor_panel() -> gtk4::Box {
        let panel = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
        panel.set_css_classes(&["monitor-strip"]);

        let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        let title = gtk4::Label::new(Some("Monitor"));
        title.set_css_classes(&["metric-title"]);
        header.append(&title);

        let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        header.append(&spacer);

        let settings_button = gtk4::MenuButton::new();
        settings_button.set_icon_name("preferences-system-symbolic");
        settings_button.set_tooltip_text(Some("Monitor Settings"));
        settings_button.set_valign(gtk4::Align::Center);
        settings_button.set_css_classes(&["toolbar-icon-button", "monitor-settings-button"]);

        let settings_popover = gtk4::Popover::new();
        let settings_box = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        settings_box.set_margin_top(8);
        settings_box.set_margin_bottom(8);
        settings_box.set_margin_start(8);
        settings_box.set_margin_end(8);

        let smoothing_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        let smoothing_label = gtk4::Label::new(Some("Smoothing"));
        smoothing_row.append(&smoothing_label);
        let smoothing_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 15.0, 95.0, 1.0);
        smoothing_scale.set_size_request(116, -1);
        smoothing_scale.set_hexpand(true);
        smoothing_row.append(&smoothing_scale);
        settings_box.append(&smoothing_row);

        let display_gain_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        let display_gain_label = gtk4::Label::new(Some("Display Gain"));
        display_gain_row.append(&display_gain_label);
        let display_gain_scale = gtk4::Scale::with_range(gtk4::Orientation::Horizontal, -12.0, 32.0, 1.0);
        display_gain_scale.set_size_request(116, -1);
        display_gain_scale.set_hexpand(true);
        display_gain_row.append(&display_gain_scale);
        settings_box.append(&display_gain_row);

        let freeze_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        let freeze_label = gtk4::Label::new(Some("Freeze"));
        freeze_row.append(&freeze_label);
        let freeze_switch = gtk4::Switch::new();
        freeze_switch.set_valign(gtk4::Align::Center);
        freeze_row.append(&freeze_switch);
        settings_box.append(&freeze_row);

        settings_popover.set_child(Some(&settings_box));
        settings_button.set_popover(Some(&settings_popover));
        header.append(&settings_button);

        let monitor_switch = gtk4::Switch::new();
        monitor_switch.set_valign(gtk4::Align::Center);
        header.append(&monitor_switch);
        panel.append(&header);

        let detail_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        detail_row.set_css_classes(&["monitor-detail-row"]);

        let loudness_meter = gtk4::DrawingArea::new();
        loudness_meter.set_size_request(104, 16);
        loudness_meter.set_hexpand(true);
        loudness_meter.set_valign(gtk4::Align::Center);
        detail_row.append(&loudness_meter);

        let loudness_value = gtk4::Label::new(Some("-23 LUFS"));
        loudness_value.set_css_classes(&["numeric", "loudness-value-label"]);
        loudness_value.set_width_chars(8);
        detail_row.append(&loudness_value);
        panel.append(&detail_row);

        let summary_label = gtk4::Label::new(Some("On · -23 LUFS"));
        summary_label.set_css_classes(&["monitor-summary-label"]);
        summary_label.set_halign(gtk4::Align::Start);
        panel.append(&summary_label);

        panel
    }

    pub fn widget(&self) -> &gtk4::ScrolledWindow {
        &self.container
    }
}

impl Default for UtilityPane {
    fn default() -> Self {
        Self::new()
    }
}