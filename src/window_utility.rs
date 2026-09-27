//! Utility sidebar: three independently-selectable panels (Preset, Signal
//! Analyzer, Headroom) held in a `gtk4::Stack`. The header's three buttons
//! switch `container`'s visible child, so only one panel shows at a time
//! and the user never has to scroll a single mega-column.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;

use crate::window_analyzer;
use crate::window_graph;
use crate::window_headroom;
use crate::window_presets;

/// Stack page names (used by the header buttons to switch panels).
pub const PAGE_PRESET: &str = "preset";
pub const PAGE_ANALYZER: &str = "analyzer";
pub const PAGE_HEADROOM: &str = "headroom";

/// Utility sidebar holding the three panels in a `Stack`.
pub struct UtilityPane {
    /// The sidebar: a `Stack` whose visible child is the selected panel.
    pub container: gtk4::Stack,
    pub analyzer: Rc<RefCell<window_analyzer::AnalyzerPanel>>,
    pub headroom: Rc<RefCell<window_headroom::HeadroomPanel>>,
    pub graph: Rc<RefCell<window_graph::EqGraph>>,
    pub presets: Rc<RefCell<window_presets::PresetPanel>>,
    /// The Monitor on/off switch (wired to backend start/stop_monitor).
    pub monitor_switch: gtk4::Switch,
    /// Loudness readout label in the monitor strip (updated from the meter).
    pub monitor_loudness_value: gtk4::Label,
    /// "On · -23 LUFS" summary label in the monitor strip.
    pub monitor_summary: gtk4::Label,
    /// A/B compare (EQ bypass) switch.
    pub bypass_switch: gtk4::Switch,
    /// Output device dropdown (moved from the header into Output Controls).
    pub output_dropdown: gtk4::DropDown,
    /// Fallback preset action (default preset for unmatched outputs).
    pub fallback_button: gtk4::Button,
    pub fallback_label: gtk4::Label,
    /// Link-to-output action (auto-load current preset for the active output).
    pub link_button: gtk4::Button,
    pub link_label: gtk4::Label,
}

impl UtilityPane {
    pub fn new() -> Self {
        let analyzer = Rc::new(RefCell::new(window_analyzer::AnalyzerPanel::new()));
        let headroom = Rc::new(RefCell::new(window_headroom::HeadroomPanel::new()));
        let graph = Rc::new(RefCell::new(window_graph::EqGraph::new()));
        let presets = window_presets::PresetPanel::new();
        presets.borrow_mut().start_file_monitoring();

        // --- Preset page: the preset panel (has its own scrollable list).
        let preset_page = Self::scroll_page(presets.borrow().widget());

        // --- Signal Analyzer page: spectrum + monitor strip.
        let (monitor_panel, monitor_switch, monitor_loudness_value, monitor_summary) =
            Self::build_monitor_panel();
        let analyzer_box = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        analyzer_box.set_margin_top(8);
        analyzer_box.set_margin_bottom(8);
        analyzer_box.set_margin_start(8);
        analyzer_box.set_margin_end(8);
        analyzer_box.append(analyzer.borrow().widget());
        analyzer_box.append(&monitor_panel);
        let analyzer_page = Self::scroll_page(&analyzer_box);

        // --- Headroom page: preamp + peak meter + auto-safe + A/B compare.
        let headroom_box = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
        headroom_box.set_margin_top(8);
        headroom_box.set_margin_bottom(8);
        headroom_box.set_margin_start(8);
        headroom_box.set_margin_end(8);
        let bypass_switch = Self::build_compare_row();
        headroom_box.append(&bypass_switch.0);
        headroom_box.append(headroom.borrow().widget());

        // Output Controls: output device + per-device auto-preset actions.
        let (
            output_controls,
            output_dropdown,
            fallback_button,
            fallback_label,
            link_button,
            link_label,
        ) = Self::build_output_controls();
        headroom_box.append(&output_controls);

        let headroom_page = Self::scroll_page(&headroom_box);
        let bypass_switch = bypass_switch.1;

        // --- Stack: the three pages, Preset shown by default.
        let stack = gtk4::Stack::new();
        stack.set_hhomogeneous(false);
        stack.set_vhomogeneous(false);
        stack.set_transition_type(gtk4::StackTransitionType::SlideLeftRight);
        stack.add_titled(&preset_page, Some(PAGE_PRESET), "Preset");
        stack.add_titled(&analyzer_page, Some(PAGE_ANALYZER), "Analyzer");
        stack.add_titled(&headroom_page, Some(PAGE_HEADROOM), "Headroom");
        stack.set_visible_child_name(PAGE_PRESET);

        Self {
            container: stack,
            analyzer,
            headroom,
            graph,
            presets,
            monitor_switch,
            monitor_loudness_value,
            monitor_summary,
            bypass_switch,
            output_dropdown,
            fallback_button,
            fallback_label,
            link_button,
            link_label,
        }
    }

