//! Appearance and theme management for mini-eq.

use gtk4::prelude::*;
use serde::{Deserialize, Serialize};

/// Appearance preference: system, light, or dark.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppearancePreference {
    #[default]
    System,
    Light,
    Dark,
}

impl AppearancePreference {
    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "light" => Self::Light,
            "dark" => Self::Dark,
            _ => Self::System,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
}

/// Apply the appearance preference to the Libadwaita style manager.
pub fn apply_appearance_preference(preference: AppearancePreference) {
    let manager = adw::StyleManager::default();
    match preference {
        AppearancePreference::System => manager.set_color_scheme(adw::ColorScheme::Default),
        AppearancePreference::Light => manager.set_color_scheme(adw::ColorScheme::ForceLight),
        AppearancePreference::Dark => manager.set_color_scheme(adw::ColorScheme::ForceDark),
    }
}

/// Returns true if the current style manager is using a dark color scheme.
pub fn style_manager_is_dark() -> bool {
    adw::StyleManager::default().is_dark()
}

/// Sync the mini-eq CSS class (mini-eq-dark / mini-eq-light) on a widget.
pub fn sync_appearance_css_class(widget: &impl IsA<gtk4::Widget>) {
    let is_dark = style_manager_is_dark();
    let class = if is_dark {
        "mini-eq-dark"
    } else {
        "mini-eq-light"
    };
    widget.set_css_classes(&[class]);
}

/// Build a settings payload for appearance.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppearanceSettings {
    pub preference: String,
    pub window_width: Option<i32>,
    pub window_height: Option<i32>,
    pub window_x: Option<i32>,
    pub window_y: Option<i32>,
}

impl AppearanceSettings {
    pub fn load() -> Self {
        if let Ok(data) = std::fs::read_to_string(
            std::path::Path::new(&std::env::var("HOME").unwrap_or_default())
                .join(".config/mini-eq/appearance.json"),
        ) {
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) {
        let path = std::path::Path::new(&std::env::var("HOME").unwrap_or_default())
            .join(".config/mini-eq");
        let _ = std::fs::create_dir_all(&path);
        let data = serde_json::to_string_pretty(self).unwrap_or_default();
        let _ = std::fs::write(path.join("appearance.json"), data);
    }
}
