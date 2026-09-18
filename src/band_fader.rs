//! Custom EQ band fader widget (Cairo-rendered slider).

use gio::glib;
use gtk4::prelude::*;

use crate::core::{EQ_GAIN_MAX_DB, EQ_GAIN_MIN_DB, FilterType};

const CONTENT_W: i32 = 72;
const CONTENT_H: i32 = 182;
const GAIN_STEP_DB: f64 = 0.5;

/// A single EQ band fader control.
pub struct EqBandFader {
    pub container: gtk4::Box,
    pub drawing_area: gtk4::DrawingArea,
    pub index: usize,
    pub gain_db: f64,
    pub frequency: f64,
    pub q_value: f64,
    pub filter_type: FilterType,
    pub selected: bool,
    pub active: bool,
    pub muted: bool,
    pub soloed: bool,
}

impl EqBandFader {
    pub fn new(index: usize) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
        container.set_css_classes(&["eq-band-box"]);
        container.set_size_request(CONTENT_W, CONTENT_H);
        container.set_hexpand(false);

        let drawing_area = gtk4::DrawingArea::new();
        drawing_area.set_content_width(CONTENT_W);
        drawing_area.set_content_height(CONTENT_H);
        drawing_area.set_hexpand(false);
        drawing_area.set_vexpand(true);
        drawing_area.set_tooltip_text(Some("Band Gain"));

        // Drag gesture for gain adjustment
        let drag = gtk4::GestureDrag::new();
        let da = drawing_area.clone();
        drag.connect_begin(move |gesture, _event_seq| {
            gesture.set_state(gtk4::EventSequenceState::Claimed);
            da.queue_draw();
        });
        drag.connect_end(move |_gesture, _event_seq| {});
        drawing_area.add_controller(drag);

        // Scroll wheel
        let scroll = gtk4::EventControllerScroll::new(gtk4::EventControllerScrollFlags::VERTICAL);
        let da3 = drawing_area.clone();
        scroll.connect_scroll(move |_controller, _dx, dy| {
            let step = if dy < 0.0 {
                GAIN_STEP_DB
            } else {
                -GAIN_STEP_DB
            };
            da3.set_tooltip_text(Some(&format!("Gain step: {:.1} dB", step)));
            glib::Propagation::Proceed
        });
        drawing_area.add_controller(scroll);

        // Click to select
        let click = gtk4::GestureClick::new();
        let da4 = drawing_area.clone();
        click.connect_released(move |_gesture, _n_press, _x, _y| {
            da4.grab_focus();
        });
        drawing_area.add_controller(click);

        // Keyboard focus
        let focus = gtk4::EventControllerFocus::new();
        let da5 = drawing_area.clone();
        focus.connect_enter(move |_controller| {
            da5.queue_draw();
        });
        let da6 = drawing_area.clone();
        focus.connect_leave(move |_controller| {
            da6.queue_draw();
        });
        drawing_area.add_controller(focus);

        // Keyboard
        let key = gtk4::EventControllerKey::new();
        let da7 = drawing_area.clone();
        key.connect_key_pressed(move |_controller, key, _state, _hardware| {
            if key == gtk4::gdk::Key::space || key == gtk4::gdk::Key::Return {
                da7.set_tooltip_text(Some("Activated"));
                glib::Propagation::Proceed
            } else {
                glib::Propagation::Proceed
            }
        });
        drawing_area.add_controller(key);

        container.append(&drawing_area);

        EqBandFader {
            container,
            drawing_area,
            index,
            gain_db: 0.0,
            frequency: 1000.0,
            q_value: 0.7,
            filter_type: FilterType::Bell,
            selected: false,
            active: true,
            muted: false,
            soloed: false,
        }
    }

    pub fn set_band_state(&mut self, gain: f64, frequency: f64, q: f64, filter_type: FilterType) {
        self.gain_db = gain.clamp(EQ_GAIN_MIN_DB, EQ_GAIN_MAX_DB);
        self.frequency = frequency;
        self.q_value = q;
        self.filter_type = filter_type;
        self.drawing_area.queue_draw();
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }
}

/// Filter type short label for the fader badge.
pub fn filter_type_short_label(ft: FilterType) -> &'static str {
    match ft {
        FilterType::Off => "Off",
        FilterType::Bell => "Bell",
        FilterType::HiPass => "HP",
        FilterType::HiShelf => "HS",
        FilterType::LoPass => "LP",
        FilterType::LoShelf => "LS",
        FilterType::Notch => "Notch",
        FilterType::Resonance => "Res",
        FilterType::Allpass => "AP",
        FilterType::Bandpass => "BP",
        FilterType::LadderPass => "LdP",
        FilterType::LadderRej => "LdR",
    }
}

/// Format a gain value for display.
pub fn format_gain_label(gain: f64) -> String {
    if gain.abs() < 0.005 {
        "0.0 dB".to_string()
    } else {
        format!("{:+.1} dB", gain)
    }
}
