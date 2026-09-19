# Mini EQ Architecture

## Overview

Mini EQ is a system-wide parametric equalizer for PipeWire desktops, implemented in Rust with a GTK4/Libadwaita UI. This document describes the architecture of the Rust rewrite.

## System Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     User Interface Layer                     │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────────────┐ │
│  │ Main     │ │ Analyzer │ │ Presets  │ │ AutoEq/Search  │ │
│  │ Window   │ │ Panel    │ │ Panel    │ │ Panel          │ │
│  └──────────┘ └──────────┘ └──────────┘ └────────────────┘ │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────────────┐ │
│  │ Graph    │ │ Headroom │ │ Prefere- │ │ Utility        │ │
│  │ View     │ │ Display  │ │ nces     │ │ Functions      │ │
│  └──────────┘ └──────────┘ └──────────┘ └────────────────┘ │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                   Application Services Layer                 │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────────────┐ │
│  │ Settings │ │ Preset   │ │ Instance │ │ Appearance     │ │
│  │ Manager  │ │ Manager  │ │ Guard    │ │ Controller     │ │
│  └──────────┘ └──────────┘ └──────────┘ └────────────────┘ │
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────────────┐ │
│  │ D-Bus    │ │ Backgro- │ │ Desktop  │ │ Diagnostics    │ │
│  │ Control  │ │ und Mode │ │ Integrat │ │                │ │
│  └──────────┘ └──────────┘ └──────────┘ └────────────────┘ │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                    Audio Processing Layer                    │
│  ┌──────────────────┐         ┌──────────────────────────┐  │
│  │  PipeWire        │         │  DSP Core                │  │
│  │  Backend         │◄───────►│  - Biquad Filters        │  │
│  │  - Virtual Sink  │         │  - Coefficient Calc      │  │
│  │  - Filter Chain  │         │  - Band Management       │  │
│  │  - Routing       │         │                          │  │
│  └──────────────────┘         └──────────────────────────┘  │
│  ┌──────────────────┐         ┌──────────────────────────┐  │
│  │  Spectrum        │         │  Loudness Meter          │  │
│  │  Analyzer        │         │  (EBU R128)              │  │
│  │  - FFT           │         │                          │  │
│  │  - Visualization │         │                          │  │
│  └──────────────────┘         └──────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                      System Integration                      │
│  ┌──────────────────┐         ┌──────────────────────────┐  │
│  │  PipeWire Server │         │  D-Bus Session Bus       │  │
│  │  - Audio Router  │         │  - MPRIS Control         │  │
│  │  - Filter Module │         │  - Remote Commands       │  │
│  └──────────────────┘         └──────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Module Organization

### Core DSP (`core.rs`)

**Purpose**: Mathematical foundation for parametric EQ processing

**Key Components**:
- `EqBand`: Represents a single EQ band with frequency, gain, Q, filter type
- `BiquadCoefficients`: Direct form II biquad filter coefficients (b0, b1, b2, a0, a1, a2)
- `FilterType`: Enum for filter types (Bell, Hi-Shelf, Lo-Shelf, etc.)
- `band_biquad_coefficients()`: Calculates filter coefficients using RBJ Audio EQ Cookbook formulas
- Constants: `SAMPLE_RATE`, `MAX_BANDS`, `EQ_GAIN_MIN_DB`, etc.

**Design Decisions**:
- Uses Direct Form II biquad structure for numerical stability
- Coefficients calculated at initialization, not per-sample
- Wet/dry mixing support for parallel processing
- Pure functions for testability

### PipeWire Integration

#### `pipewire_backend.rs`

**Purpose**: Manages PipeWire connection and audio graph

**Responsibilities**:
- Initialize PipeWire main loop
- Create virtual sink node (`mini_eq_sink`)
- Create filter chain node with biquad plugin
- Manage node lifecycle
- Handle registry events

**Key Types**:
- `PipeWireBackend`: Main backend struct with mainloop, context, core
- `OutputRoute`: Represents an audio output destination

#### `pipewire_routes.rs`

**Purpose**: Discovers and tracks available audio output routes

**Responsibilities**:
- Scan PipeWire registry for sink nodes
- Monitor route changes via registry listener
- Maintain list of available outputs
- Provide route selection API

#### `pipewire_stream_router.rs`

**Purpose**: Routes audio streams through the EQ filter chain

**Responsibilities**:
- Link default output to virtual sink
- Route individual streams to EQ
- Handle stream connect/disconnect events
- Maintain routing state

#### `filter_chain.rs`

**Purpose**: Configures and manages the PipeWire filter-chain module

