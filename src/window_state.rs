//! Window state persistence (size, position).

use gtk4::prelude::*;

pub fn initial_window_default_size() -> (i32, i32) {
    let display = gtk4::gdk::Display::default();
    if let Some(display) = display {
        let monitors = display.monitors();
        if monitors.n_items() > 0 {
            if let Some(obj) = monitors.item(0) {
                if let Ok(monitor) = obj.downcast::<gtk4::gdk::Monitor>() {
                    let geometry = monitor.geometry();
                    let width = (geometry.width() as f64 * 0.85).round() as i32;
                    let height = (geometry.height() as f64 * 0.85).round() as i32;
                    let width = width.max(980);
                    let height = height.max(600);
                    return (width.min(1360), height.min(720));
                }
            }
        }
    }
    (1360, 720)
}

pub fn bind_window_state(window: &impl IsA<gtk4::Window>) {
    let window = window.as_ref();
    let settings = crate::appearance::AppearanceSettings::load();
    if let Some(width) = settings.window_width {
        window.set_default_width(width);
    }
    if let Some(height) = settings.window_height {
        window.set_default_height(height);
    }

    let _ = window.connect_notify(Some("default-width"), move |win, _| {
        let width = win.default_width();
        let mut settings = crate::appearance::AppearanceSettings::load();
        settings.window_width = Some(width);
        settings.save();
    });

    let _ = window.connect_notify(Some("default-height"), move |win, _| {
        let height = win.default_height();
        let mut settings = crate::appearance::AppearanceSettings::load();
        settings.window_height = Some(height);
        settings.save();
    });
}
