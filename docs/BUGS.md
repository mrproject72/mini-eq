# Bug Tracker

## Open Bugs

- `cargo check --release` emits 19 warnings: unused `Result` from cairo `stroke()`/`fill()` calls in `src/band_fader.rs` (lines 683, 694, 698, 709, 717, 720, 725). These are not functional bugs but should be fixed with `let _ = ctx.stroke();` / `let _ = ctx.fill();`.
- `src/cli.rs` is empty but declared as a module in `src/lib.rs`. The CLI is implemented via clap derive in `src/main.rs`, so `cli.rs` is dead code.
- `src/screenshot.rs`, `src/diagnostics.rs`, `src/release_notes.rs` are empty stubs declared in `src/lib.rs` but contain no functionality.
- `pipewire_backend.rs` and `routing.rs` are **unverified on live PipeWire**. Node creation, filter-chain module loading, biquad parameter application, and stream routing are best-effort stubs that may not work on a real PipeWire session.
- `core.rs` is missing `total_response_db`, `estimate_response_peak_db`, `format_frequency`, `stepped_response_frequencies`, `FILTER_TYPE_INDEX_BY_VALUE`, `MODE_ORDER`/`MODE_INDEX_BY_VALUE`. This means the graph cannot draw a correct frequency response curve.
- `window_presets.rs` does not serialize band data when saving presets (writes `{}` JSON). No revert/reapply, import/export, delete, file monitoring, or output-preset linking is implemented.
- `window_autoeq.rs` and `window_preferences.rs` are placeholder dialogs, not functional.
- `window_state.rs` does not restore window position and does not fall back to monitor geometry for initial window size.
- No Flatpak manifest, no GNOME Shell extension, no CI workflow present in this repo.

## Fixed Bugs

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
