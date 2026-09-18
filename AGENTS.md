# AGENTS.md — mini-eq (Rust Rewrite)

## Required reading (in this order)

0. **This document** — static starting point, reference for onboarding not a journal. Always update `docs/{date}-updates.md` and `docs/{date}-handover.md`.
1. **`README.md`** — project overview, features, install instructions
2. **`docs/BUGS.md`** — outstanding and fixed bugs (check before implementing)
3. **`docs/PLAN/`** — implementation plans and architecture decisions
4. **`docs/{date}-updates.md`** — latest implementation changes
5. **`docs/{date}-handover.md`** — dev session handover for context
6. **`/home/mrproject/code/mini-eq/docs/BUGS.md`** — current open and fixed bugs

Important considerations:

### This is a Rust rewrite of the Python mini-eq project
- Original: https://github.com/bhack/mini-eq (Python/GTK/PipeWire)
- Goal: Same functionality, significantly lower CPU usage, no GIL
- Target: PipeWire system-wide parametric EQ using Rust + GTK4 + Libadwaita
- Use the `pipewire` crate (v0.10.1) for PipeWire integration
- Use `gtk4` crate (v0.11.4) + `libadwaita` crate (v0.9.2) for UI
- No Python, no GIL, no garbage collection

### Always update the status for next session under docs/{date}-updates.md

### Never leave artifacts when you complete a task
- Clean your mess: /tmp/ files, temporary scripts, anything else
- Keep track of what you create and where

### finish the task that you started and then continue with the backlog
- If asked to look at another issue, evaluate the priority and add it to docs/BUGS.md as backlog or docs/TODO.md as task to do

---

## Project Overview

