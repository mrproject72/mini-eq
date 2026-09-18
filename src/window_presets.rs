//! Preset management for mini-eq.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::core::EqBand;

/// A serializable preset payload.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PresetPayload {
    pub name: String,
    pub bands: Vec<EqBand>,
    pub preamp_gain: f64,
    pub output_links: HashMap<String, String>,
    pub fallback_preset: Option<String>,
}

impl PresetPayload {
    pub fn new(name: &str, bands: &[EqBand], preamp: f64) -> Self {
        Self {
            name: name.to_string(),
            bands: bands.to_vec(),
            preamp_gain: preamp,
            output_links: HashMap::new(),
            fallback_preset: None,
        }
    }

    pub fn state_signature(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.name.hash(&mut hasher);
        for band in &self.bands {
            band.index.hash(&mut hasher);
            band.frequency.to_bits().hash(&mut hasher);
            band.gain_db.to_bits().hash(&mut hasher);
            band.q.to_bits().hash(&mut hasher);
            band.filter_type.hash(&mut hasher);
            band.enabled.hash(&mut hasher);
        }
        self.preamp_gain.to_bits().hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

/// Get the preset storage directory.
pub fn preset_storage_dir() -> PathBuf {
    PathBuf::from(&std::env::var("HOME").unwrap_or_default()).join(".config/mini-eq/presets")
}

/// Ensure the preset storage directory exists.
pub fn ensure_preset_storage_dir() -> PathBuf {
    let dir = preset_storage_dir();
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// List all preset names (sorted).
pub fn list_preset_names() -> Vec<String> {
    let dir = ensure_preset_storage_dir();
    let mut names = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "json")
                && let Some(stem) = path.file_stem()
            {
                names.push(stem.to_string_lossy().to_string());
            }
        }
    }
    names.sort();
    names
}

/// Get the file path for a preset name.
pub fn preset_path_for_name(name: &str) -> PathBuf {
    ensure_preset_storage_dir().join(format!("{}.json", name))
}

/// Load a preset from a file.
pub fn load_preset_file(path: &PathBuf) -> Result<PresetPayload, String> {
    let data =
        std::fs::read_to_string(path).map_err(|e| format!("Failed to read preset: {}", e))?;
    serde_json::from_str(&data).map_err(|e| format!("Invalid preset JSON: {}", e))
}

/// Write a preset to a file.
pub fn write_preset_file(path: &PathBuf, payload: &PresetPayload) -> Result<(), String> {
    let data = serde_json::to_string_pretty(payload)
        .map_err(|e| format!("Failed to serialize preset: {}", e))?;
    std::fs::write(path, data).map_err(|e| format!("Failed to write preset: {}", e))
}

/// Delete a preset file.
pub fn delete_preset_file(name: &str) -> Result<(), String> {
    let path = preset_path_for_name(name);
    std::fs::remove_file(&path).map_err(|e| format!("Failed to delete preset: {}", e))
}

/// Sanitize a preset name.
pub fn sanitize_preset_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_whitespace() { '_' } else { c })
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .collect()
}
