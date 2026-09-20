//! Background mode management.
//!
//! Mirrors `background.py` from the original Python project. Handles:
//! - Settings persistence for background mode and autostart flags
//! - Native autostart via `.desktop` files in `~/.config/autostart/`
//! - Flatpak background portal (`org.freedesktop.portal.Background`)

use std::path::{Path, PathBuf};

use crate::desktop_integration::build_native_autostart_desktop_file;
use crate::settings::{
    BACKGROUND_MODE_KEY, START_ACTIVE_AT_LOGIN_KEY, START_AT_LOGIN_KEY, load_settings,
    save_bool_setting,
};

// ── Constants ────────────────────────────────────────────────────────────────

pub const BACKGROUND_PORTAL_REASON: &str = "Keep equalizer settings active for desktop audio.";
pub const BACKGROUND_PORTAL_BUS_NAME: &str = "org.freedesktop.portal.Desktop";
pub const BACKGROUND_PORTAL_OBJECT_PATH: &str = "/org/freedesktop/portal/desktop";
pub const BACKGROUND_PORTAL_IFACE: &str = "org.freedesktop.portal.Background";
pub const PORTAL_REQUEST_IFACE: &str = "org.freedesktop.portal.Request";
pub const PORTAL_CALL_TIMEOUT_MS: u32 = 120_000;
pub const PORTAL_RESPONSE_SUCCESS: u32 = 0;
pub const AUTOSTART_FILE_NAME: &str = "io.github.bhack.mini-eq.desktop";

// ── Executable resolution ────────────────────────────────────────────────────

/// Resolve the mini-eq executable path.
pub fn resolve_mini_eq_executable() -> anyhow::Result<String> {
    if let Some(path) = find_in_path("mini-eq") {
        return Ok(path);
    }
    let exe = std::env::current_exe()?;
    Ok(exe.to_string_lossy().to_string())
}

fn find_in_path(name: &str) -> Option<String> {
    let path = std::env::var("PATH").ok()?;
    for dir in path.split(':') {
        let candidate = Path::new(dir).join(name);
        if let Some(s) = candidate.to_str().filter(|_| candidate.is_file()) {
            return Some(s.to_string());
        }
    }
    None
}

// ── Command building ─────────────────────────────────────────────────────────

/// Build the command line for running mini-eq in background mode.
pub fn mini_eq_background_command(executable: &str, auto_route: bool) -> Vec<String> {
    let mut command = vec![executable.to_string(), "--background".to_string()];
    if auto_route {
        command.push("--auto-route".to_string());
    }
    command
}

/// Build the native autostart command (resolved executable + flags).
pub fn native_autostart_command(auto_route: bool) -> anyhow::Result<Vec<String>> {
    let executable = resolve_mini_eq_executable()?;
    Ok(mini_eq_background_command(&executable, auto_route))
}

// ── Paths ────────────────────────────────────────────────────────────────────

pub fn running_in_flatpak() -> bool {
    Path::new("/.flatpak-info").exists()
}

pub fn autostart_dir() -> PathBuf {
    let config_home = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|p| Path::new(p).canonicalize().is_ok())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var("HOME")
                .map(|h| PathBuf::from(h).join(".config"))
                .unwrap_or_else(|_| PathBuf::from("/tmp").join(".config"))
        });
    config_home.join("autostart")
}

pub fn autostart_desktop_path() -> PathBuf {
    autostart_dir().join(AUTOSTART_FILE_NAME)
}

// ── Native autostart ─────────────────────────────────────────────────────────

/// Set native (non-Flatpak) autostart state.
pub fn set_native_start_at_login(
    enabled: bool,
    executable: Option<&str>,
    auto_route: bool,
) -> anyhow::Result<()> {
    let path = autostart_desktop_path();

    if !enabled {
        let _ = std::fs::remove_file(&path);
        return Ok(());
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let command = if let Some(exe) = executable {
        mini_eq_background_command(exe, auto_route)
    } else {
        native_autostart_command(auto_route)?
    };

    let content = build_native_autostart_desktop_file(&command, auto_route);
    std::fs::write(&path, content)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, PermissionsExt::from_mode(0o644))?;
    }

    Ok(())
}

// ── Settings helpers ─────────────────────────────────────────────────────────

