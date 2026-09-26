# Bug Tracker

## Open Bugs

- **Band editor is a non-functional stub.** `window_layout.rs::build_band_editor()` builds Mute/Solo toggles and Type/Freq/Q/Gain controls with no signal handlers and no link to the selected band. Upstream `window_layout.py` drives these from the selected band; the Rust controls are inert.
- **`route_switch` ("System EQ") is unwired.** `src/window.rs` creates the switch but connects no handler; toggling it does nothing. Wiring it requires the `RoutingEngine`/`PipeWireBackend` to be shared into the window (the GUI currently owns no backend handle).
- **GUI has no path to the audio backend.** `WindowBandFader::new` passes a no-op `gain_changed_callback` and `window.rs` never constructs `PipeWireBackend`/`RoutingEngine`; the graph re-reads fader state each tick so the UI looks correct, but band edits are never applied to PipeWire.
- **Output dropdown is a placeholder.** The header `DropDown` is populated with two hard-coded strings ("System Output", "Virtual Sink") and is not connected to `detect_routes()`.
- `pipewire_backend.rs` and `routing.rs` are **unverified on live PipeWire**. `remove_link` only mutates the in-memory list and never destroys the PipeWire link; `route_stream`/`unroute_stream` are no-ops.
- Dead code: `analyzer.rs::{spectrum_db_values_to_levels, interleaved_f32le_bytes_to_channel_payloads, smooth_power_values}`, `window_graph.rs::set_selected_band` (now wired), `window_presets.rs::set_bands_and_preamp`, and several `pipewire_backend.rs`/`routing.rs` accessors have no non-test callers.
- `window_autoeq.rs` and `window_preferences.rs` are placeholder dialogs, not functional.
- `window_state.rs` does not restore window position and does not fall back to monitor geometry for initial window size.
- No Flatpak manifest, no GNOME Shell extension, no CI workflow present in this repo.

## Fixed Bugs

- **Band selection was multi-select and never reached the graph.** Each fader set `selected = true` on click/keypress with no coordination, so multiple bands could render as selected and `EqGraph::set_selected_band` was never called. Selection is now owned by `window.rs`, which clears sibling faders and mirrors the index into the graph.
- **Preset preamp was ignored on load, reset, and change detection.** The apply callback discarded `_preamp`, the reset callback left the old preamp, and the state-signature callback hard-coded `0.0` so preamp-only edits never marked the preset dirty. All three now use the headroom panel's preamp value.
- **Inspector toggle button was unwired.** The `ToggleButton` in the header now drives `AdwOverlaySplitView::set_collapsed`.
- Fader drag direction was inverted: dragging down increased gain instead of decreasing. The drag formula also had an incorrect scale factor that limited a full drag to ~0.44 dB instead of the full ±20 dB range.
- Fader interaction did not match upstream Python: missing drag threshold, modifier-aware fine/coarse steps, snap-to-0.1 dB rounding, and full keyboard bindings.
- Fader rendering did not match upstream Python: missing track gradient, zero-line fill, tick marks, knob shadow/highlight, state badges, frequency/Q labels, and gain pill.

## Known Issues

- GTK4/Libadwaita dev packages not permanently installed (using `deps/` directory).
- PipeWire filter-chain module availability not verified for Rust `pipewire` crate.
- No CI/CD pipeline running yet.

## Bug Report Template

When reporting bugs, please include:
1. mini-eq version or commit hash
2. Steps to reproduce
3. Expected vs actual behavior
4. Relevant log output (`--verbose` flag)
5. System information (PipeWire version, GTK version, OS)

See [CONTRIBUTING.md](../CONTRIBUTING.md) for details.
