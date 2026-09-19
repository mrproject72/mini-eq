//! Utility pane for mini-eq.
//!
//! Provides utility functions like import/export, reset, and about dialog.

use gtk4::prelude::*;
use adw::prelude::*;

/// Utility pane widget.
pub struct UtilityPane {
    /// The main container widget.
    pub container: gtk4::Box,
    /// Import button.
    import_button: gtk4::Button,
    /// Export button.
    export_button: gtk4::Button,
    /// Reset button.
    reset_button: gtk4::Button,
    /// About button.
    about_button: gtk4::Button,
}

impl Default for UtilityPane {
    fn default() -> Self {
        Self::new()
    }
}

impl UtilityPane {
    /// Create a new utility pane.
    pub fn new() -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
        container.set_margin_start(12);
        container.set_margin_end(12);
        container.set_margin_top(12);
        container.set_margin_bottom(12);

        // Header
        let header = gtk4::Label::new(Some("Utilities"));
        header.set_css_classes(&["heading", "title-3"]);
        header.set_halign(gtk4::Align::Start);
        container.append(&header);

        // Import button
        let import_button = gtk4::Button::with_label("Import APO Preset");
        import_button.set_icon_name(Some("document-open-symbolic"));
        import_button.set_halign(gtk4::Align::Start);
        container.append(&import_button);

        // Export button
        let export_button = gtk4::Button::with_label("Export Current Preset");
        export_button.set_icon_name(Some("document-save-symbolic"));
        export_button.set_halign(gtk4::Align::Start);
        container.append(&export_button);

        // Separator
        let separator = gtk4::Separator::new(gtk4::Orientation::Horizontal);
        container.append(&separator);

        // Reset button
        let reset_button = gtk4::Button::with_label("Reset All Bands");
        reset_button.set_icon_name(Some("edit-clear-all-symbolic"));
        reset_button.set_css_classes(&["destructive-action"]);
        reset_button.set_halign(gtk4::Align::Start);
        container.append(&reset_button);

        // Separator
        let separator2 = gtk4::Separator::new(gtk4::Orientation::Horizontal);
        container.append(&separator2);

        // About button
        let about_button = gtk4::Button::with_label("About Mini EQ");
        about_button.set_icon_name(Some("help-about-symbolic"));
        about_button.set_halign(gtk4::Align::Start);
        container.append(&about_button);

        Self {
            container,
            import_button,
            export_button,
            reset_button,
            about_button,
        }
    }

    /// Connect import button click handler.
    pub fn connect_import<F>(&self, callback: F)
    where
        F: Fn() + 'static,
    {
        self.import_button.connect_clicked(move |_| {
            callback();
        });
    }

    /// Connect export button click handler.
    pub fn connect_export<F>(&self, callback: F)
    where
        F: Fn() + 'static,
    {
        self.export_button.connect_clicked(move |_| {
            callback();
        });
    }

    /// Connect reset button click handler.
    pub fn connect_reset<F>(&self, callback: F)
    where
        F: Fn() + 'static,
    {
        self.reset_button.connect_clicked(move |_| {
            callback();
        });
    }

    /// Connect about button click handler.
    pub fn connect_about<F>(&self, callback: F)
    where
        F: Fn() + 'static,
    {
        self.about_button.connect_clicked(move |_| {
            callback();
        });
    }

    /// Show the about dialog.
    pub fn show_about_dialog(&self, parent: &gtk4::Window) {
        let about = adw::AboutWindow::builder()
            .application_name("Mini EQ")
            .application_icon("audio-card")
            .version(env!("CARGO_PKG_VERSION"))
            .website("https://github.com/bhack/mini-eq")
            .issue_url("https://github.com/bhack/mini-eq/issues")
            .license_type(gtk4::License::Gpl30)
            .copyright("© 2024-2025 Mini EQ Contributors")
            .developers(["Mini EQ Team"])
            .transient_for(parent)
            .modal(true)
            .build();

        about.present();
    }

    /// Get the underlying container widget.
    pub fn get_container(&self) -> &gtk4::Box {
        &self.container
    }

    /// Enable or disable all buttons.
    pub fn set_buttons_sensitive(&self, sensitive: bool) {
        self.import_button.set_sensitive(sensitive);
        self.export_button.set_sensitive(sensitive);
        self.reset_button.set_sensitive(sensitive);
        self.about_button.set_sensitive(sensitive);
    }
}