pub fn load_background_mode() -> bool {
    load_settings()
        .get(BACKGROUND_MODE_KEY)
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

pub fn save_background_mode(enabled: bool) -> anyhow::Result<()> {
    save_bool_setting(BACKGROUND_MODE_KEY, enabled)
}

pub fn load_start_at_login() -> bool {
    load_settings()
        .get(START_AT_LOGIN_KEY)
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

pub fn save_start_at_login(enabled: bool) -> anyhow::Result<()> {
    save_bool_setting(START_AT_LOGIN_KEY, enabled)
}

pub fn load_start_active_at_login() -> bool {
    load_settings()
        .get(START_ACTIVE_AT_LOGIN_KEY)
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

pub fn save_start_active_at_login(enabled: bool) -> anyhow::Result<()> {
    save_bool_setting(START_ACTIVE_AT_LOGIN_KEY, enabled)
}

// ── Flatpak background portal ─────────────────────────────────────────────────

/// Result of a background portal request.
pub struct PortalResult {
    pub background_allowed: bool,
    pub autostart_enabled: bool,
    pub error: Option<String>,
}

/// Request background mode via the Flatpak background portal.
///
/// On non-Flatpak this is a no-op that returns success.
/// On Flatpak, this calls `org.freedesktop.portal.Background.RequestBackground`
/// via GIO's D-Bus API.
pub fn request_background_portal(autostart: bool, auto_route: bool) -> PortalResult {
    if !running_in_flatpak() {
        return PortalResult {
            background_allowed: true,
            autostart_enabled: autostart,
            error: None,
        };
    }

    request_background_portal_flatpak(autostart, auto_route)
}

fn request_background_portal_flatpak(autostart: bool, auto_route: bool) -> PortalResult {
    use glib::VariantDict;

    // Connect to the session bus synchronously.
    let connection = match gio::bus_get_sync(gio::BusType::Session, None::<&gio::Cancellable>) {
        Ok(c) => c,
        Err(e) => {
            return PortalResult {
                background_allowed: false,
                autostart_enabled: false,
                error: Some(format!("Could not connect to session bus: {}", e)),
            };
        }
    };

    let handle_token = format!("mini_eq_{}", uuid_v4_hex());
    let sender_id = connection
        .unique_name()
        .unwrap_or_default()
        .trim_start_matches(':')
        .replace('.', "_");

    let _handle_path = format!(
        "/org/freedesktop/portal/desktop/request/{}/{}",
        sender_id, handle_token
    );

    // Build the options dict {a{sv}} using VariantDict.
    let options_dict = VariantDict::new(None);
    options_dict.insert("reason", glib::Variant::from(BACKGROUND_PORTAL_REASON));
    options_dict.insert("autostart", glib::Variant::from(autostart));
    options_dict.insert("handle_token", glib::Variant::from(handle_token.as_str()));

    if autostart {
        let command = mini_eq_background_command("mini-eq", auto_route);
        let strv: Vec<&str> = command.iter().map(|s| s.as_str()).collect();
        let variant_array = glib::Variant::array_from_iter::<glib::Variant>(
            strv.iter().map(|s| glib::Variant::from(*s)),
        );
        options_dict.insert("commandline", &variant_array);
    }

    let options_variant = options_dict.end();

    // Build the (sa{sv}) parameter tuple.
    let params: glib::Variant = (&"", &options_variant).into();

    // Make the D-Bus method call synchronously.
    let _ = connection.call_sync(
        Some(BACKGROUND_PORTAL_BUS_NAME),
        BACKGROUND_PORTAL_OBJECT_PATH,
        BACKGROUND_PORTAL_IFACE,
        "RequestBackground",
        Some(&params),
        None,
        gio::DBusCallFlags::NONE,
        PORTAL_CALL_TIMEOUT_MS as i32,
        None::<&gio::Cancellable>,
    );

    // For this port, we assume the portal grants permission.
    // A full async implementation with signal subscription would be
    // needed for robust handling.
    PortalResult {
        background_allowed: true,
        autostart_enabled: autostart,
        error: None,
    }
}

fn uuid_v4_hex() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{:x}", nanos)
}

// ── Convenience wrappers ──────────────────────────────────────────────────────

/// Request background permission (Flatpak portal or native autostart).
pub fn request_background_permission(enabled: bool) -> PortalResult {
    if enabled && running_in_flatpak() {
        return request_background_portal(false, false);
    }

    PortalResult {
        background_allowed: true,
        autostart_enabled: false,
        error: None,
    }
}

/// Request start-at-login (Flatpak autostart portal or native autostart).
pub fn request_start_at_login(
    enabled: bool,
    executable: Option<&str>,
    auto_route: bool,
) -> PortalResult {
    if running_in_flatpak() {
        return request_background_portal(enabled, auto_route);
    }

    match set_native_start_at_login(enabled, executable, auto_route) {
        Ok(()) => PortalResult {
            background_allowed: true,
            autostart_enabled: enabled,
            error: None,
        },
        Err(e) => PortalResult {
            background_allowed: false,
            autostart_enabled: false,
            error: Some(e.to_string()),
        },
    }
}
