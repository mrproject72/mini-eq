//! Desktop integration for mini-eq.
//!
//! Handles .desktop file installation, autostart configuration,
//! and system tray integration.

use anyhow::Context;
use std::path::{Path, PathBuf};
use std::fs;

/// Desktop entry content for mini-eq.
const DESKTOP_ENTRY: &str = r#"[Desktop Entry]
Name=Mini EQ
Comment=System-wide parametric equalizer for PipeWire
Exec=mini-eq %U
Icon=audio-card
Terminal=false
Type=Application
Categories=Audio;Settings;HardwareSettings;
Keywords=equalizer;audio;pipewire;sound;
StartupNotify=true
MimeType=x-scheme-handler/mini-eq;
"#;

/// Autostart desktop entry (minimal version for startup).
const AUTOSTART_ENTRY: &str = r#"[Desktop Entry]
Name=Mini EQ (Background)
Comment=System-wide parametric equalizer for PipeWire (background mode)
Exec=mini-eq --background
Icon=audio-card
Terminal=false
Type=Application
Categories=Audio;Settings;
Keywords=equalizer;audio;pipewire;sound;
Hidden=false
NoDisplay=true
X-GNOME-Autostart-enabled=true
X-GNOME-Autostart-Phase=Applications
"#;

/// Desktop integration manager.
pub struct DesktopIntegration {
    /// Whether integration is active.
    active: bool,
}

impl DesktopIntegration {
    /// Create a new desktop integration manager.
    pub fn new() -> Self {
        Self { active: false }
    }

    /// Initialize desktop integration.
    pub fn init(&mut self) -> anyhow::Result<()> {
        log::info!("Initializing desktop integration...");

        // Ensure application directories exist
        let apps_dir = get_user_apps_directory()?;
        let autostart_dir = get_user_autostart_directory()?;

        fs::create_dir_all(&apps_dir)?;
        fs::create_dir_all(&autostart_dir)?;

        self.active = true;
        log::info!("Desktop integration initialized");
        Ok(())
    }

    /// Install the main .desktop file.
    pub fn install_desktop_file(&self) -> anyhow::Result<PathBuf> {
        let apps_dir = get_user_apps_directory()?;
        let desktop_path = apps_dir.join("mini-eq.desktop");

        fs::write(&desktop_path, DESKTOP_ENTRY)
            .context("Failed to write desktop file")?;

        log::info!("Installed desktop file at {:?}", desktop_path);
        Ok(desktop_path)
    }

    /// Install autostart .desktop file.
    pub fn install_autostart(&self) -> anyhow::Result<PathBuf> {
        let autostart_dir = get_user_autostart_directory()?;
        let autostart_path = autostart_dir.join("mini-eq.desktop");

        fs::write(&autostart_path, AUTOSTART_ENTRY)
            .context("Failed to write autostart file")?;

        log::info!("Installed autostart file at {:?}", autostart_path);
        Ok(autostart_path)
    }

    /// Remove autostart .desktop file.
    pub fn remove_autostart(&self) -> anyhow::Result<()> {
        let autostart_dir = get_user_autostart_directory()?;
        let autostart_path = autostart_dir.join("mini-eq.desktop");

        if autostart_path.exists() {
            fs::remove_file(&autostart_path)
                .context("Failed to remove autostart file")?;
            log::info!("Removed autostart file");
        }

        Ok(())
    }

    /// Check if autostart is enabled.
    pub fn is_autostart_enabled(&self) -> bool {
        let autostart_dir = match get_user_autostart_directory() {
            Ok(d) => d,
            Err(_) => return false,
        };
        
        autostart_dir.join("mini-eq.desktop").exists()
    }

    /// Enable or disable autostart.
    pub fn set_autostart(&self, enable: bool) -> anyhow::Result<()> {
        if enable {
            self.install_autostart()?;
        } else {
            self.remove_autostart()?;
        }
        Ok(())
    }

    /// Get the path to the icon directory.
    pub fn get_icon_dir() -> Option<PathBuf> {
        dirs::data_local_dir().map(|p| p.join("icons"))
    }

    /// Install application icon.
    pub fn install_icon(&self, icon_data: &[u8]) -> anyhow::Result<PathBuf> {
        let icon_dir = Self::get_icon_dir()
            .ok_or_else(|| anyhow::anyhow!("Icon directory not found"))?;
        
        fs::create_dir_all(&icon_dir)?;
        
        let icon_path = icon_dir.join("mini-eq.png");
        fs::write(&icon_path, icon_data)
            .context("Failed to write icon file")?;

        log::info!("Installed icon at {:?}", icon_path);
        Ok(icon_path)
    }

    /// Check if integration is active.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Cleanup integration files (optional).
    pub fn cleanup(&self) -> anyhow::Result<()> {
        log::info!("Cleaning up desktop integration...");
        // Optionally remove installed files
        Ok(())
    }
}

impl Default for DesktopIntegration {
    fn default() -> Self {
        Self::new()
    }
}

/// Get the user's applications directory for .desktop files.
fn get_user_apps_directory() -> anyhow::Result<PathBuf> {
    dirs::data_local_dir()
        .map(|p| p.join("applications"))
        .or_else(|| {
            dirs::home_dir().map(|p| {
                p.join(".local")
                    .join("share")
                    .join("applications")
            })
        })
        .ok_or_else(|| anyhow::anyhow!("Could not determine applications directory"))
}

/// Get the user's autostart directory.
fn get_user_autostart_directory() -> anyhow::Result<PathBuf> {
    dirs::config_local_dir()
        .map(|p| p.join("autostart"))
        .or_else(|| {
            dirs::home_dir().map(|p| {
                p.join(".config")
                    .join("autostart")
            })
        })
        .ok_or_else(|| anyhow::anyhow!("Could not determine autostart directory"))
}

/// Check if running as a Flatpak.
pub fn is_flatpak() -> bool {
    Path::new("/.flatpak-info").exists()
}

/// Get Flatpak application ID if running as Flatpak.
pub fn get_flatpak_id() -> Option<String> {
    std::env::var("FLATPAK_ID").ok()
}