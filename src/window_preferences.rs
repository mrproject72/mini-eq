//! Preferences dialog.

use adw::prelude::*;

use crate::background::{request_background_permission, request_start_at_login, save_background_mode, save_start_active_at_login, save_start_at_login};
use crate::settings::{load_background_mode, load_start_active_at_login, load_start_at_login};

/// Preferences dialog.
pub struct PreferencesDialog {
    pub dialog: adw::PreferencesDialog,
}

impl PreferencesDialog {
    pub fn new(parent: &impl IsA<gtk4::Widget>) -> Self {
        let dialog = adw::PreferencesDialog::new();
        dialog.set_title("Preferences");

        let page = adw::PreferencesPage::new();
        page.set_icon_name(Some("preferences-system-symbolic"));
        let group = adw::PreferencesGroup::new();
        group.set_title("Background");

        let background_row = adw::SwitchRow::new();
        background_row.set_title("Keep Running in Background");
        background_row.set_subtitle("Closing the window keeps Mini EQ active for this session.");
        background_row.set_active(load_background_mode());
        group.add(&background_row);

        let start_row = adw::SwitchRow::new();
        start_row.set_title("Start at Login");
        start_row.set_subtitle("Start Mini EQ hidden when you sign in.");
        start_row.set_active(load_start_at_login());
        start_row.set_sensitive(background_row.is_active());
        group.add(&start_row);

        let start_active_row = adw::SwitchRow::new();
        start_active_row.set_title("Enable System-wide EQ at Login");
        start_active_row.set_subtitle("Route system audio through Mini EQ when it starts at login.");
        start_active_row.set_active(load_start_active_at_login());
        start_active_row.set_sensitive(background_row.is_active() && start_row.is_active());
        group.add(&start_active_row);

        page.add(&group);
        dialog.add(&page);

        background_row.connect_active_notify({
            let start_row = start_row.clone();
            let start_active_row = start_active_row.clone();
            move |row| {
                let desired = row.is_active();
                let _ = save_background_mode(desired);
                if !desired {
                    let _ = save_start_at_login(false);
                    start_row.set_active(false);
                }
                start_row.set_sensitive(desired);
                start_active_row.set_sensitive(desired && start_row.is_active());
                let _ = request_background_permission(desired);
            }
        });

        start_row.connect_active_notify({
            let background_row = background_row.clone();
            let start_active_row = start_active_row.clone();
            move |row| {
                let desired = row.is_active();
                if desired && !background_row.is_active() {
                    row.set_active(false);
                    return;
                }
                let _ = save_start_at_login(desired);
                start_active_row.set_sensitive(background_row.is_active() && desired);
                let _ = request_start_at_login(desired, None, false);
            }
        });

        start_active_row.connect_active_notify(move |row| {
            let _ = save_start_active_at_login(row.is_active());
        });

        dialog.present(Some(parent));
        Self { dialog }
    }

    pub fn show(&self) {
        self.dialog.present(None::<&gtk4::Window>);
    }
}
