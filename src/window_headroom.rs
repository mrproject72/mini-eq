//! Headroom meter and preamp control widget.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::cairo::Context;
use gtk4::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeadroomState {
    Safe,
    Tight,
    Risk,
    Bypass,
}

impl HeadroomState {
    pub fn css_class(self) -> &'static str {
        match self {
            HeadroomState::Safe => "headroom-panel-safe",
            HeadroomState::Tight => "headroom-panel-tight",
            HeadroomState::Risk => "headroom-panel-risk",
            HeadroomState::Bypass => "headroom-panel-bypass",
        }
    }
}

pub struct HeadroomPanel {
    pub container: gtk4::Box,
    pub preamp_spin: gtk4::SpinButton,
    pub peak_label: gtk4::Label,
    pub state_label: gtk4::Label,
    pub meter_area: gtk4::DrawingArea,
    pub detail_label: gtk4::Label,
    pub set_safe_button: gtk4::Button,
    pub state: Rc<RefCell<HeadroomState>>,
    pub peak_value: Rc<RefCell<f64>>,
}

impl HeadroomPanel {
    pub fn new() -> Self {
        let preamp_adj = gtk4::Adjustment::new(0.0, -24.0, 6.0, 0.5, 1.0, 0.0);
        let preamp_spin = gtk4::SpinButton::new(Some(&preamp_adj), 0.5, 1);
        preamp_spin.set_digits(1);
        preamp_spin.set_width_chars(6);
        preamp_spin.set_tooltip_text(Some("Preamp gain (dB)"));

        let peak_label = gtk4::Label::new(Some("Peak: -inf dBFS"));
        peak_label.set_css_classes(&["numeric"]);

        let state_label = gtk4::Label::new(Some("Safe"));
        state_label.set_css_classes(&["headroom-state-label", "headroom-panel-safe"]);

        let meter_area = gtk4::DrawingArea::new();
        meter_area.set_size_request(260, 14);
        meter_area.set_hexpand(true);
        meter_area.set_valign(gtk4::Align::Center);

        let detail_label = gtk4::Label::new(Some(""));
        detail_label.set_css_classes(&["numeric"]);

        let set_safe_button = gtk4::Button::with_label("Set Safe");
        set_safe_button.set_visible(false);

        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
        container.set_css_classes(&["headroom-panel-safe"]);
        container.set_margin_bottom(8);

        let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        let title = gtk4::Label::new(Some("Headroom"));
        title.set_css_classes(&["heading"]);
        header.append(&title);
        header.set_hexpand(true);
        header.append(&state_label);
        container.append(&header);

        let preamp_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        let preamp_label = gtk4::Label::new(Some("Preamp:"));
        preamp_box.append(&preamp_label);
        preamp_box.append(&preamp_spin);
        container.append(&preamp_box);

        container.append(&peak_label);
        container.append(&meter_area);
        container.append(&detail_label);
        container.append(&set_safe_button);

        let state = Rc::new(RefCell::new(HeadroomState::Safe));
        let peak_value = Rc::new(RefCell::new(f64::NEG_INFINITY));
        let meter_state = state.clone();
        let meter_peak = peak_value.clone();
        meter_area.set_draw_func(move |_area, ctx, width, height| {
            Self::draw_meter(ctx, width, height, *meter_state.borrow(), *meter_peak.borrow());
        });

        Self {
            container,
            preamp_spin,
            peak_label,
            state_label,
            meter_area,
            detail_label,
            set_safe_button,
            state,
            peak_value,
        }
    }

    pub fn set_state(&self, state: HeadroomState) {
        *self.state.borrow_mut() = state;
        self.state_label.set_label(match state {
            HeadroomState::Safe => "Safe",
            HeadroomState::Tight => "Tight",
            HeadroomState::Risk => "Risk",
            HeadroomState::Bypass => "Bypass",
        });
        self.state_label.set_css_classes(&["headroom-state-label", state.css_class()]);
        self.container.set_css_classes(&[state.css_class()]);
        self.set_safe_button.set_visible(state == HeadroomState::Risk);
        self.meter_area.queue_draw();
    }

    pub fn update_peak(&mut self, peak_db: f64) {
        *self.peak_value.borrow_mut() = peak_db;
        let label = if peak_db.is_infinite() || peak_db < -100.0 {
            "Peak: -inf dBFS".to_string()
        } else {
            format!("Peak: {:.1} dBFS", peak_db)
        };
        self.peak_label.set_label(&label);

        let state = if peak_db.is_infinite() || peak_db < -100.0 {
            HeadroomState::Safe
        } else if peak_db > -3.0 {
            HeadroomState::Risk
        } else if peak_db > -6.0 {
            HeadroomState::Tight
        } else {
            HeadroomState::Safe
        };
        self.set_state(state);

        let detail = if peak_db.is_infinite() || peak_db < -100.0 {
            "".to_string()
        } else if peak_db > -3.0 {
            format!("{:.1} dBFS — reduce input or preamp", peak_db)
        } else if peak_db > -6.0 {
            format!("{:.1} dBFS — close to limit", peak_db)
        } else {
            format!("{:.1} dBFS", peak_db)
        };
        self.detail_label.set_label(&detail);
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    fn draw_meter(ctx: &Context, width: i32, height: i32, _state: HeadroomState, peak_db: f64) {
        let w = width as f64;
        let h = height as f64;

        ctx.set_source_rgb(0.12, 0.12, 0.14);
        ctx.rectangle(0.0, 0.0, w, h);
        ctx.fill().unwrap();

        let segment_count = 3.0;
        let segment_w = w / segment_count;
        let peak_norm = if peak_db.is_infinite() || peak_db < -100.0 {
            0.0
        } else {
            ((peak_db + 60.0) / 60.0).clamp(0.0, 1.0)
        };

        let colors = [
            (0.2, 0.8, 0.2),
            (0.9, 0.7, 0.1),
            (0.95, 0.2, 0.2),
        ];

        for i in 0..3 {
            let x = i as f64 * segment_w;
            let seg_h = h * peak_norm * segment_count - i as f64;
            let fill = seg_h > 0.0 && peak_norm > (i as f64 / segment_count);

            if fill {
                ctx.set_source_rgb(colors[i].0, colors[i].1, colors[i].2);
                ctx.rectangle(x + 1.0, h - seg_h.min(h), segment_w - 2.0, seg_h.min(h));
                ctx.fill().unwrap();
            }

            ctx.set_source_rgba(0.3, 0.3, 0.35, 0.6);
            ctx.set_line_width(1.0);
            ctx.rectangle(x, 0.0, segment_w, h);
            ctx.stroke().unwrap();
        }
    }
}

impl Default for HeadroomPanel {
    fn default() -> Self {
        Self::new()
    }
}
