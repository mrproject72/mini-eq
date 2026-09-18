# Mini EQ — Rust Rewrite

Compact PipeWire system-wide parametric equalizer for Linux desktops.

This is a Rust rewrite of [mini-eq](https://github.com/bhack/mini-eq) (Python/GTK) to eliminate the 30% CPU usage caused by Python's GIL and real-time audio processing overhead.

## Features

- System-wide parametric EQ for PipeWire desktop playback
- GTK4/Libadwaita interface with a compact 10-band fader workflow (up to 32 bands)
- PipeWire routing and default-output tracking
- PipeWire filter-chain DSP using builtin biquad filters
- Optional spectrum analyzer and LUFS loudness readout
- Auto preset links that follow the detected PipeWire port
- Background mode keeps the EQ active after closing the window
- Optional GNOME Shell extension for quick panel access
- Search and import headphone correction presets from AutoEq
- Import Equalizer APO-style text presets

## Why Rust?

The original Python implementation uses ~30 modules and the Python GIL for real-time audio processing, causing ~30% CPU usage. This Rust rewrite eliminates the GIL, compiles to native code, and processes audio at native speed with minimal overhead.

## Tech Stack

- **Rust 1.98.1** — compiled native binary, no GIL
- **GTK4 + Libadwaita** — modern GNOME UI
- **PipeWire 0.10** — audio routing and filter-chain DSP
- **SPA biquad filters** — native DSP processing

## Install (when available)

```bash
# Build from source
cargo build --release
./target/release/mini-eq

# Or install via Flatpak (when packaged)
flatpak install flathub io.github.bhack.mini-eq
```

## Development

See [AGENTS.md](AGENTS.md) for project structure, architecture, and development workflow.

## License

GPL-3.0-or-later
