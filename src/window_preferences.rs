//! Preferences dialog for mini-eq.
//!
//! Provides UI for configuring application settings.

use gtk4::prelude::*;
use adw::prelude::*;

/// Preferences dialog.
pub struct PreferencesDialog {
    /// The main preferences window.
    pub window: adw::PreferencesWindow,
    /// Audio device selection combo.
    device_combo: gtk4::DropDown,
    /// Sample rate entry.
    sample_rate_entry: gtk4::Entry,
    /// Buffer size entry.
    buffer_size_entry: gtk4::Entry,
    /// Theme selection combo.
    theme_combo: gtk4::DropDown,
    /// Auto-start toggle.
    auto_start_toggle: gtk4::Switch,
    /// Show analyzer toggle.
    show_analyzer_toggle: gtk4::Switch,
}

impl Default for PreferencesDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl PreferencesDialog {
    /// Create a new preferences dialog.
    pub fn new() -> Self {
        let window = adw::PreferencesWindow::builder()
            .title("Preferences")
            .modal(true)
            .default_width(500)
            .default_height(450)
            .build();

        // Audio page
        let audio_page = adw::PreferencesPage::builder()
            .name("audio")
            .title("Audio")
            .icon_name("audio-card-symbolic")
            .build();

        // Audio group
        let audio_group = adw::PreferencesGroup::builder()
            .title("Device Settings")
            .description("Configure audio backend parameters")
            .build();

        // Device selection
        let device_row = adw::ComboRow::builder()
            .title("Output Device")
            .subtitle("Select the virtual sink device")
            .build();

        let device_store = gtk4::StringList::new(&[
            "Default (Auto)",
            "PipeWire Virtual Sink",
            "Custom Device..."
        ]);
        device_row.set_model(Some(&device_store));
        device_row.set_selected(0);

        // Sample rate
        let sample_rate_row = adw::ActionRow::builder()
            .title("Sample Rate")
            .subtitle("Hz (e.g., 48000)")
            .build();

        let sample_rate_entry = gtk4::Entry::new();
        sample_rate_entry.set_text("48000");
        sample_rate_entry.set_width_chars(8);
        sample_rate_row.add_suffix(&sample_rate_entry);
        sample_rate_row.set_activatable_widget(Some(&sample_rate_entry));

        // Buffer size
        let buffer_size_row = adw::ActionRow::builder()
            .title("Buffer Size")
            .subtitle("Samples per block")
            .build();

        let buffer_size_entry = gtk4::Entry::new();
        buffer_size_entry.set_text("1024");
        buffer_size_entry.set_width_chars(8);
        buffer_size_row.add_suffix(&buffer_size_entry);
        buffer_size_row.set_activatable_widget(Some(&buffer_size_entry));

        audio_group.add(&device_row);
        audio_group.add(&sample_rate_row);
        audio_group.add(&buffer_size_row);
        audio_page.add(&audio_group);

        // Appearance page
        let appearance_page = adw::PreferencesPage::builder()
            .name("appearance")
            .title("Appearance")
            .icon_name("preferences-desktop-theme-symbolic")
            .build();

        // Appearance group
        let appearance_group = adw::PreferencesGroup::builder()
            .title("Theme & Display")
            .build();

        // Theme selection
        let theme_row = adw::ComboRow::builder()
            .title("Theme")
            .subtitle("Application color scheme")
            .build();

        let theme_store = gtk4::StringList::new(&[
            "System",
            "Light",
            "Dark"
        ]);
        theme_row.set_model(Some(&theme_store));
        theme_row.set_selected(0);

        // Show analyzer
        let analyzer_row = adw::ActionRow::builder()
            .title("Show Analyzer")
            .subtitle("Display spectrum analyzer panel")
            .build();

        let show_analyzer_toggle = gtk4::Switch::new();
        show_analyzer_toggle.set_active(true);
        analyzer_row.add_suffix(&show_analyzer_toggle);
        analyzer_row.set_activatable_widget(Some(&show_analyzer_toggle));

        appearance_group.add(&theme_row);
        appearance_group.add(&analyzer_row);
        appearance_page.add(&appearance_group);

        // Behavior page
        let behavior_page = adw::PreferencesPage::builder()
            .name("behavior")
            .title("Behavior")
            .icon_name("preferences-system-symbolic")
            .build();

        // Behavior group
        let behavior_group = adw::PreferencesGroup::builder()
            .title("Startup")
            .build();

        // Auto-start
        let auto_start_row = adw::ActionRow::builder()
            .title("Start on Login")
            .subtitle("Launch mini-eq when you log in")
            .build();

        let auto_start_toggle = gtk4::Switch::new();
        auto_start_toggle.set_active(false);
        auto_start_row.add_suffix(&auto_start_toggle);
        auto_start_row.set_activatable_widget(Some(&auto_start_toggle));

        behavior_group.add(&auto_start_row);
        behavior_page.add(&behavior_group);

        // Add pages to window
        window.add(&audio_page);
        window.add(&appearance_page);
        window.add(&behavior_page);

        Self {
            window,
            device_combo: gtk4::DropDown::new(Some(device_store), None),
            sample_rate_entry,
            buffer_size_entry,
            theme_combo: gtk4::DropDown::new(Some(theme_store), None),
            auto_start_toggle,
            show_analyzer_toggle,
        }
    }