**Responsibilities**:
- Create filter-chain node with biquad configuration
- Configure individual band filters
- Update coefficients in real-time
- Enable/disable processing

**Integration**: Uses PipeWire's `libpipewire-module-filter-chain` with SPA builtin biquad plugin

### UI Framework (`window*.rs`)

#### Main Window (`window.rs`)

**Structure**:
```rust
pub struct MiniEqWindow {
    pub window: adw::ApplicationWindow,
}
```

**Components**:
- Header bar with title and menu
- Content area with band faders
- Status bar with loudness display

#### Band Fader Widget (`band_fader.rs`)

**Custom Widget**: Cairo-rendered vertical slider

**Features**:
- Drag gesture for gain adjustment
- Scroll wheel support
- Keyboard navigation
- Visual feedback (selected, active, muted states)
- Frequency label display

**Rendering**: Custom draw callback renders:
- Slider track
- Handle position based on gain value
- Band frequency label
- Active/inactive visual states

#### Analyzer Panel (`window_analyzer.rs`)

**Displays**:
- Real-time spectrum (30 logarithmic bands)
- LUFS loudness values (momentary, short-term, integrated)
- Configurable display gain

**Update Rate**: ~30 FPS (33ms interval)

#### Preset Panel (`window_presets.rs`)

**Functionality**:
- List saved presets
- Create new preset
- Delete preset
- Import/export JSON files
- Auto-eq preset links by output device

**Storage**: `~/.config/mini-eq/presets/*.json`

#### AutoEq Panel (`window_autoeq.rs`)

**Features**:
- Search headphone database
- Download parametric EQ presets from AutoEq.app API
- Import Equalizer APO text files
- Apply preset to current bands

### Analyzer DSP (`analyzer.rs`)

**FFT Pipeline**:
1. Capture audio from PipeWire monitor source
2. Apply window function (Hann)
3. Compute FFT using `rustfft`
4. Convert to power spectrum
5. Map to logarithmic frequency bands
6. Smooth over time (exponential moving average)
7. Convert to dB scale

**Loudness Metering**:
- Uses `ebur128` crate for EBU R128 compliance
- Updates every 250ms
- Tracks momentary (400ms), short-term (3s), integrated

### AutoEq Support (`autoeq.rs`)

#### APO File Parser

**Parses Equalizer APO format**:
```
Filter 1: ON PK Fc 1000 Hz Gain 6.0 dB Q 1.0
Filter 2: ON HS Fc 10000 Hz Gain -3.0 dB Q 0.707
Preamp: -2.0 dB
```

**Regex-based parsing** mirrors Python implementation for compatibility

#### AutoEq API Client

**Endpoints**:
- Search: `https://api.autoeq.app/v2/headphones`
- Download: `https://api.autoeq.app/v2/target/{id}/parametric-eq`

**Response Format**: JSON with filter parameters

### Settings & Persistence

#### `settings.rs` (TO IMPLEMENT)

**Planned Structure**:
```rust
pub struct AppSettings {
    pub window_geometry: WindowState,
    pub last_preset: Option<String>,
    pub appearance: AppearancePreference,
    pub audio_device: Option<String>,
    pub auto_route: bool,
    pub start_in_background: bool,
}
```

**Storage**: `~/.config/mini-eq/settings.json`

#### `window_state.rs`

**Tracks**:
- Window size and position
- Maximized state
- Last active panel

### System Integration

#### Background Mode (`background.rs`) - TO IMPLEMENT

**Purpose**: Keep EQ active after closing GUI

**Implementation Strategy**:
- Spawn background process on `--background` flag
- Use PipeWire main loop in background thread
- Communicate via D-Bus or Unix socket
- Clean shutdown on session end

#### D-Bus Control (`dbus_control.rs`) - TO IMPLEMENT

**Interface**: `org.mpris.MediaPlayer2.MiniEq`

**Methods**:
- `SetPreset(name: String)`
- `ToggleEQ(enabled: bool)`
- `GetStatus() -> (bool, String)`
- `SetGain(band: u32, gain_db: f64)`

**Signals**:
- `PresetChanged(name: String)`
- `BandChanged(index: u32, gain_db: f64)`

#### Desktop Integration (`desktop_integration.rs`) - TO IMPLEMENT

**Components**:
- `.desktop` file installation
- Autostart entry
- GNOME Shell extension IPC
- Application metadata (AppData XML)

## Data Flow

### Initialization Sequence

```
1. main() parses CLI arguments
2. If --background: start daemon mode
3. Else: create adw::Application
4. Connect activate signal → MiniEqWindow::new()
5. Window creates UI components
6. PipeWireBackend::new() initializes audio
7. Load settings from disk
8. Restore last preset
9. Enter main loop
```

