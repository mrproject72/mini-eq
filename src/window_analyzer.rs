//! Analyzer panel widget with FFT spectrum visualization.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::cairo::Context;
use gtk4::prelude::*;

use crate::analyzer::analyzer_level_to_display_norm;
use crate::core::clamp_level;

pub struct AnalyzerPanel {
    pub container: gtk4::Box,
    pub drawing_area: gtk4::DrawingArea,
    pub enabled_toggle: gtk4::ToggleButton,
    pub smoothing_scale: gtk4::Scale,
    pub display_gain_scale: gtk4::Scale,
    pub freeze_switch: gtk4::Switch,
    pub lufs_value_label: gtk4::Label,
    pub summary_label: gtk4::Label,
    pub levels: Rc<RefCell<Vec<f64>>>,
}

impl AnalyzerPanel {
    pub fn new() -> Self {
        let enabled_toggle = gtk4::ToggleButton::new();
        enabled_toggle.set_active(true);
        enabled_toggle.set_tooltip_text(Some("Enable analyzer"));

        let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        let title = gtk4::Label::new(Some("Analyzer"));
        title.set_css_classes(&["heading"]);
        header.append(&title);
        header.set_hexpand(true);
        header.append(&enabled_toggle);

        let drawing_area = gtk4::DrawingArea::new();
        drawing_area.set_size_request(-1, 120);
        drawing_area.set_vexpand(true);
        drawing_area.set_hexpand(true);

        let smoothing_adj = gtk4::Adjustment::new(50.0, 15.0, 95.0, 1.0, 5.0, 0.0);
        let smoothing_scale = gtk4::Scale::new(gtk4::Orientation::Horizontal, Some(&smoothing_adj));
        smoothing_scale.set_hexpand(true);
        smoothing_scale.set_tooltip_text(Some("Smoothing"));

        let display_gain_adj = gtk4::Adjustment::new(0.0, -12.0, 32.0, 1.0, 4.0, 0.0);
        let display_gain_scale =
            gtk4::Scale::new(gtk4::Orientation::Horizontal, Some(&display_gain_adj));
        display_gain_scale.set_hexpand(true);
        display_gain_scale.set_tooltip_text(Some("Display Gain"));

        let freeze_switch = gtk4::Switch::new();
        freeze_switch.set_tooltip_text(Some("Freeze"));
        freeze_switch.set_valign(gtk4::Align::Center);

        let lufs_value_label = gtk4::Label::new(Some("-23 LUFS"));
        lufs_value_label.set_css_classes(&["numeric"]);

        let summary_label = gtk4::Label::new(Some("On · -23 LUFS"));
        summary_label.set_css_classes(&["analyzer-summary-label"]);

        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
        container.set_css_classes(&["utility-section"]);
        container.set_margin_bottom(8);
        container.append(&header);
        container.append(&drawing_area);

        let controls = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        controls.append(&smoothing_scale);
        controls.append(&display_gain_scale);
        controls.append(&freeze_switch);
        container.append(&controls);

        container.append(&lufs_value_label);
        container.append(&summary_label);

        let levels = Rc::new(RefCell::new(vec![0.0; 64]));
        let draw_levels = levels.clone();
        drawing_area.set_draw_func(move |_area, ctx, width, height| {
            let levels = draw_levels.borrow();
            Self::draw_spectrum(ctx, width, height, &levels);
        });

        Self {
            container,
            drawing_area,
            enabled_toggle,
            smoothing_scale,
            display_gain_scale,
            freeze_switch,
            lufs_value_label,
            summary_label,
            levels,
        }
    }

    pub fn update(&self, levels: &[f64]) {
        *self.levels.borrow_mut() = levels.to_vec();
        self.drawing_area.queue_draw();
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    pub fn set_height(&self, height: i32) {
        self.drawing_area.set_size_request(-1, height);
        self.drawing_area.queue_draw();
    }

    fn draw_spectrum(ctx: &Context, width: i32, height: i32, levels: &[f64]) {
        let w = width as f64;
        let h = height as f64;

        ctx.set_source_rgb(0.08, 0.08, 0.08);
        ctx.paint().unwrap();

        ctx.set_source_rgba(0.2, 0.2, 0.2, 0.5);
        ctx.set_line_width(0.5);
        for i in 0..5 {
            let y = h * (i + 1) as f64 / 6.0;
            ctx.move_to(0.0, y);
            ctx.line_to(w, y);
            ctx.stroke().unwrap();
        }

        let bar_count = levels.len().min(64);
        let bar_width = w / bar_count as f64;
        for (i, &level) in levels.iter().take(bar_count).enumerate() {
            let norm = analyzer_level_to_display_norm(clamp_level(level), 0.0);
            let bar_height = norm * h * 0.9;
            let x = i as f64 * bar_width;
            let y = h - bar_height;

            let r = norm;
            let g = 1.0 - norm;
            ctx.set_source_rgb(r, g, 0.0);
            ctx.rectangle(x, y, bar_width - 1.0, bar_height);
            ctx.fill().unwrap();
        }
    }
}

impl Default for AnalyzerPanel {
    fn default() -> Self {
        Self::new()
    }
}
