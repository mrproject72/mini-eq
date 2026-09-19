//! Frequency response graph drawing and interaction.

use gtk4::prelude::*;

/// EQ frequency response graph widget.
pub struct EqGraph {
    /// The GTK drawing area for rendering.
    pub widget: gtk4::DrawingArea,
    /// Whether the graph is interactive (drag to adjust bands).
    pub interactive: bool,
}

impl Default for EqGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl EqGraph {
    /// Create a new EQ graph widget.
    pub fn new() -> Self {
        let drawing_area = gtk4::DrawingArea::new();
        drawing_area.set_content_width(800);
        drawing_area.set_content_height(400);
        
        // Set up draw callback
        drawing_area.set_draw_func(|_, cr, width, height| {
            // Clear background
            cr.set_source_rgb(1.0, 1.0, 1.0);
            cr.paint().unwrap();
            
            // Draw grid lines
            cr.set_source_rgb(0.8, 0.8, 0.8);
            cr.set_line_width(1.0);
            
            // Horizontal lines (dB levels)
            for i in 0..=10 {
                let y = (height as f64 / 10.0) * i as f64;
                cr.move_to(0.0, y);
                cr.line_to(width as f64, y);
            }
            
            // Vertical lines (frequency bands - logarithmic)
            for i in 0..=10 {
                let x = (width as f64 / 10.0) * i as f64;
                cr.move_to(x, 0.0);
                cr.line_to(x, height as f64);
            }
            
            cr.stroke().unwrap();
        });

        Self {
            widget: drawing_area,
            interactive: true,
        }
    }

    /// Enable or disable interactive mode.
    pub fn set_interactive(&mut self, interactive: bool) {
        self.interactive = interactive;
    }

    /// Get the underlying GTK widget.
    pub fn get_widget(&self) -> &gtk4::DrawingArea {
        &self.widget
    }

    /// Queue a redraw of the graph.
    pub fn queue_draw(&self) {
        self.widget.queue_draw();
    }
}