### Band Adjustment Flow

```
User drags fader
    ↓
BandFader updates gain_db value
    ↓
Signal emitted to window controller
    ↓
Window updates EqBand in state
    ↓
PipeWireBackend::update_band_coefficients() called
    ↓
Biquad coefficients recalculated
    ↓
Filter chain node properties updated
    ↓
PipeWire applies new DSP
    ↓
Audio processed with new EQ curve
```

### Preset Save Flow

```
User clicks "Save Preset"
    ↓
Dialog collects preset name
    ↓
PresetPayload created from current state
    ↓
Serialized to JSON
    ↓
Written to ~/.config/mini-eq/presets/{name}.json
    ↓
UI updates preset list
```

## Threading Model

```
Main Thread (GTK)
├── UI event loop
├── Widget rendering
└── User input handling

Audio Thread (PipeWire)
├── Main loop iteration
├── Node callbacks
└── DSP parameter updates

Analyzer Thread (Optional)
├── FFT computation
├── Loudness metering
└── Spectrum smoothing

Background Thread (if --background)
├── D-Bus message handling
├── Preset auto-loading
└── Cleanup on session end
```

**Thread Safety**:
- `Arc<Mutex<T>>` for shared state
- GTK signals marshaled to main thread via `glib::idle_add()`
- PipeWire callbacks run in main loop context

## Error Handling Strategy

### Result Propagation

```rust
pub fn create_virtual_sink(&mut self) -> Result<(), Error> {
    // ...
    Ok(())
}
```

### Error Types

- **PipeWire errors**: `pipewire::Error`
- **IO errors**: `std::io::Error` wrapped with `anyhow`
- **Parse errors**: Custom error types with `thiserror`
- **UI errors**: Logged and displayed to user

### Recovery Strategies

- **PipeWire disconnect**: Attempt reconnection, notify user
- **Invalid preset**: Show error dialog, keep current state
- **DSP overflow**: Clamp values to safe ranges

## Performance Considerations

### DSP Efficiency

- Coefficients calculated once per band change, not per sample
- PipeWire handles actual sample processing in optimized C code
- Rust code only manages configuration

### UI Responsiveness

- Heavy operations (FFT, file I/O) on background threads
- UI updates throttled to 60 FPS maximum
- Lazy loading for preset lists

### Memory Usage Targets

- Binary size: < 10 MB
- Runtime memory: < 100 MB
- Startup time: < 1 second

## Security Considerations

### File Access

- Presets stored in user directory only
- No system-wide configuration without privileges
- APO file parser validates input before applying

### Network Access

- AutoEq API uses HTTPS
- No data sent to external servers except search queries
- Cache downloaded presets locally

### Privilege Separation

- No root access required
- Runs as regular user
- PipeWire handles audio isolation

## Testing Strategy

### Unit Tests

- Core DSP coefficient calculations
- APO file parsing
- Preset serialization
- Frequency-to-band mapping

### Integration Tests

- Mock PipeWire backend
- Simulated user interactions
- End-to-end preset workflow

### Manual Testing

- Real PipeWire setup required
- Audio quality verification
- CPU usage measurement
- Compatibility testing across distros

## Future Extensions

### Potential Features

1. **Convolution reverb**: Add impulse response support
2. **Multi-channel**: Surround sound processing
3. **Per-application profiles**: Automatic preset switching
4. **Cloud sync**: Preset backup across devices
5. **Plugin host**: VST3/LV2 support

### Architectural Improvements

1. **Plugin architecture**: Third-party DSP modules
2. **Scripting API**: Lua/Python automation
3. **Remote control**: Mobile app integration
4. **Real-time collaboration**: Shared sessions

---

## Glossary

- **Biquad**: Second-order linear filter with 6 coefficients
- **PipeWire**: Multimedia server for Linux
- **SPA**: Simple Plugin API (PipeWire's plugin system)
- **LUFS**: Loudness Units Full Scale (EBU R128 standard)
- **Q**: Filter quality factor (center frequency / bandwidth)
- **AutoEq**: Database of headphone correction profiles

## References

- [Robert Bristow-Johnson Audio EQ Cookbook](https://www.musicdsp.org/en/latest/Filters/197-rbj-audio-eq-cookbook.html)
- [PipeWire Documentation](https://docs.pipewire.org/)
- [GTK4 Documentation](https://docs.gtk.org/gtk4/)
- [Libadwaita Documentation](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/)
- [EBU R128 Standard](https://tech.ebu.ch/docs/r/r128.pdf)
