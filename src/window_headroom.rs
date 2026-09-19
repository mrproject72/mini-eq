//! Headroom meter and preamp control.

use gtk4::prelude::*;

/// Headroom meter panel showing current headroom and preamp level.
pub struct HeadroomPanel {
    /// The main container widget.
    pub container: gtk4::Box,
    /// Current headroom in dB.
    pub headroom_db: f32,
    /// Preamp gain in dB.
    pub preamp_gain_db: f32,
    /// Whether clipping is detected.
    pub clipping: bool,
}

impl Default for HeadroomPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl HeadroomPanel {
    /// Create a new headroom panel.
    pub fn new() -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
        container.set_margin_start(8);
        container.set_margin_end(8);
        container.set_margin_top(8);
        container.set_margin_bottom(8);

        // Headroom label
        let headroom_label = gtk4::Label::new(Some("Headroom: 0.0 dB"));
        headroom_label.set_css_classes(&["heading"]);
        headroom_label.set_halign(gtk4::Align::Center);

        // Preamp label
        let preamp_label = gtk4::Label::new(Some("Preamp: 0.0 dB"));
        preamp_label.set_css_classes(&["dim-label"]);
        preamp_label.set_halign(gtk4::Align::Center);

        // Clipping indicator
        let clip_label = gtk4::Label::new(Some("⚠ CLIPPING"));
        clip_label.set_css_classes(&["error", "heading"]);
        clip_label.set_visible(false);
        clip_label.set_halign(gtk4::Align::Center);

        container.append(&headroom_label);
        container.append(&preamp_label);
        container.append(&clip_label);

        Self {
            container,
            headroom_db: 0.0,
            preamp_gain_db: 0.0,
            clipping: false,
        }
    }

    /// Update the headroom display.
    pub fn update_headroom(&mut self, headroom_db: f32) {
        self.headroom_db = headroom_db;
        
        if let Some(label) = self.container.first_child().and_then(|c| c.downcast::<gtk4::Label>().ok()) {
            label.set_text(&format!("Headroom: {:.1} dB", headroom_db));
        }

        // Check for clipping (headroom < 0.5 dB)
        self.clipping = headroom_db < 0.5;
        if let Some(clip_label) = self.container.last_child().and_then(|c| c.downcast::<gtk4::Label>().ok()) {
            clip_label.set_visible(self.clipping);
        }
    }

    /// Update the preamp gain display.
    pub fn update_preamp(&mut self, preamp_gain_db: f32) {
        self.preamp_gain_db = preamp_gain_db;
        
        if let Some(label) = self.container.nth_child(1).and_then(|c| c.downcast::<gtk4::Label>().ok()) {
            label.set_text(&format!("Preamp: {:.1} dB", preamp_gain_db));
        }
    }

    /// Get the underlying GTK container.
    pub fn get_container(&self) -> &gtk4::Box {
        &self.container
    }

    /// Check if clipping is occurring.
    pub fn is_clipping(&self) -> bool {
        self.clipping
    }
}
