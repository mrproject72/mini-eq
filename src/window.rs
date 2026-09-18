//! Main application window for mini-eq.

use adw::prelude::*;

use crate::appearance::{AppearancePreference, apply_appearance_preference};
use crate::style;

/// Main application window.
pub struct MiniEqWindow {
    pub window: adw::ApplicationWindow,
}

impl MiniEqWindow {
    pub fn new(app: &adw::Application) -> Self {
        let window = adw::ApplicationWindow::new(app);
        window.set_default_size(1360, 720);
        window.set_title(Some("Mini EQ"));

        // Load CSS styling
        style::load_style();

        // Apply appearance
        let settings = crate::appearance::AppearanceSettings::load();
        apply_appearance_preference(AppearancePreference::parse(&settings.preference));

        // Build UI
        let header_bar = adw::HeaderBar::new();
        let window_title = adw::WindowTitle::new("Mini EQ", "");
        header_bar.set_title_widget(Some(&window_title));

        // Menu button
        let menu_button = gtk4::MenuButton::new();
        menu_button.set_icon_name("open-menu-symbolic");
        header_bar.pack_start(&menu_button);

        // Content: a simple placeholder for now
        let content_box = gtk4::Box::new(gtk4::Orientation::Vertical, 10);
        content_box.set_margin_start(12);
        content_box.set_margin_end(12);
        content_box.set_margin_top(12);
        content_box.set_margin_bottom(12);

        let label = gtk4::Label::new(Some("Mini EQ — Phase 4 UI"));
        label.set_css_classes(&["heading"]);
        content_box.append(&label);

        let subtitle = gtk4::Label::new(Some("GTK4 + Libadwaita UI is loading..."));
        subtitle.set_css_classes(&["dim-label"]);
        content_box.append(&subtitle);

        let toolbar_view = adw::ToolbarView::new();
        toolbar_view.add_top_bar(&header_bar);
        toolbar_view.set_content(Some(&content_box));

        window.set_content(Some(&toolbar_view));

        MiniEqWindow { window }
    }

    pub fn present(&self) {
        self.window.present();
    }
}
