//! Band fader widget - vertical slider only, no per-band controls.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::prelude::*;

use crate::band_fader::EqBandFader;
use crate::core::{EQ_GAIN_MAX_DB, EQ_GAIN_MIN_DB, FilterType};

/// A single band fader (vertical slider only).
pub struct WindowBandFader {
    pub container: gtk4::Box,
    pub fader: Rc<RefCell<EqBandFader>>,
}

impl WindowBandFader {
    pub fn new(
        index: usize,
        frequency: f64,
        gain_db: f64,
        q: f64,
        filter_type: FilterType,
        enabled: bool,
    ) -> Self {
        let fader = EqBandFader::new(
            index,
            Box::new(move |idx, gain| {
                let _ = (idx, gain);
            }),
        );
        {
            let mut f = fader.borrow_mut();
            f.frequency = frequency;
            f.frequency_label = format_frequency_label(frequency);
            f.q_value = q;
            f.q_label = format_q_label(q);
            f.filter_type = filter_type;
            f.filter_type_label = crate::band_fader::filter_type_short_label(filter_type).into();
            f.gain_db = gain_db.clamp(EQ_GAIN_MIN_DB, EQ_GAIN_MAX_DB);
            f.active = enabled;
        }

        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
        container.set_css_classes(&["band-fader-row"]);
        container.set_hexpand(false);
        container.set_vexpand(true);
        container.append(&fader.borrow().container);

        Self { container, fader }
    }

    pub fn update(
        &self,
        gain_db: f64,
        frequency: f64,
        q: f64,
        filter_type: FilterType,
        enabled: bool,
    ) {
        let mut f = self.fader.borrow_mut();
        f.frequency = frequency;
        f.frequency_label = format_frequency_label(frequency);
        f.q_value = q;
        f.q_label = format_q_label(q);
        f.filter_type = filter_type;
        f.filter_type_label = crate::band_fader::filter_type_short_label(filter_type).into();
        f.gain_db = gain_db.clamp(EQ_GAIN_MIN_DB, EQ_GAIN_MAX_DB);
        f.active = enabled;
        drop(f);
        self.fader.borrow().drawing_area.queue_draw();
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }
}

fn format_frequency_label(frequency: f64) -> String {
    if frequency >= 1000.0 {
        format!("{:.2} kHz", frequency / 1000.0)
    } else {
        format!("{:.0} Hz", frequency)
    }
}

fn format_q_label(q: f64) -> String {
    if q >= 10.0 {
        format!("{:.1}", q)
    } else {
        format!("{:.2}", q)
    }
}
