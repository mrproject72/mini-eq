//! Window layout management for mini-eq.
//!
//! Handles window geometry persistence, panel visibility, and layout preferences.

use gtk4::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Window layout configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowLayout {
    /// Main window width.
    pub width: i32,
    /// Main window height.
    pub height: i32,
    /// Whether the analyzer panel is visible.
    pub analyzer_visible: bool,
    /// Whether the presets panel is visible.
    pub presets_visible: bool,
    /// Whether the AutoEq panel is visible.
    pub autoeq_visible: bool,
    /// Splitter position for band faders vs graph.
    pub splitter_position: i32,
}

impl Default for WindowLayout {
    fn default() -> Self {
        Self {
            width: 1360,
            height: 720,
            analyzer_visible: true,
            presets_visible: true,
            autoeq_visible: false,
            splitter_position: 400,
        }
    }
}

/// Window layout manager.
pub struct LayoutManager {
    /// Current layout configuration.
    layout: WindowLayout,
    /// Path to layout config file.
    config_path: Option<PathBuf>,
}

impl LayoutManager {
    /// Create a new layout manager.
    pub fn new() -> Self {
        Self {
            layout: WindowLayout::default(),
            config_path: Self::config_path(),
        }
    }

    /// Get the path to the layout configuration file.
    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("mini-eq").join("layout.json"))
    }

    /// Load layout from disk.
    pub fn load(&mut self) {
        if let Some(path) = &self.config_path {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(layout) = serde_json::from_str(&content) {
                    self.layout = layout;
                    log::info!("Loaded window layout from {:?}", path);
                    return;
                }
            }
        }
        log::info!("Using default window layout");
    }

    /// Save layout to disk.
    pub fn save(&self) -> anyhow::Result<()> {
        if let Some(path) = &self.config_path {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let content = serde_json::to_string_pretty(&self.layout)?;
            std::fs::write(path, content)?;
            log::info!("Saved window layout to {:?}", path);
        }
        Ok(())
    }

    /// Apply layout to a window.
    pub fn apply_to_window(&self, window: &gtk4::Window) {
        window.set_default_size(self.layout.width, self.layout.height);
    }

    /// Get current layout.
    pub fn get_layout(&self) -> &WindowLayout {
        &self.layout
    }

    /// Update window dimensions.
    pub fn update_dimensions(&mut self, width: i32, height: i32) {
        self.layout.width = width;
        self.layout.height = height;
    }

    /// Set panel visibility.
    pub fn set_panel_visible(&mut self, panel: &str, visible: bool) {
        match panel {
            "analyzer" => self.layout.analyzer_visible = visible,
            "presets" => self.layout.presets_visible = visible,
            "autoeq" => self.layout.autoeq_visible = visible,
            _ => {}
        }
    }

    /// Get panel visibility.
    pub fn is_panel_visible(&self, panel: &str) -> bool {
        match panel {
            "analyzer" => self.layout.analyzer_visible,
            "presets" => self.layout.presets_visible,
            "autoeq" => self.layout.autoeq_visible,
            _ => false,
        }
    }
}

impl Default for LayoutManager {
    fn default() -> Self {
        Self::new()
    }
}