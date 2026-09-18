//! CSS styling for the mini-eq GTK4 application.

use gtk4::CssProvider;
use gtk4::gdk;

const STYLE_CSS: &str = r#"
:root {
    --window-fg-color: rgb(235, 236, 237);
    --window-bg-color: rgb(30, 30, 32);
    --panel-bg-color: rgb(40, 41, 44);
    --panel-border-color: rgb(60, 61, 65);
    --accent-color: rgb(120, 160, 220);
    --accent-color-active: rgb(140, 180, 240);
    --danger-color: rgb(220, 80, 80);
    --success-color: rgb(80, 180, 120);
    --warning-color: rgb(220, 180, 80);
    --text-muted: rgb(150, 151, 155);
    --toolbar-bg-color: rgb(35, 36, 38);
}

.mini-eq-dark {
    --window-fg-color: rgb(235, 236, 237);
    --window-bg-color: rgb(30, 30, 32);
    --panel-bg-color: rgb(40, 41, 44);
    --panel-border-color: rgb(60, 61, 65);
}

.mini-eq-light {
    --window-fg-color: rgb(40, 40, 42);
    --window-bg-color: rgb(240, 241, 243);
    --panel-bg-color: rgb(255, 255, 255);
    --panel-border-color: rgb(200, 201, 205);
}

.panel-card {
    background-color: var(--panel-bg-color);
    border-radius: 12px;
    padding: 12px;
}

.graph-shell-panel {
    background-color: var(--panel-bg-color);
    border-radius: 12px;
    padding: 12px;
}

.quick-view-shell {
    background-color: var(--panel-bg-color);
    border-radius: 12px;
    padding: 12px;
}

.toolbar-row {
    background-color: var(--toolbar-bg-color);
    border-radius: 10px;
    padding: 6px 10px;
}

.heading {
    font-weight: 600;
    color: var(--window-fg-color);
}

.dim-label {
    color: var(--text-muted);
}

.numeric {
    font-variant-numeric: tabular-nums;
}

.fader-title-label {
    font-size: 1.1em;
}

.eq-band-box {
    background-color: var(--panel-bg-color);
    border-radius: 8px;
    padding: 4px;
    border: 1px solid transparent;
}

.eq-band-box-selected {
    border-color: var(--accent-color);
    box-shadow: 0 0 0 1px var(--accent-color);
}

.eq-band-box-muted {
    opacity: 0.5;
}

.band-editor {
    background-color: var(--panel-bg-color);
    border-radius: 10px;
    padding: 10px;
}

.band-editor-selected {
    border-color: var(--accent-color);
}

.band-editor-field {
    background-color: var(--panel-bg-color);
    border-radius: 8px;
    padding: 6px 10px;
}

.utility-section {
    background-color: var(--panel-bg-color);
    border-radius: 10px;
    padding: 10px;
    margin-bottom: 8px;
}

.utility-pane-shell {
    background-color: var(--panel-bg-color);
    border-radius: 12px;
    padding: 8px;
}

.preset-state-chip {
    border-radius: 10px;
    padding: 2px 8px;
    font-size: 0.85em;
}

.preset-state-chip-saved {
    background-color: color-mix(in srgb, var(--success-color) 20%, transparent);
    color: var(--success-color);
}

.preset-state-chip-modified {
    background-color: color-mix(in srgb, var(--warning-color) 20%, transparent);
    color: var(--warning-color);
}

.preset-state-chip-unsaved {
    background-color: color-mix(in srgb, var(--accent-color) 20%, transparent);
    color: var(--accent-color);
}

.system-state-chip {
    border-radius: 10px;
    padding: 2px 8px;
    font-size: 0.85em;
}

.system-state-chip-live {
    background-color: color-mix(in srgb, var(--success-color) 20%, transparent);
    color: var(--success-color);
}

.system-state-chip-bypass {
    background-color: color-mix(in srgb, var(--danger-color) 20%, transparent);
    color: var(--danger-color);
}

.headroom-panel-safe {
    background-color: var(--panel-bg-color);
    border-radius: 8px;
    padding: 8px;
}

.headroom-panel-risk {
    background-color: color-mix(in srgb, var(--danger-color) 15%, transparent);
    border-radius: 8px;
    padding: 8px;
}

.headroom-panel-tight {
    background-color: color-mix(in srgb, var(--warning-color) 15%, transparent);
    border-radius: 8px;
    padding: 8px;
}

.headroom-peak-chip {
    border-radius: 6px;
    padding: 1px 6px;
    font-size: 0.8em;
}

.graph-header-title {
    font-size: 0.95em;
}

.graph-stage {
    background-color: rgb(20, 20, 22);
    border-radius: 8px;
    overflow: hidden;
}

.route-box {
    background-color: var(--toolbar-bg-color);
    border-radius: 8px;
    padding: 4px 8px;
}

.utility-pane-scroller {
    background-color: var(--panel-bg-color);
    border-radius: 12px;
}
"#;

/// Load the mini-eq CSS stylesheet into the GTK default display.
pub fn load_style() {
    let provider = CssProvider::new();
    provider.load_from_data(STYLE_CSS);
    if let Some(display) = gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
