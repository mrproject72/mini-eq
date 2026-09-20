//! User settings persistence (mirrors `settings.py`).
//!
//! Settings are stored as JSON in `$XDG_CONFIG_HOME/mini-eq/settings.json`
//! (or `~/.config/mini-eq/settings.json`).

use std::fs;
use std::path::PathBuf;

use serde_json::{Map, Value};

use crate::core::app_config_file_path;

// ── Constants ────────────────────────────────────────────────────────────────

pub const SETTINGS_FILE_NAME: &str = "settings.json";
pub const SETTINGS_VERSION_KEY: &str = "version";
pub const SETTINGS_VERSION: i32 = 1;

pub const MONITOR_ENABLED_KEY: &str = "monitor_enabled";
pub const APPEARANCE_KEY: &str = "appearance";
pub const BACKGROUND_MODE_KEY: &str = "background_mode";
pub const START_AT_LOGIN_KEY: &str = "start_at_login";
pub const START_ACTIVE_AT_LOGIN_KEY: &str = "start_active_at_login";

const BOOL_SETTINGS_KEYS: &[&str] = &[
    MONITOR_ENABLED_KEY,
    BACKGROUND_MODE_KEY,
    START_AT_LOGIN_KEY,
    START_ACTIVE_AT_LOGIN_KEY,
];

const APPEARANCE_VALUES: &[&str] = &["system", "light", "dark"];

// ── Path helpers ─────────────────────────────────────────────────────────────

pub fn settings_path() -> PathBuf {
    app_config_file_path(SETTINGS_FILE_NAME)
}

// ── Normalization ────────────────────────────────────────────────────────────

fn settings_payload_version(payload: &Map<String, Value>) -> Option<i32> {
    let raw = payload.get(SETTINGS_VERSION_KEY)?;
    if raw.is_boolean() {
        return None;
    }
    match raw.as_i64() {
        Some(v) if v >= 0 => Some(v as i32),
        _ => None,
    }
}

fn normalize_settings_values(payload: &Map<String, Value>) -> Map<String, Value> {
    let mut normalized = Map::new();

    for key in BOOL_SETTINGS_KEYS {
        if let Some(Value::Bool(b)) = payload.get(*key) {
            normalized.insert((*key).to_string(), Value::Bool(*b));
        }
    }

    if let Some(Value::String(s)) = payload.get(APPEARANCE_KEY)
        && APPEARANCE_VALUES.contains(&s.as_str())
    {
        normalized.insert(APPEARANCE_KEY.to_string(), Value::String(s.clone()));
    }

    normalized
}

fn normalize_settings_payload(payload: &Map<String, Value>) -> Map<String, Value> {
    match settings_payload_version(payload) {
        None => return Map::new(),
        Some(v) if v > SETTINGS_VERSION => return Map::new(),
        _ => {}
    }

    let mut normalized = normalize_settings_values(payload);
    normalized.insert(
        SETTINGS_VERSION_KEY.to_string(),
        Value::Number(SETTINGS_VERSION.into()),
    );
    normalized
}

// ── Load / save ──────────────────────────────────────────────────────────────

pub fn load_settings() -> Map<String, Value> {
    let path = settings_path();
    if !path.is_file() {
        return Map::new();
    }

    let data = match fs::read_to_string(&path) {
        Ok(d) => d,
        Err(_) => return Map::new(),
    };

    let payload: Value = match serde_json::from_str(&data) {
        Ok(v) => v,
        Err(_) => return Map::new(),
    };

    match payload.as_object() {
        Some(obj) => normalize_settings_payload(obj),
        None => Map::new(),
    }
}

pub fn save_settings(payload: &Map<String, Value>) -> anyhow::Result<()> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let normalized = normalize_settings_values(payload);
    let mut full = normalized;
    full.insert(
        SETTINGS_VERSION_KEY.to_string(),
        Value::Number(SETTINGS_VERSION.into()),
    );

    let data = serde_json::to_string_pretty(&Value::Object(full))?;
    fs::write(path, format!("{}\n", data))?;
    Ok(())
}

pub fn update_setting(key: &str, value: Value) -> anyhow::Result<()> {
    let mut payload = load_settings();
    payload.insert(key.to_string(), value);
    save_settings(&payload)
}

// ── Typed bool helpers ──────────────────────────────────────────────────────

pub fn load_bool_setting(key: &str, default: bool) -> bool {
    match load_settings().get(key) {
        Some(Value::Bool(b)) => *b,
        _ => default,
    }
}

pub fn save_bool_setting(key: &str, enabled: bool) -> anyhow::Result<()> {
    update_setting(key, Value::Bool(enabled))
}

// ── Appearance ─────────────────────────────────────────────────────────────

pub fn load_appearance() -> String {
    match load_settings().get(APPEARANCE_KEY) {
        Some(Value::String(s)) => s.clone(),
        _ => "system".to_string(),
    }
}

pub fn save_appearance(appearance: &str) -> anyhow::Result<()> {
    update_setting(APPEARANCE_KEY, Value::String(appearance.to_string()))
}

// ── Background mode ─────────────────────────────────────────────────────────

pub fn load_background_mode() -> bool {
    load_bool_setting(BACKGROUND_MODE_KEY, false)
}

pub fn save_background_mode(enabled: bool) -> anyhow::Result<()> {
    save_bool_setting(BACKGROUND_MODE_KEY, enabled)
}

// ── Start at login ─────────────────────────────────────────────────────────

pub fn load_start_at_login() -> bool {
    load_bool_setting(START_AT_LOGIN_KEY, false)
}

pub fn save_start_at_login(enabled: bool) -> anyhow::Result<()> {
    save_bool_setting(START_AT_LOGIN_KEY, enabled)
}

// ── Start active at login ───────────────────────────────────────────────────

pub fn load_start_active_at_login() -> bool {
    load_bool_setting(START_ACTIVE_AT_LOGIN_KEY, false)
}

pub fn save_start_active_at_login(enabled: bool) -> anyhow::Result<()> {
    save_bool_setting(START_ACTIVE_AT_LOGIN_KEY, enabled)
}

// ── Monitor enabled ─────────────────────────────────────────────────────────

pub fn load_monitor_enabled() -> bool {
    load_bool_setting(MONITOR_ENABLED_KEY, true)
}

pub fn save_monitor_enabled(enabled: bool) -> anyhow::Result<()> {
    save_bool_setting(MONITOR_ENABLED_KEY, enabled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_settings_values() {
        let mut payload = Map::new();
        payload.insert("background_mode".to_string(), Value::Bool(true));
        payload.insert("start_at_login".to_string(), Value::Bool(false));
        payload.insert("appearance".to_string(), Value::String("dark".to_string()));
        payload.insert("invalid".to_string(), Value::String("bad".to_string()));

        let normalized = normalize_settings_values(&payload);
        assert_eq!(normalized.len(), 3);
        assert!(normalized.get("invalid").is_none());
    }

    #[test]
    fn test_appearance_values() {
        for v in APPEARANCE_VALUES {
            assert!(["system", "light", "dark"].contains(v));
        }
    }
}
