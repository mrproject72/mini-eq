//! Headroom meter and preamp control widget.

use std::cell::RefCell;
use std::rc::Rc;

use gtk4::cairo::Context;
use gtk4::prelude::*;

/// Meter axis bounds, matching upstream `window_headroom`.
pub const HEADROOM_METER_MIN_DB: f64 = -12.0;
pub const HEADROOM_METER_MAX_DB: f64 = 24.0;
pub const HEADROOM_SAFE_LIMIT_DB: f64 = -3.0;
pub const HEADROOM_RISK_LIMIT_DB: f64 = 0.0;

/// Target output peak (dBFS) that Auto-Safe keeps the curve under. Matches the
/// one-shot "Set Safe" margin (`peak + 1.0` ⇒ peak lands at −1 dBFS).
pub const AUTO_SAFE_TARGET_DBFS: f64 = -1.0;

/// Compute the preamp that keeps the EQ curve's peak at or below
/// `target_dbfs`, never boosting above 0 dB. `raw_peak_db` is the curve peak
/// with preamp = 0. This is the continuous form of the one-shot "Set Safe":
/// sliding the EQ up auto-lowers the preamp; sliding it down lets the preamp
/// rise back toward 0 (but never above).
pub fn auto_safe_preamp_db(raw_peak_db: f64, target_dbfs: f64) -> f64 {
    if !raw_peak_db.is_finite() {
        return 0.0;
    }
    (target_dbfs - raw_peak_db).clamp(crate::core::EQ_PREAMP_MIN_DB, 0.0)
}

/// Position of `value_db` along the meter, in `0.0..=1.0`.
pub fn headroom_meter_norm(value_db: f64) -> f64 {
    let span = HEADROOM_METER_MAX_DB - HEADROOM_METER_MIN_DB;
    ((value_db - HEADROOM_METER_MIN_DB) / span).clamp(0.0, 1.0)
}

/// Peak text formatting, mirroring upstream `format_headroom_peak_db`.
pub fn format_headroom_peak_db(peak_db: f64) -> String {
    if peak_db > HEADROOM_METER_MAX_DB {
        format!(">{:+.0} dB", HEADROOM_METER_MAX_DB)
    } else if peak_db < HEADROOM_METER_MIN_DB {
        format!("<{:.0} dB", HEADROOM_METER_MIN_DB.abs())
    } else if peak_db < HEADROOM_RISK_LIMIT_DB {
        format!("{:.1} dB", peak_db.abs())
    } else {
        format!("{:+.1} dB", peak_db)
    }
}

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
    pub auto_safe_switch: gtk4::Switch,
    pub auto_safe: Rc<std::cell::Cell<bool>>,
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

        let auto_safe_switch = gtk4::Switch::new();
        auto_safe_switch.set_valign(gtk4::Align::Center);
        auto_safe_switch.set_tooltip_text(Some(
            "Automatically keep the output peak under -1 dBFS as you adjust the EQ",
        ));
        let auto_safe = Rc::new(std::cell::Cell::new(false));
        {
            let auto_safe = auto_safe.clone();
            let preamp_spin = preamp_spin.clone();
            auto_safe_switch.connect_state_set(move |_sw, on| {
                auto_safe.set(on);
                // The auto algorithm owns the preamp while enabled, so the
                // manual control is disabled to avoid fighting it.
                preamp_spin.set_sensitive(!on);
                glib::Propagation::Proceed
            });
        }

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

        // NOTE: the Auto-Safe switch is placed in the MAIN window (between
        // the spectrum and the faders) by build_main_layout, not here. The
        // switch widget + its wiring still live on this panel and are
        // reparented there.

        container.append(&peak_label);
        container.append(&meter_area);
        container.append(&detail_label);
        container.append(&set_safe_button);

        let state = Rc::new(RefCell::new(HeadroomState::Safe));
        let peak_value = Rc::new(RefCell::new(f64::NEG_INFINITY));
        let meter_state = state.clone();
        let meter_peak = peak_value.clone();
        meter_area.set_draw_func(move |_area, ctx, width, height| {
            Self::draw_meter(
                ctx,
                width,
                height,
                *meter_state.borrow(),
                *meter_peak.borrow(),
            );
        });

        Self {
            container,
            preamp_spin,
            peak_label,
            state_label,
            meter_area,
            detail_label,
            set_safe_button,
            auto_safe_switch,
            auto_safe,
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
        self.state_label
            .set_css_classes(&["headroom-state-label", state.css_class()]);
        self.container.set_css_classes(&[state.css_class()]);
        self.set_safe_button
            .set_visible(state == HeadroomState::Risk);
        self.meter_area.queue_draw();
    }

    pub fn update_peak(&mut self, peak_db: f64) {
        *self.peak_value.borrow_mut() = peak_db;

        // Upstream classifies the *estimated curve peak* (not a live signal
        // level): clipping risk above +0.5 dB, tight between -0.5 and +0.5 dB,
        // safe below that.
        let state = if peak_db > 0.5 {
            HeadroomState::Risk
        } else if peak_db > -0.5 {
            HeadroomState::Tight
        } else {
            HeadroomState::Safe
        };
        self.set_state(state);

        let peak_text = format!("Peak: {}", format_headroom_peak_db(peak_db));
        self.peak_label.set_label(&peak_text);

        let detail = if peak_db > 0.5 {
            format!("Lower preamp by {:.1} dB.", peak_db + 1.0)
        } else if peak_db > -0.5 {
            "Small boosts may clip.".to_string()
        } else {
            "Curve stays below 0 dBFS.".to_string()
        };
        self.detail_label.set_label(&detail);

        // Upstream only surfaces "Set Safe" when the curve is actually at risk.
        let needs_fix = peak_db > 0.5;
        self.set_safe_button.set_visible(needs_fix);
        self.set_safe_button.set_sensitive(needs_fix);
    }

    /// Recompute the estimated curve peak from the live band state and refresh
    /// the panel, mirroring upstream `update_status_summary`.
    pub fn update_curve_peak(&mut self, bands: &[crate::core::EqBand], preamp_db: f64) {
        let peak =
            crate::core::estimate_response_peak_db(bands, preamp_db, crate::core::SAMPLE_RATE);
        self.update_peak(peak);
    }

    pub fn preamp_value(&self) -> f64 {
        self.preamp_spin.value()
    }

    /// Whether the Auto-Safe continuous preamp clamp is enabled.
    pub fn auto_safe_enabled(&self) -> bool {
        self.auto_safe.get()
    }

    pub fn set_preamp_value(&self, preamp_db: f64) {
        self.preamp_spin.set_value(preamp_db);
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    fn draw_meter(ctx: &Context, width: i32, height: i32, state: HeadroomState, peak_db: f64) {
        let w = width as f64;
        let h = height as f64;

        ctx.set_source_rgb(0.12, 0.12, 0.14);
        ctx.rectangle(0.0, 0.0, w, h);
        ctx.fill().unwrap();

        // Segment boundaries come from upstream `headroom_meter_norm`: the axis
        // spans HEADROOM_METER_MIN_DB..HEADROOM_METER_MAX_DB (-12..+24 dB), with
        // colour changes at the safe (-3 dB) and risk (0 dB) limits.
        let segments = [
            (
                HEADROOM_METER_MIN_DB,
                HEADROOM_SAFE_LIMIT_DB,
                (0.38, 0.78, 0.50),
            ),
            (
                HEADROOM_SAFE_LIMIT_DB,
                HEADROOM_RISK_LIMIT_DB,
                (0.58, 0.66, 0.76),
            ),
            (
                HEADROOM_RISK_LIMIT_DB,
                HEADROOM_METER_MAX_DB,
                (1.0, 0.35, 0.28),
            ),
        ];

        for (left_db, right_db, color) in segments {
            let left = headroom_meter_norm(left_db) * w;
            let right = headroom_meter_norm(right_db) * w;
            ctx.set_source_rgb(color.0, color.1, color.2);
            ctx.rectangle(left, 0.0, (right - left).max(1.0), h);
            ctx.fill().unwrap();
        }

        // 0 dBFS reference line.
        let zero_x = headroom_meter_norm(0.0) * w;
        ctx.set_source_rgba(0.05, 0.07, 0.10, 0.62);
        ctx.set_line_width(1.0);
        ctx.move_to(zero_x, 0.0);
        ctx.line_to(zero_x, h);
        ctx.stroke().unwrap();

        if state != HeadroomState::Bypass && peak_db.is_finite() {
            let marker_x = headroom_meter_norm(peak_db) * w;
            ctx.set_source_rgba(0.96, 0.98, 1.0, 0.98);
            ctx.arc(marker_x, h / 2.0, 3.2, 0.0, std::f64::consts::TAU);
            ctx.fill().unwrap();
        }
    }
}

