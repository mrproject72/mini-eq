# mini-eq Project Memory

## Project Identity
- **Goal**: Rust rewrite of `bhack/mini-eq` (Python/GTK/PipeWire parametric EQ). Same UI look-and-feel, lower CPU, no GIL.
- **Original upstream**: `https://github.com/bhack/mini-eq` (fetched as `upstream/main`, tags v0.1.0–v0.8.8).
- **Rust repo**: `/home/mrproject/code/mini-eq` (origin: `git@github.com:mrproject72/mini-eq.git`).
- **Tech stack**: Rust 1.98.1, GTK4 0.11.4, libadwaita 0.9.2, pipewire 0.10.1, rustfft 6.4.1, ebur128 0.1.10, clap 4.6.7, tokio 1.53.1.

## Key Decisions
- UI must use native GTK4/Libadwaita crates, no web/interpreted layer.
- PipeWire filter-chain must use native biquad filters (SPA plugin), coefficients at DSP clock rate.
- Virtual sink: `mini_eq_sink` with description `Mini-EQ-Sink`; output client: `Mini EQ Output`.
- Presets stored under `~/.config/mini-eq/`; AutoEq cache under `~/.cache/mini-eq/autoeq/`.
- Original project has no relation to TUD.
- Flatpak is the target distribution model.
- No scope creep: stick to upstream feature set.

## Current State (2026-09-19)
- **Build**: `cargo check --release` compiles with 19 warnings, 0 errors.
- **Tests**: `cargo test --lib` passes 29 tests.
- **Source**: ~6,900 lines across 32 modules.
- **Upstream fetched**: Yes (`upstream/main`).
- **Working directory for this project**: `/home/mrproject/code/mini-eq` (NOT `/home/mrproject/code/mini-eq_rust`).

## Module Status
| Module | Lines | Status |
|--------|-------|--------|
| `core.rs` | 481 | ✅ Constants, FilterType, BiquadCoefficients, EqBand, 9 filter types |
| `autoeq.rs` | 704 | ✅ APO parser, search, API client, 9 tests |
| `ebur128.rs` | 122 | ✅ LUFS wrapper, 4 tests |
| `settings.rs` | 223 | ✅ JSON settings, 2 tests |
| `analyzer.rs` | 593 | ✅ FFT spectrum, smoothing, display gain |
| `band_fader.rs` | 746 | ✅ Full cairo rendering + upstream interaction |
| `window_layout.rs` | 137 | ✅ OverlaySplitView + Clamp + ToastOverlay + breakpoints |
| `window_utility.rs` | 321 | ✅ Preset/system sections, headroom meter, monitor strip |
| `window_headroom.rs` | 204 | ✅ 3-segment meter, Set Safe, CSS classes |
| `window_analyzer.rs` | 154 | ✅ Smoothing/gain/freeze/LUFS/summary |
| `window_graph.rs` | 300 | ✅ 3-layer overlay, click+drag editing |
| `style.rs` | 214 | ✅ GTK CSS loading |
| `appearance.rs` | 86 | ✅ Light/dark/system preference |
| `dbus_control.rs` | 475 | ✅ D-Bus remote control |
| `background.rs` | 303 | ✅ Background mode, start-at-login |
| `instance.rs` | 29 | ✅ Single-instance guard |
| `desktop_integration.rs` | 230 | ✅ Desktop file, autostart, icon |
| `main.rs` | 177 | ✅ clap CLI, headless/background, GUI launch |
| `pipewire_backend.rs` | 420 | ⏳ Compiles but unverified on live PipeWire |
| `routing.rs` | 382 | ⏳ Compiles but unverified on live PipeWire |
| `window_presets.rs` | 157 | ⏳ Basic ListBox; no band serialization |
| `window_autoeq.rs` | 27 | ⏳ Placeholder dialog |
| `window_preferences.rs` | 41 | ⏳ Placeholder dialog |
| `cli.rs` | 1 | ❌ Empty; CLI in `main.rs` |
| `screenshot.rs` | 1 | ❌ Empty stub |
| `diagnostics.rs` | 1 | ❌ Empty stub |
| `release_notes.rs` | 1 | ❌ Empty stub |
| `window_utils.rs` | 5 | ⏳ Minimal helpers |
| `window_state.rs` | 13 | ⏳ Basic binding |
| `window_band_fader.rs` | 93 | ⏳ Partial band editor integration |
| `lib.rs` | 30 | ✅ Module declarations |

## Known Gaps
- Backend unverified on live PipeWire.
- Missing core DSP helpers: `total_response_db`, `estimate_response_peak_db`, `format_frequency`, `stepped_response_frequencies`, `FILTER_TYPE_INDEX_BY_VALUE`, `MODE_ORDER`/`MODE_INDEX_BY_VALUE`.
- Preset lifecycle incomplete.
- AutoEq and preferences are placeholders.
- Screenshot, diagnostics, release notes are empty stubs.
- No Flatpak manifest, GNOME Shell extension, or CI workflow.
- 19 cairo `Result` warnings in `band_fader.rs`.

## Environment
- Rust toolchain: 1.98.1
- GTK4 dev: available via `deps/` directory or system packages
- PipeWire: required for runtime; filter-chain module must be loaded
- Original project reference: `/var/lib/flatpak/app/io.github.bhack.mini-eq/`
- Original config: `~/.config/mini-eq/`
- AutoEq cache: `~/.cache/mini-eq/autoeq/`
