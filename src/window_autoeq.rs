//! AutoEq import dialog for mini-eq.
//!
//! Provides UI for searching and importing AutoEq presets.

use gtk4::prelude::*;
use adw::prelude::*;

/// AutoEq import dialog.
pub struct AutoEqDialog {
    /// The main dialog widget.
    pub dialog: adw::Window,
    /// Search entry field.
    search_entry: gtk4::Entry,
    /// Results list box.
    results_box: gtk4::ListBox,
    /// Progress spinner.
    spinner: gtk4::Spinner,
    /// Status label.
    status_label: gtk4::Label,
}

impl Default for AutoEqDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl AutoEqDialog {
    /// Create a new AutoEq dialog.
    pub fn new() -> Self {
        let dialog = adw::Window::builder()
            .title("Import from AutoEq")
            .modal(true)
            .default_width(600)
            .default_height(500)
            .build();

        // Main container
        let main_box = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
        main_box.set_margin_start(18);
        main_box.set_margin_end(18);
        main_box.set_margin_top(18);
        main_box.set_margin_bottom(18);

        // Header
        let header = gtk4::Label::new(Some("Search AutoEq Database"));
        header.set_css_classes(&["heading", "title-2"]);
        header.set_halign(gtk4::Align::Start);
        main_box.append(&header);

        // Search entry
        let search_entry = gtk4::Entry::new();
        search_entry.set_placeholder_text(Some("Search for headphones or IEMs..."));
        search_entry.set_icon_from_name(Some("edit-find-symbolic"), gtk4::EntryIconPosition::Start);
        main_box.append(&search_entry);

        // Status label
        let status_label = gtk4::Label::new(Some("Enter a search term above"));
        status_label.set_css_classes(&["dim-label"]);
        status_label.set_halign(gtk4::Align::Start);
        main_box.append(&status_label);

        // Progress spinner
        let spinner = gtk4::Spinner::new();
        spinner.set_visible(false);
        main_box.append(&spinner);

        // Results list
        let scrolled = gtk4::ScrolledWindow::new();
        scrolled.set_vexpand(true);
        scrolled.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

        let results_box = gtk4::ListBox::new();
        results_box.set_css_classes(&["boxed-list"]);
        scrolled.set_child(Some(&results_box));
        main_box.append(&scrolled);

        // Button box
        let button_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
        button_box.set_halign(gtk4::Align::End);

        let cancel_button = gtk4::Button::with_label("Cancel");
        cancel_button.connect_clicked(clone!(@weak dialog => move |_| {
            dialog.close();
        }));

        let import_button = gtk4::Button::with_label("Import");
        import_button.set_css_classes(&["suggested-action"]);
        import_button.set_sensitive(false);

        button_box.append(&cancel_button);
        button_box.append(&import_button);
        main_box.append(&button_box);

        dialog.set_child(Some(&main_box));

        // Connect search signal
        search_entry.connect_activate(clone!(@weak status_label, @weak results_box => move |entry| {
            let query = entry.text();
            if !query.is_empty() {
                status_label.set_text(&format!("Searching for '{}'...", query));
                // TODO: Implement actual AutoEq search
                log::info!("AutoEq search: {}", query);
            }
        }));

        Self {
            dialog,
            search_entry,
            results_box,
            spinner,
            status_label,
        }
    }

    /// Show the dialog.
    pub fn show(&self, parent: Option<&gtk4::Window>) {
        if let Some(p) = parent {
            self.dialog.set_transient_for(Some(p));
        }
        self.dialog.present();
    }

    /// Hide the dialog.
    pub fn hide(&self) {
        self.dialog.close();
    }

    /// Set the search query.
    pub fn set_search_query(&self, query: &str) {
        self.search_entry.set_text(query);
    }

    /// Get the current search query.
    pub fn get_search_query(&self) -> String {
        self.search_entry.text().to_string()
    }

    /// Show progress indicator.
    pub fn show_progress(&self) {
        self.spinner.start();
        self.spinner.set_visible(true);
        self.status_label.set_text("Searching...");
    }

    /// Hide progress indicator.
    pub fn hide_progress(&self) {
        self.spinner.stop();
        self.spinner.set_visible(false);
    }

    /// Update status message.
    pub fn set_status(&self, message: &str) {
        self.status_label.set_text(message);
    }

    /// Clear search results.
    pub fn clear_results(&self) {
        while let Some(child) = self.results_box.first_child() {
            self.results_box.remove(&child);
        }
    }

    /// Add a result item to the list.
    pub fn add_result(&self, name: &str, brand: &str) {
        let row = adw::ActionRow::builder()
            .title(name)
            .subtitle(brand)
            .activatable(true)
            .build();

        self.results_box.append(&row);
    }

    /// Get the underlying dialog widget.
    pub fn get_dialog(&self) -> &adw::Window {
        &self.dialog
    }
}