Mini EQ is a compact system-wide parametric equalizer for PipeWire desktops. This is a Rust rewrite of the original Python project (https://github.com/bhack/mini-eq) to eliminate the 30% CPU usage caused by Python's GIL and real-time audio processing overhead.

### Original Python project (reference)
- ~30 Python modules, ~2000+ lines
- GTK4/Libadwaita UI via PyGObject
- pipewire-gobject for PipeWire routing
- PipeWire filter-chain with biquad DSP filters
- NumPy FFT analyzer, libebur128 loudness metering
- AutoEq preset support, APO preset import
- GNOME Shell extension, D-Bus control
- Background mode with auto-route

### Rust rewrite target
- Same features, lower CPU, no GIL
- Direct PipeWire filter-chain API via `pipewire` crate
- GTK4 + Libadwaita via `gtk4` + `libadwaita` crates
- Compiled native binary, no interpreter overhead
- Real-time DSP at native speed

## Tech Stack

| Layer | Technology | Notes |
|-------|------------|-------|
| Language | Rust 1.98.1 | Compiled native binary |
| UI | GTK4 + Libadwaita | `gtk4` v0.11.4, `libadwaita` v0.9.2 |
| Audio | PipeWire | `pipewire` v0.10.1, `pipewire-sys` v0.10.1 |
| DSP | PipeWire filter-chain | Builtin biquad filters |
| FFT Analyzer | `rustfft` or `numpy` bindings | Spectrum visualization |
| Loudness | `libebur128` FFI | LUFS metering |
| Build | Cargo | Standard Rust build system |
| Packaging | Flatpak | Same distribution model as original |
| License | GPL-3.0-or-later | Same as original |

## Environment

| Item | Value |
|------|-------|
| Rust | 1.98.1 (cargo 1.98.1) |
| GTK4 | System packages required |
| PipeWire | System PipeWire with filter-chain module |
| Original project | /var/lib/flatpak/app/io.github.bhack.mini-eq/ |
| Original config | ~/.config/mini-eq/ |
| Original presets | ~/.config/mini-eq/ |
| AutoEq cache | ~/.cache/mini-eq/autoeq/ |

## Architecture

### Core modules (from original Python, to be reimplemented in Rust)

```
src/
├── main.rs                    # Entry point
├── core.rs                    # EQ constants, biquad coefficients, band config
├── pipewire_backend.rs        # PipeWire connection, filter-chain management
├── pipewire_routes.rs         # Output route detection and tracking
├── pipewire_stream_router.rs  # Stream routing logic
├── filter_chain.rs            # PipeWire filter-chain DSP configuration
├── band_fader.rs              # Individual EQ band control
├── analyzer.rs                # FFT spectrum analysis
├── ebur128.rs                 # LUFS loudness metering
├── autoeq.rs                  # AutoEq preset search and import
├── settings.rs                # User settings persistence
├── dbus_control.rs            # D-Bus remote control
├── background.rs              # Background mode daemon
├── cli.rs                     # Command-line interface
├── instance.rs                # Application instance management
├── appearance.rs              # GTK theme/appearance
├── glib_utils.rs              # GLib utilities (if needed)
├── desktop_integration.rs     # Desktop file, autostart
├── diagnostics.rs             # Debug/diagnostic tools
├── release_notes.rs           # Version changelog
├── routing.rs                 # PipeWire routing logic
├── screenshot.rs              # Screenshot/capture feature
├── window.rs                  # Main GTK window
├── window_analyzer.rs         # Analyzer panel
├── window_autoeq.rs           # AutoEq panel
├── window_graph.rs            # Frequency graph
├── window_headroom.rs         # Headroom display
├── window_layout.rs           # Window layout management
├── window_preferences.rs      # Preferences dialog
├── window_presets.rs          # Preset management panel
├── window_state.rs            # Window state persistence
├── window_utility.rs          # Utility window functions
├── window_utils.rs            # Window helper functions
├── style.css                  # GTK CSS styling
└── assets/                    # Application assets
```

### PipeWire filter-chain architecture
- Creates virtual sink: `mini_eq_sink` → `alsa_output.pci-0000_04_00.6.analog-stereo`
- Uses `libpipewire-module-filter-chain` for DSP
- Biquad filters calculate coefficients at DSP clock rate
- EQ processing not pinned to 48 kHz
- Native PipeWire biquad filters via SPA plugin

### Key constants (from original)
- `MAX_BANDS`: 32
- `DEFAULT_ACTIVE_BANDS`: 10
- `SAMPLE_RATE`: 48000.0
- `EQ_FREQUENCY_MIN_HZ`: 20.0
- `EQ_FREQUENCY_MAX_HZ`: 20000.0
- `EQ_GAIN_MIN_DB`: -20.0
- `EQ_GAIN_MAX_DB`: 20.0
- `EQ_Q_MIN`: 0.18248
- `EQ_Q_MAX`: 6.0
- `EQ_PREAMP_MIN_DB`: -24.0
- `EQ_PREAMP_MAX_DB`: 6.0
- `FILTER_TYPES`: Off, Bell, Hi-pass, Hi-shelf, Lo-pass, Lo-shelf, Notch, Resonance, Allpass, Bandpass, Ladder-pass, Ladder-rej

## Launch & Development

### Create new project
```bash
cd ~/code/mini-eq
cargo init --name mini-eq
```

### Build
```bash
cargo build --release
```

### Run
```bash
cargo run --release
```

### Check dependencies
```bash
# Verify PipeWire, GTK4, Libadwaita are available
# See docs/PLAN/ for dependency checklist
```

### Test
```bash
cargo test
```

## Common Commands

```bash
# Build release
cargo build --release

# Run with debug output
cargo run -- --verbose

# Run background mode
cargo run -- --background --auto-route

# Import APO preset
cargo run -- --import-apo path/to/ParametricEQ.txt

# Check dependencies
cargo run -- --check-deps

# Headless mode (no GUI)
cargo run -- --headless --background --duration 30
```

## Key Business Logic

**Critical rules:**
- PipeWire filter-chain must use native biquad filters (SPA plugin)
- Coefficients calculated at DSP clock rate, not pinned to 48 kHz
- Virtual sink name: `mini_eq_sink` with description `Mini-EQ-Sink`
- Output client name: `Mini EQ Output`
- Background mode keeps EQ active after window closes
- Auto-route routes all output streams to virtual sink
- Presets stored under `~/.config/mini-eq/`
- AutoEq data cached under `~/.cache/mini-eq/autoeq/`

**Important:** The Rust rewrite must produce identical audio processing results to the Python original. Biquad filter coefficients must match exactly.

## Gotchas

- PipeWire filter-chain module must be loaded: `libpipewire-module-filter-chain`
- SPA builtin filter graph plugin required: `libspa-filter-graph-plugin-builtin.so`
- `pipewire-gobject` Python binding is NOT available in Rust — use `pipewire` crate directly
- The original uses `pipewire-gobject` for app-facing PipeWire routing — the Rust `pipewire` crate provides equivalent functionality
- NumPy FFT analyzer will need `rustfft` or similar crate
- libebur128 will need FFI binding or Rust crate
- GTK4/Libadwaita must be installed on the system
- Flatpak packaging will need to be recreated for Rust

## External References

- Original project: https://github.com/bhack/mini-eq
- Flathub: https://flathub.org/apps/io.github.bhack.mini-eq
- AutoEq: https://autoeq.app/
- PipeWire: https://pipewire.org/
- Libadwaita: https://gitlab.gnome.org/GNOME/libadwaita

## Status Tracking

- **Phase 1**: ✅ Complete — Project skeleton, Cargo.toml, basic structure
- **Phase 2**: ✅ Complete — Core module (biquad coefficients, band config, constants, 4 unit tests passing)
- **Phase 3**: ✅ Complete — PipeWire backend (filter-chain, virtual sink, routing, stream router)
- **Phase 4**: ✅ Complete — GTK4/Libadwaita UI skeleton (window, band fader, presets, utility panes, appearance, style, instance guard)
- **Phase 5**: ✅ Complete — Analyzer (FFT spectrum via `rustfft`, LUFS loudness via `ebur128`), 21 unit tests passing
- **Phase 6**: ⏳ Pending — AutoEq/APO preset support
- **Phase 7**: ⏳ Pending — Background mode, D-Bus control, desktop integration
- **Phase 8**: ⏳ Pending — Testing, Flatpak packaging, performance validation

### Build instructions

```bash
# Install dev dependencies (if not already installed)
cd /tmp/kilo && apt download libgtk-4-dev libadwaita-1-dev libgraphene-1.0-dev libpipewire-0.3-dev libspa-0.2-dev libdbus-1-dev
for deb in *.deb; do dpkg-deb -x "$deb" ~/code/mini-eq/deps/; done

# Only env var needed: PKG_CONFIG_PATH. The linker search path for deps/*.so
# is added automatically by build.rs (conditional on deps/ existing, so CI is
# unaffected).
export PKG_CONFIG_PATH="$HOME/code/mini-eq/deps/usr/lib/x86_64-linux-gnu/pkgconfig:$PKG_CONFIG_PATH"

# Build / test / run
cargo build --release
cargo test --lib
cargo run --release -- --background --auto-route
```

## CI Notes

- GitHub Actions workflow installs `libdbus-1-dev` (required by `libdbus-sys v0.2.7`)
- CI runs on `ubuntu-24.04` with `stable` Rust toolchain
- Jobs: `cargo check --release`, `cargo test --lib`, `cargo clippy -- -D warnings`, `cargo fmt --check`, `cargo build --release`
- CI is currently **passing** (commit `002db80`)
- UI directive: use native Rust UI (GTK4/Libadwaita crates), no web/interpreted layer — maximum performance

> **Full project status:** see `docs/{date}-updates.md` and `docs/{date}-handover.md`.
> This document is a static starting point, reference for onboarding not a journal or dynamic status update.