    /// Wrap a panel in a vertically-scrolling ScrolledWindow so a tall panel
    /// degrades gracefully instead of overflowing the sidebar.
    fn scroll_page(child: &impl IsA<gtk4::Widget>) -> gtk4::ScrolledWindow {
        let sw = gtk4::ScrolledWindow::new();
        sw.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
        sw.set_vexpand(true);
        sw.set_hexpand(true);
        sw.set_child(Some(child));
        sw
    }

    /// A/B compare (EQ bypass) row for the headroom page. Returns the row
    /// box and the bypass switch.
    fn build_compare_row() -> (gtk4::Box, gtk4::Switch) {
        let compare_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        compare_row.set_css_classes(&["compare-row"]);
        let compare_label = gtk4::Label::new(Some("A/B Compare"));
        compare_label.set_css_classes(&["metric-title"]);
        compare_label.set_hexpand(true);
        compare_row.append(&compare_label);

        let bypass_switch = gtk4::Switch::new();
        bypass_switch.set_valign(gtk4::Align::Center);
        bypass_switch.set_tooltip_text(Some("Bypass the EQ to compare with/without"));
        compare_row.append(&bypass_switch);
        (compare_row, bypass_switch)
    }

    /// "Output Controls" section for the Headroom page: the output-device
    /// dropdown (moved out of the header) plus the per-output-device
    /// auto-preset actions (Fallback / Link to Output).
    fn build_output_controls() -> (
        gtk4::Box,
        gtk4::DropDown,
        gtk4::Button,
        gtk4::Label,
        gtk4::Button,
        gtk4::Label,
    ) {
        let section = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
        section.set_css_classes(&["utility-section"]);

        let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        let title = gtk4::Label::new(Some("Output Controls"));
        title.set_css_classes(&["heading"]);
        header.append(&title);
        section.append(&header);

        // Output device dropdown.
        let output_list = gtk4::StringList::new(&["System Output", "Virtual Sink"]);
        let output_dropdown = gtk4::DropDown::new(Some(output_list), None::<gtk4::Expression>);
        output_dropdown.set_hexpand(true);
        output_dropdown.set_tooltip_text(Some("EQ output device"));
        let output_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        let output_label = gtk4::Label::new(Some("Output"));
        output_label.set_css_classes(&["metric-title"]);
        output_row.append(&output_label);
        output_row.append(&output_dropdown);
        section.append(&output_row);

        // Fallback: default preset for unmatched output devices.
        let fallback_button = gtk4::Button::with_label("Set Fallback");
        fallback_button.set_tooltip_text(Some("Use the current preset for unmatched outputs"));
        let fallback_label = gtk4::Label::new(Some("None"));
        fallback_label.set_css_classes(&["dim-label"]);
        fallback_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        let fallback_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        let fallback_title = gtk4::Label::new(Some("Fallback"));
        fallback_title.set_css_classes(&["metric-title"]);
        fallback_title.set_hexpand(true);
        fallback_row.append(&fallback_title);
        fallback_row.append(&fallback_label);
        fallback_row.append(&fallback_button);
        section.append(&fallback_row);

        // Link to Output: auto-load the current preset for the active output.
        let link_button = gtk4::Button::with_label("Link to Output");
        link_button.set_tooltip_text(Some("Auto-load the current preset for this output device"));
        let link_label = gtk4::Label::new(Some("None"));
        link_label.set_css_classes(&["dim-label"]);
        link_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        let link_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        let link_title = gtk4::Label::new(Some("Link to Output"));
        link_title.set_css_classes(&["metric-title"]);
        link_title.set_hexpand(true);
        link_row.append(&link_title);
        link_row.append(&link_label);
        link_row.append(&link_button);
        section.append(&link_row);

        (
            section,
            output_dropdown,
            fallback_button,
            fallback_label,
            link_button,
            link_label,
        )
    }

    fn build_monitor_panel() -> (gtk4::Box, gtk4::Switch, gtk4::Label, gtk4::Label) {
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
        let smoothing_scale =
            gtk4::Scale::with_range(gtk4::Orientation::Horizontal, 15.0, 95.0, 1.0);
        smoothing_scale.set_size_request(116, -1);
        smoothing_scale.set_hexpand(true);
        smoothing_row.append(&smoothing_scale);
        settings_box.append(&smoothing_row);

        let display_gain_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        let display_gain_label = gtk4::Label::new(Some("Display Gain"));
        display_gain_row.append(&display_gain_label);
        let display_gain_scale =
            gtk4::Scale::with_range(gtk4::Orientation::Horizontal, -12.0, 32.0, 1.0);
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

        (panel, monitor_switch, loudness_value, summary_label)
    }

    /// Show a specific panel by page name (called by the header buttons).
    pub fn show_page(&self, page: &str) {
        self.container.set_visible_child_name(page);
    }
}

impl Default for UtilityPane {
    fn default() -> Self {
        Self::new()
    }
}
