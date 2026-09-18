//! Configuration persistence and settings management.
//!
//! Handles loading/saving user preferences to JSON configuration file.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Application settings persisted to disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    /// Master preamp gain in dB.
    pub preamp_gain_db: f32,
    /// Whether the equalizer is enabled.
    pub eq_enabled: bool,
    /// Selected preset name.
    pub selected_preset: Option<String>,
    /// Analyzer display gain in dB.
    pub analyzer_display_gain_db: f64,
    /// Analyzer response speed.
    pub analyzer_response_speed: f64,
    /// AutoEq profile path or URL.
    pub autoeq_profile: Option<String>,
    /// Window width.
    pub window_width: i32,
    /// Window height.
    pub window_height: i32,
    /// Appearance preference (light/dark/system).
    pub appearance: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            preamp_gain_db: 0.0,
            eq_enabled: true,
            selected_preset: None,
            analyzer_display_gain_db: 0.0,
            analyzer_response_speed: 1.0,
            autoeq_profile: None,
            window_width: 1360,
            window_height: 720,
            appearance: "system".to_string(),
        }
    }
}

impl Settings {
    /// Get the configuration directory path.
    pub fn config_dir() -> Option<PathBuf> {
        dirs::config_dir().map(|p| p.join("mini-eq"))
    }

    /// Get the path to the settings file.
    pub fn config_path() -> Option<PathBuf> {
        Self::config_dir().map(|p| p.join("settings.json"))
    }

    /// Load settings from disk, or return defaults if not found.
    pub fn load() -> Self {
        let path = match Self::config_path() {
            Some(p) => p,
            None => return Self::default(),
        };

        match std::fs::read_to_string(&path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_else(|e| {
                log::warn!("Failed to parse settings: {}", e);
                Self::default()
            }),
            Err(e) => {
                log::info!("No settings file found at {:?}: {}", path, e);
                Self::default()
            }
        }
    }

    /// Save settings to disk.
    pub fn save(&self) -> anyhow::Result<()> {
        let dir = Self::config_dir().ok_or_else(|| anyhow::anyhow!("Config directory not found"))?;
        
        std::fs::create_dir_all(&dir)?;
        
        let path = dir.join("settings.json");
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        
        Ok(())
    }

    /// Ensure config directory exists.
    pub fn ensure_config_dir() -> anyhow::Result<PathBuf> {
        let dir = Self::config_dir().ok_or_else(|| anyhow::anyhow!("Config directory not found"))?;
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }
}