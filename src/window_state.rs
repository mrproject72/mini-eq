//! Window state persistence (size, position).

use gtk4::prelude::*;

pub fn bind_window_state(window: &impl IsA<gtk4::Window>) {
    // TODO: bind window state to GSettings
    let _ = window;
}
