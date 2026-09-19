//! Screenshot and capture functionality for mini-eq.
//!
//! Provides ability to capture the UI or graph for sharing presets.

use anyhow::Context;
use gtk4::prelude::*;

/// Screenshot capture options.
#[derive(Debug, Clone)]
pub struct ScreenshotOptions {
    /// Include window decorations.
    pub include_decorations: bool,
    /// Output format (png, jpg, webp).
    pub format: String,
    /// Quality for lossy formats (0-100).
    pub quality: u8,
    /// Scale factor for output.
    pub scale: f32,
}

impl Default for ScreenshotOptions {
    fn default() -> Self {
        Self {
            include_decorations: false,
            format: "png".to_string(),
            quality: 95,
            scale: 1.0,
        }
    }
}

/// Screenshot manager for capturing UI state.
pub struct ScreenshotManager {
    /// Default options for screenshots.
    options: ScreenshotOptions,
}

impl Default for ScreenshotManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ScreenshotManager {
    /// Create a new screenshot manager.
    pub fn new() -> Self {
        Self {
            options: ScreenshotOptions::default(),
        }
    }

    /// Create with custom options.
    pub fn with_options(options: ScreenshotOptions) -> Self {
        Self { options }
    }

    /// Capture a widget to a pixbuf.
    pub fn capture_widget(&self, widget: &gtk4::Widget) -> anyhow::Result<gdk4::Texture> {
        // Get the paintable for the widget
        let paintable = gtk4::SnapshotPaintable::for_widget(widget);
        
        // Create a snapshot
        let snapshot = gtk4::Snapshot::new();
        
        // Render the widget into the snapshot
        widget.snapshot(snapshot.clone());
        
        // Get the texture from the snapshot
        let texture = snapshot.to_texture();
        
        Ok(texture)
    }

    /// Save a texture to file.
    pub fn save_texture(
        &self,
        texture: &gdk4::Texture,
        path: &std::path::Path,
    ) -> anyhow::Result<()> {
        let format = self.options.format.to_lowercase();
        
        match format.as_str() {
            "png" => {
                // PNG is the native format for GDK textures
                if let Some(bytes) = texture.save_to_png_bytes() {
                    std::fs::write(path, bytes.as_ref())
                        .context("Failed to write PNG file")?;
                } else {
                    anyhow::bail!("Failed to save texture as PNG");
                }
            }
            _ => {
                // For other formats, we'd need additional libraries like image-rs
                // For now, save as PNG and note the limitation
                log::warn!("Only PNG format is fully supported, saving as PNG");
                if let Some(bytes) = texture.save_to_png_bytes() {
                    std::fs::write(path, bytes.as_ref())
                        .context("Failed to write PNG file")?;
                } else {
                    anyhow::bail!("Failed to save texture");
                }
            }
        }

        log::info!("Saved screenshot to {:?}", path);
        Ok(())
    }

    /// Capture and save a widget directly to file.
    pub fn capture_widget_to_file(
        &self,
        widget: &gtk4::Widget,
        path: &std::path::Path,
    ) -> anyhow::Result<()> {
        let texture = self.capture_widget(widget)?;
        self.save_texture(&texture, path)
    }

    /// Capture the frequency graph area.
    pub fn capture_graph(
        &self,
        graph_widget: &gtk4::Widget,
    ) -> anyhow::Result<gdk4::Texture> {
        log::info!("Capturing frequency graph...");
        self.capture_widget(graph_widget)
    }

    /// Capture the entire application window.
    pub fn capture_window(
        &self,
        window: &gtk4::Window,
    ) -> anyhow::Result<gdk4::Texture> {
        log::info!("Capturing application window...");
        
        // Force a redraw first
        window.queue_draw();
        
        // Give GTK time to render
        while gtk4::events_pending() {
            gtk4::main_iteration();
        }
        
        self.capture_widget(window.as_ref())
    }

    /// Set the output format.
    pub fn set_format(&mut self, format: &str) {
        self.options.format = format.to_string();
    }

    /// Set the quality level.
    pub fn set_quality(&mut self, quality: u8) {
        self.options.quality = quality.min(100);
    }

    /// Set the scale factor.
    pub fn set_scale(&mut self, scale: f32) {
        self.options.scale = scale.max(0.1).min(4.0);
    }

    /// Get current options.
    pub fn get_options(&self) -> &ScreenshotOptions {
        &self.options
    }
}

/// Quick function to capture a widget to file.
pub fn capture_widget_to_file(
    widget: &gtk4::Widget,
    path: &std::path::Path,
) -> anyhow::Result<()> {
    let manager = ScreenshotManager::new();
    manager.capture_widget_to_file(widget, path)
}

/// Generate a preset preview image.
pub fn generate_preset_preview(
    graph_widget: &gtk4::Widget,
    output_path: &std::path::Path,
) -> anyhow::Result<()> {
    log::info!("Generating preset preview at {:?}", output_path);
    
    let manager = ScreenshotManager::new();
    manager.capture_widget_to_file(graph_widget, output_path)
}