# Bug Tracker

## Open Bugs

- **Monitor settings popover sliders unwired; monitor-enabled state not persisted.** The smoothing / display-gain sliders and `settings.rs::load_monitor_enabled`/`save_monitor_enabled` exist but nothing calls them. (Also: freeze switch missing window-side.)
- **No runtime re-target on default-sink change.** The monitor and filter-chain keep the sink they were started with; switching the system default output does not follow.
- **No `node.dont-move` / foreign-target guards on the EQ sink.** WirePlumber or other tools can re-link around it.
- Dead code: `analyzer.rs::{spectrum_db_values_to_levels, interleaved_f32le_bytes_to_channel_payloads, smooth_power_values}` and several `pipewire_backend.rs`/`routing.rs` accessors have no non-test callers.
- **`EqGraphState` carries three dead fields.** `frequency`, `q`, and `filter_type` are written by `EqGraph::update` on every tick but never read; the response curve and the selected-band marker both derive their values from `bands`/`selected_band` instead. Either drop the fields or use them for the marker overlay.
- **Appearance is applied but never persisted.** `window.rs` calls `AppearanceSettings::load()` and `apply_appearance_preference`, but nothing calls `AppearanceSettings::save()`, and `settings.rs::{load_appearance, save_appearance}` and `appearance.rs::sync_appearance_css_class` have no callers. Upstream persists the `appearance` key via `settings.py`.
- `window_autoeq.rs` and `window_preferences.rs` are placeholder dialogs, not functional.
- `window_state.rs` does not restore window position.
- Preset lifecycle incomplete: no revert/reapply, import/export/delete file monitoring, or output-preset auto-load on device switch.
- No Flatpak manifest, no GNOME Shell extension, no CI workflow present in this repo.

## Fixed Bugs

- **Headroom/Auto-Safe peak estimate was graph-clamped (under-compensated stacked boosts).** `estimate_response_peak_db` sampled through the display-clamped `total_response_db` (±36 dB), so 4× HiShelf @ +20 dB (true peak ~+84 dB) reported only +36 dB and Auto-Safe pinned at the −24 dB floor could never clear the warning. Split into `total_response_db_unclamped` + clamped wrapper; the estimate now uses the unclamped path (upstream `clamp_output=False`). Locked in by `test_estimate_response_peak_db_is_unclamped`. See `2026-09-28-updates.md`.
- **System EQ toggle unwired / GUI had no backend path / output dropdown placeholder.** Resolved in the 2026-09-27 sessions: the window owns `PipeWireBackend`, fader/preamp edits push live `SPA_PARAM_Props` (`bq_raw`), the output dropdown is populated from detected routes, and System EQ routes/unroutes app streams via default metadata. See `2026-09-27-handover.md`.
- **EQ detached on filter-type change.** The native-biquad strategy reloaded the module on type change, recreating the virtual sink with a new node id and orphaning routed streams. Switched to the upstream `bq_raw` strategy (type encoded in coefficients, fixed topology, live pushes only). See `2026-09-27-updates.md`.
- **Mute was conflated with the filter-type "Off" state and lost on save/load.** `EqBand` modelled a single `enabled` flag that upstream treats as two independent things (`filter_type != Off` for "active", `mute` for "muted"). Because `eq_band_to_dict` wrote `"mute": !enabled`, an `Off` band and a muted Bell band serialised identically, and a muted band came back unmuted. `EqBand` now has a real `mute` field matching upstream, `band_is_effective` honours it, and `test_muted_band_roundtrips_through_preset` locks the behaviour in. Legacy presets written by this port that used `enabled` still load (inverted).
- **Band editor was a non-functional stub.** `window_layout.rs::build_band_editor()` built Mute/Solo toggles and Type/Freq/Q/Gain controls with no signal handlers and no link to the selected band. Replaced by `src/window_band_editor.rs`, a view over the selected fader that mirrors upstream `update_selected_band_editor` / `on_selected_band_*`: mute, solo, filter type, frequency, Q, and gain all write through to the fader and re-render it, with an `updating_ui`-style guard so programmatic repopulation does not echo back as user edits.
- **Editor handlers would have panicked on first edit.** The change handlers used `if let Some(index) = *index.borrow()`, which keeps the `Ref` alive for the whole body on edition 2024; the callback then re-entered the same cell via `refresh()` and hit `RefCell already borrowed`. Switched to `let ... else`, which drops the temporary before the callback runs.
- **Preset apply/reset left the editor stale.** Both callbacks now refresh the editor, and reset recomputes `solo_active` instead of carrying the pre-reset value forward.
- **`cargo clippy -- -D warnings` (the CI gate) did not pass.** Three `needless_return` warnings in `band_fader.rs` and an `approx_constant` error (`3.14` in a `window_headroom.rs` test) broke it. Fixed; the full CI command set is green again.

- **Band selection was multi-select and never reached the graph.** Each fader set `selected = true` on click/keypress with no coordination, so multiple bands could render as selected and `EqGraph::set_selected_band` was never called. Selection is now owned by `window.rs`, which clears sibling faders and mirrors the index into the graph.
- **Preset preamp was ignored on load, reset, and change detection.** The apply callback discarded `_preamp`, the reset callback left the old preamp, and the state-signature callback hard-coded `0.0` so preamp-only edits never marked the preset dirty. All three now use the headroom panel's preamp value.
- **Inspector toggle button was unwired.** The `ToggleButton` in the header now drives `AdwOverlaySplitView::set_collapsed`.
- Fader drag direction was inverted: dragging down increased gain instead of decreasing. The drag formula also had an incorrect scale factor that limited a full drag to ~0.44 dB instead of the full ±20 dB range.
- Fader interaction did not match upstream Python: missing drag threshold, modifier-aware fine/coarse steps, snap-to-0.1 dB rounding, and full keyboard bindings.
- Fader rendering did not match upstream Python: missing track gradient, zero-line fill, tick marks, knob shadow/highlight, state badges, frequency/Q labels, and gain pill.

## Known Issues

- **Fader bottom slightly clipped at some window sizes (minor).** The fader's
  bottom edge (Q value "1.50" + the box's bottom border) can still be a few
  px short of fully visible at certain window heights. The band_scrolled's
  `min_content_height` was raised to match the fader height per breakpoint
  (164 compact / 208 wide, see `window.rs` breakpoints), which improved it
  a lot, but a small residual clip remains at some sizes. The faders are
  fully usable; only the last border row is tight. Root cause is the tight
  vertical budget (graph + faders + editor + header vs MIN_WINDOW_HEIGHT).
  Possible follow-ups: shave the graph height, reduce fader CONTENT_H, or
  let the fader area scroll.
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
