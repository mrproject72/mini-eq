# Implementation TODO — mini-eq Rust Port

## Blocker: Backend Parity (Phase 1)

- [ ] Run `cargo run --release -- --check-deps` on a PipeWire desktop to verify dependencies.
- [ ] Test `pipewire_backend.rs` `create_virtual_sink()` on live PipeWire session; confirm node appears in `pw-cli`.
- [ ] Test `create_filter_chain()`; confirm `libpipewire-module-filter-chain` is loaded and node is created.
- [ ] Test `configure_biquad_filters()` + `update_band_coefficients()`; confirm biquad params are applied to running graph.
- [ ] Test `link_nodes()`; confirm virtual sink → filter chain → output node links are created.
- [ ] Verify `routing.rs` `detect_routes()` finds actual output sinks via PipeWire metadata.
- [ ] Verify `auto_route_to_sink()` routes playback streams to virtual sink.
- [ ] Consolidate `pipewire_routes.rs` + `pipewire_stream_router.rs` into `routing.rs` (or delete duplicates).
- [ ] Add error handling: if PipeWire connection fails, show toast and allow retry.

## Core Math Completion (Phase 7)

- [ ] Add `FILTER_TYPE_INDEX_BY_VALUE` and reverse lookup in `core.rs`.
- [ ] Add `SELECTABLE_FILTER_TYPES` with all 12 upstream types (currently missing Resonance, Ladder-pass, Ladder-rej from selectable list).
- [ ] Add `MODE_ORDER` and `MODE_INDEX_BY_VALUE` in `core.rs` (upstream has 2 modes: "Live PipeWire", "Auto Eq").
- [ ] Add `format_frequency()` helper in `core.rs`.
- [ ] Add `stepped_response_frequencies()` and `log_response_frequencies()` in `core.rs`.
- [ ] Add `total_response_db_at_frequencies()` in `core.rs` for graph curve drawing.
- [ ] Add `estimate_response_peak_db()` in `core.rs` for headroom panel.
- [ ] Verify biquad coefficient formulas match upstream Python exactly for all 12 filter types.

## Preset Lifecycle (Phase 6)

- [ ] Replace `window_presets.rs` ListBox with full upstream preset panel: Current Curve label + state chip, Load Preset dropdown, Scope row, Auto Preset switch, Fallback row, Save + More popover.
- [ ] Implement preset serialization: save band data + preamp to JSON (`preset_path_for_name`).
- [ ] Implement preset loading: `load_library_preset` with apply-to-engine + UI sync.
- [ ] Implement save-as with replace dialog for duplicate names.
- [ ] Implement revert/reapply: track `curve_revert_baseline` payload + signature.
- [ ] Implement reset-to-neutral: restore `default_state_signature`.
- [ ] Implement delete: remove file + refresh list.
- [ ] Implement file monitoring: `Gio.FileMonitor` on preset directory.
- [ ] Implement output-preset auto-linking by route key (`output-presets.json`).
- [ ] Implement fallback preset: load when output has no auto preset link.
- [ ] Implement import/export: APO text import, JSON export.

## AutoEq Dialog (Phase 6)

- [ ] Replace `window_autoeq.rs` placeholder with full dialog: search entry + refresh + results list + curve preview + import button.
- [ ] Add async search with debounce (240ms).
- [ ] Add curve preview drawing area showing imported frequency response.
- [ ] Wire import flow to `controller.import_apo_preset()`.

## Preferences Dialog (Phase 6)

- [ ] Replace `window_preferences.rs` placeholder with `AdwPreferencesDialog`.
- [ ] Add Background group: Keep Running in Background toggle, Start at Login toggle, Enable System-wide EQ at Login toggle.
- [ ] Wire to `background.rs` persistence functions.

## Screenshot Module (Phase 8)

- [ ] Implement `screenshot.rs` using valid GTK4 0.11.4 API (`Snapshot::free_to_paintable` + `Texture::from_paintable` or cairo surface fallback).
- [ ] Add menu action to capture graph or window.

## CLI Module (Phase 8)

- [ ] Populate `cli.rs` with `parse_args()` matching upstream `cli.py` (`--install-desktop`, `--check-deps`, `--auto-route`, `--output-sink`, `--import-apo`, `--headless`, `--background`, `--duration`).
- [ ] Remove clap derive from `main.rs` and use `cli.rs` instead for parity.
- [ ] Remove empty `diagnostics.rs`, `release_notes.rs`, `screenshot.rs` stubs or implement them.

## Window State (Phase 3)

- [ ] Restore window position from `Gio.Settings` in `window_state.rs`.
- [ ] Add monitor geometry fallback for initial window size (`initial_window_default_size` from upstream).

## Flatpak / Packaging (Phase 8)

- [ ] Create Flatpak manifest `io.github.bhack.mini-eq.yaml` matching upstream.
- [ ] Add Flatpak build/test workflow.
- [ ] Verify appstream metadata, desktop file, icon installation.

## GNOME Shell Extension (Phase 8)

- [ ] Port `extensions/gnome-shell/mini-eq@bhack.github.io/extension.js` to match upstream.
- [ ] Add metadata.json, SVG icon, README.

## CI / Runtime Validation (Phase 8)

- [ ] Add GitHub Actions workflow: `cargo check --release`, `cargo test --lib`, `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo build --release`.
- [ ] Add runtime test: verify EQ is audible system-wide on PipeWire desktop.
- [ ] Add screenshot comparison test against upstream reference image.
- [ ] Fix 19 cairo `Result` warnings in `band_fader.rs`.