impl Default for HeadroomPanel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_safe_preamp_db() {
        // Curve peaks +6 dB -> preamp -7 dB keeps it at -1 dBFS.
        assert!((auto_safe_preamp_db(6.0, -1.0) - (-7.0)).abs() < 1e-9);
        // Curve already low (-3 dB) -> no boost, preamp clamped to 0.
        assert!((auto_safe_preamp_db(-3.0, -1.0) - 0.0).abs() < 1e-9);
        // Exactly at target -> 0 preamp would put peak at target already,
        // so a tiny cut keeps the -1 dBFS margin.
        assert!((auto_safe_preamp_db(0.0, -1.0) - (-1.0)).abs() < 1e-9);
        // Huge peak clamps at the preamp floor.
        assert!((auto_safe_preamp_db(100.0, -1.0) - crate::core::EQ_PREAMP_MIN_DB).abs() < 1e-9);
        // Non-finite peak -> unity (0 dB) preamp.
        assert_eq!(auto_safe_preamp_db(f64::NEG_INFINITY, -1.0), 0.0);
    }

    #[test]
    fn test_headroom_meter_norm_endpoints_and_clamp() {
        assert!((headroom_meter_norm(HEADROOM_METER_MIN_DB) - 0.0).abs() < 1e-9);
        assert!((headroom_meter_norm(HEADROOM_METER_MAX_DB) - 1.0).abs() < 1e-9);
        assert!((headroom_meter_norm(HEADROOM_METER_MIN_DB - 50.0) - 0.0).abs() < 1e-9);
        assert!((headroom_meter_norm(HEADROOM_METER_MAX_DB + 50.0) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_format_headroom_peak_db_matches_upstream() {
        assert_eq!(
            format_headroom_peak_db(HEADROOM_METER_MAX_DB + 1.0),
            ">+24 dB"
        );
        assert_eq!(
            format_headroom_peak_db(HEADROOM_METER_MIN_DB - 1.0),
            "<12 dB"
        );
        assert_eq!(format_headroom_peak_db(-4.5), "4.5 dB");
        assert_eq!(format_headroom_peak_db(0.0), "+0.0 dB");
        assert_eq!(format_headroom_peak_db(2.5), "+2.5 dB");
    }
}