    /// Show the preferences dialog.
    pub fn show(&self, parent: Option<&gtk4::Window>) {
        if let Some(p) = parent {
            self.window.set_transient_for(Some(p));
        }
        self.window.present();
    }

    /// Get selected device index.
    pub fn get_device_index(&self) -> u32 {
        self.device_combo.selected()
    }

    /// Set selected device index.
    pub fn set_device_index(&self, index: u32) {
        self.device_combo.set_selected(index);
    }

    /// Get sample rate value.
    pub fn get_sample_rate(&self) -> u32 {
        self.sample_rate_entry.text().parse().unwrap_or(48000)
    }

    /// Set sample rate value.
    pub fn set_sample_rate(&self, rate: u32) {
        self.sample_rate_entry.set_text(&rate.to_string());
    }

    /// Get buffer size value.
    pub fn get_buffer_size(&self) -> u32 {
        self.buffer_size_entry.text().parse().unwrap_or(1024)
    }

    /// Set buffer size value.
    pub fn set_buffer_size(&self, size: u32) {
        self.buffer_size_entry.set_text(&size.to_string());
    }

    /// Get theme selection.
    pub fn get_theme(&self) -> String {
        match self.theme_combo.selected() {
            0 => "system".to_string(),
            1 => "light".to_string(),
            2 => "dark".to_string(),
            _ => "system".to_string(),
        }
    }

    /// Set theme selection.
    pub fn set_theme(&self, theme: &str) {
        let index = match theme {
            "light" => 1,
            "dark" => 2,
            _ => 0,
        };
        self.theme_combo.set_selected(index);
    }

    /// Check if auto-start is enabled.
    pub fn is_auto_start_enabled(&self) -> bool {
        self.auto_start_toggle.is_active()
    }

    /// Set auto-start enabled state.
    pub fn set_auto_start_enabled(&self, enabled: bool) {
        self.auto_start_toggle.set_active(enabled);
    }

    /// Check if analyzer display is enabled.
    pub fn is_analyzer_enabled(&self) -> bool {
        self.show_analyzer_toggle.is_active()
    }

    /// Set analyzer display enabled state.
    pub fn set_analyzer_enabled(&self, enabled: bool) {
        self.show_analyzer_toggle.set_active(enabled);
    }

    /// Get the underlying window widget.
    pub fn get_window(&self) -> &adw::PreferencesWindow {
        &self.window
    }
}
