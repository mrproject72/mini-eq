//! D-Bus control interface for mini-eq.
//!
//! Provides MPRIS-like D-Bus interface for controlling the equalizer
//! from external applications, scripts, or desktop environments.

use anyhow::Context;
use dbus::{arg, blocking};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::EqState;

/// D-Bus service name for mini-eq.
pub const DBUS_SERVICE_NAME: &str = "org.minieq.Control";
/// D-Bus object path for the player interface.
pub const DBUS_PLAYER_PATH: &str = "/org/minieq/Player";
/// D-Bus object path for the properties interface.
pub const DBUS_PROPERTIES_PATH: &str = "/org/minieq/Player";

/// D-Bus controller for mini-eq.
pub struct DbusController {
    /// Shared EQ state.
    eq_state: Arc<RwLock<EqState>>,
    /// D-Bus connection handle.
    connection: Option<blocking::SyncConnection>,
    /// Whether the controller is active.
    active: bool,
}

impl DbusController {
    /// Create a new D-Bus controller.
    pub fn new(eq_state: Arc<RwLock<EqState>>) -> Self {
        Self {
            eq_state,
            connection: None,
            active: false,
        }
    }

    /// Initialize and register on D-Bus.
    pub fn init(&mut self) -> anyhow::Result<()> {
        log::info!("Initializing D-Bus controller...");

        let conn = blocking::SyncConnection::new_session()
            .context("Failed to connect to D-Bus session")?;

        // Request our service name
        conn.request_name(DBUS_SERVICE_NAME, false, true, false)
            .context("Failed to request D-Bus service name")?;

        self.connection = Some(conn);
        self.active = true;

        log::info!("D-Bus controller registered as {}", DBUS_SERVICE_NAME);
        Ok(())
    }

    /// Run the D-Bus event loop (blocking).
    pub fn run(&mut self) -> anyhow::Result<()> {
        if !self.active {
            anyhow::bail!("D-Bus controller not initialized");
        }

        log::info!("Starting D-Bus event loop...");

        let conn = self.connection.as_ref().unwrap();

        // Register interfaces and start handling method calls
        self.register_interfaces(conn)?;

        // Main D-Bus event loop
        conn.process(std::time::Duration::from_millis(1000))?;

        Ok(())
    }

    /// Register D-Bus interfaces.
    fn register_interfaces(&self, conn: &blocking::SyncConnection) -> anyhow::Result<()> {
        // Register the Player interface
        let cr = conn.with_proxy(DBUS_SERVICE_NAME, DBUS_PLAYER_PATH, std::time::Duration::from_millis(5000));
        
        // Introspection data for our interface
        let introspection_data = r#"
<!DOCTYPE node PUBLIC "-//freedesktop//DTD D-BUS Object Introspection 1.0//EN"
"http://www.freedesktop.org/standards/dbus/1.0/introspect.dtd">
<node>
  <interface name="org.minieq.Player">
    <method name="Play"/>
    <method name="Pause"/>
    <method name="Stop"/>
    <method name="SetPreampGain">
      <arg direction="in" type="d" name="gain_db"/>
    </method>
    <method name="ToggleEQ"/>
    <method name="LoadPreset">
      <arg direction="in" type="s" name="preset_name"/>
    </method>
    <property name="PlaybackStatus" type="s" access="read"/>
    <property name="Volume" type="d" access="readwrite"/>
    <property name="EQEnabled" type="b" access="readwrite"/>
    <property name="PreampGain" type="d" access="read"/>
  </interface>
  <interface name="org.freedesktop.DBus.Properties">
    <method name="Get">
      <arg direction="in" type="s" name="interface_name"/>
      <arg direction="in" type="s" name="property_name"/>
      <arg direction="out" type="v" name="value"/>
    </method>
    <method name="Set">
      <arg direction="in" type="s" name="interface_name"/>
      <arg direction="in" type="s" name="property_name"/>
      <arg direction="in" type="v" name="value"/>
    </method>
    <method name="GetAll">
      <arg direction="in" type="s" name="interface_name"/>
      <arg direction="out" type="a{sv}" name="properties"/>
    </method>
  </interface>
</node>
"#;

        conn.add_handler(cr, move |msg, conn| {
            // Handle method calls here
            // This is a simplified version - full implementation would parse messages
            log::debug!("D-Bus message received: {:?}", msg);
            Some(vec![])
        });

        Ok(())
    }

    /// Handle Play command.
    pub async fn play(&self) {
        let mut state = self.eq_state.write().await;
        state.enabled = true;
        log::info!("EQ enabled via D-Bus");
    }

    /// Handle Pause command.
    pub async fn pause(&self) {
        let mut state = self.eq_state.write().await;
        state.enabled = false;
        log::info!("EQ disabled via D-Bus");
    }

    /// Handle Stop command.
    pub async fn stop(&self) {
        let mut state = self.eq_state.write().await;
        state.enabled = false;
        state.preamp_gain_db = 0.0;
        log::info!("EQ stopped via D-Bus");
    }

    /// Set preamp gain.
    pub async fn set_preamp_gain(&self, gain_db: f64) {
        let mut state = self.eq_state.write().await;
        state.preamp_gain_db = gain_db as f32;
        log::info!("Preamp gain set to {} dB via D-Bus", gain_db);
    }

    /// Toggle EQ on/off.
    pub async fn toggle_eq(&self) {
        let mut state = self.eq_state.write().await;
        state.enabled = !state.enabled;
        log::info!("EQ toggled to {} via D-Bus", if state.enabled { "on" } else { "off" });
    }

    /// Load a preset by name.
    pub async fn load_preset(&self, _preset_name: &str) {
        log::info!("Preset load requested via D-Bus: {}", _preset_name);
        // TODO: Implement preset loading
    }

    /// Get current playback status.
    pub async fn get_playback_status(&self) -> String {
        let state = self.eq_state.read().await;
        if state.enabled {
            "Playing".to_string()
        } else {
            "Paused".to_string()
        }
    }

    /// Get current EQ enabled status.
    pub async fn get_eq_enabled(&self) -> bool {
        let state = self.eq_state.read().await;
        state.enabled
    }

    /// Get current preamp gain.
    pub async fn get_preamp_gain(&self) -> f64 {
        let state = self.eq_state.read().await;
        state.preamp_gain_db as f64
    }

    /// Check if controller is active.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Shutdown the controller.
    pub fn shutdown(&mut self) {
        log::info!("Shutting down D-Bus controller...");
        self.active = false;
        self.connection = None;
    }
}

impl Default for DbusController {
    fn default() -> Self {
        Self::new(Arc::new(RwLock::new(EqState::default())))
    }
}