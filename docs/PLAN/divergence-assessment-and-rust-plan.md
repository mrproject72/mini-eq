# mini-eq Rust Conversion Plan

## Phase 0: Source-Linked Divergence Assessment

This document maps verified gaps between the original Python mini-eq (`bhack/mini-eq`, upstream commit `9ff550e678cd69e867b1b336d83b76b4028371d9`, version `0.8.8`) and the Rust rewrite at `8ee53da`. The original visual contract is preserved in screenshots under `screenshot/`:
- Original: `Screenshot From 2026-09-18 21-27-35.png` (1358x959)
- Rust: `Screenshot From 2026-09-18 21-29-43.png` (1360x724)


## Verified Layout / Shell Divergence

**Upstream** (`window_layout.py:96-303`): `Adw.ToolbarView` > `Adw.Clamp(max=1480)` > `Gtk.Box` > `Adw.OverlaySplitView` with pinned sidebar (width fraction 0.24, min 268, max 320). Two breakpoints:
- 1320sp: collapses sidebar, pins to END, shows utility_pane_button
- 1080sp: further compacts toolbar, adjusts graph/analyzer/fader heights
- F9 toggles sidebar visibility
- Toast overlay above clamp

**Rust** (`window.rs:25-143`, `window_layout.rs:21-124`): `Adw.ToolbarView` > `Adw.Clamp(max=1480, tightening_threshold=1320)` > `Adw.ToastOverlay` > `adw::ApplicationWindow`. Content is `Adw.OverlaySplitView` with pinned sidebar (width fraction 0.24, min 268, max 320). Two breakpoints via `adw::Breakpoint` at 1320sp (collapses sidebar, pins to END) and 1080sp (compacts faders to 164px, graph to Compact mode, analyzer to 80px). F9 toggles sidebar visibility via `EventControllerKey`. Default window 1360x720, minimum via scroller constraints.

**Acceptance criterion**: Rust UI must support adaptive breakpoints at 1320sp and 1080sp matching upstream, with F9 toggling the sidebar and toolbar layout matching upstream (output dropdown + route switch + inspector toggle on left, toolbar buttons on right).

## Verified Toolbar Divergence

**Upstream** (`window_layout.py:129-219`): 
- "EQ output" dropdown (300px default, 240px compact) with sink labels
- Main Menu button (import AutoEq, import APO, appearance light/dark, preferences, about)
- "System-wide EQ" route switch + label on the right
- Inspector Pane toggle button (hidden by default, visible at narrow breakpoint, F9 binds it)

**Rust** (`window.rs:42-75`): Header bar with "Output" dropdown (`gtk4::DropDown` with System Output/Virtual Sink), Main Menu button (`gtk4::MenuButton`), "System-wide EQ" route switch + label on the right, Inspector Pane toggle button. Toolbar layout matches upstream.

**Acceptance criterion**: Rust toolbar must include output sink dropdown (mirroring upstream), route switch, and inspector pane toggle binding (F9 or UI button).

## Verified Graph Divergence

**Upstream** (`window_layout.py:342-440`, `window_graph.py`): 
- Three DrawingArea overlay on graph shell: background (grid, dB markers, freq markers, gradient), analyzer (monitor overlay with margins 58/62/26/34), frequency response curve
- Default content width 900, height 196; compact 760x156; roomy 280
- Click+drag editing: click selects band, shift+drag adjusts Q, drag adjusts frequency+gain (shelf filters), drag end applies to engine
- Selected band vertical line + halo marker

**Rust** (`window_graph.rs:60-161`): Three `DrawingArea` overlays on `gtk4::Overlay` inside a `gtk4::Box` container: background (grid, dB markers, freq markers, gradient), analyzer (monitor overlay), frequency response curve. `GraphMode` enum with Default (196px), Compact (156px), Roomy (280px) heights, switched via `set_mode`. Click+drag editing: click selects band, drag adjusts frequency+gain (shelf filters), shift+drag adjusts Q. Selected band vertical line + halo marker drawn in `draw_response`.

**Acceptance criterion**: Rust graph must overlay 3 DrawingAreas (background + analyzer + response curve), support click+drag editing with shift-Q/freq+gain, and display selected band visuals matching upstream.

## Verified Fader Divergence

**Upstream** (`window_layout.py:442-521`): 
- 32 faders in a Gtk.Grid, each box 76px wide, default height 208 (compact 164, roomy 300)
- Scroller min height 200/150/290 responsive to breakpoint
- Horizontal centering spacers (left/right expandable)
- Visible band count controls which faders show

**Rust** (`window_layout.rs:21-48`, `window_band_fader.rs:13-172`, `band_fader.rs:12-284`): Horizontal `Box` inside `ScrolledWindow` with all visible bands. Each band is an `EqBandFader` with `CONTENT_H` default 208px, adjusted to 164px at compact breakpoint via `set_height`. Scroller min-height adapts (200px default, 150px compact). Horizontal centering via `set_hexpand`. Visible band count gates which faders show via `visible_bands.min(MAX_BANDS)`. Per-band controls moved to shared selected-band editor.

**Acceptance criterion**: Rust must support responsive fader heights (164/208/300 depending on breakpoint), scroller min-height adaptation, and visible band count gating matching upstream.

## Verified Selected Band Editor Divergence

**Upstream** (`window_layout.py:522-560`): `Adw.WrapBox` "band-editor" below faders with selected band label, Mute/Solo toggles, Type dropdown, Frequency/Q/Gain spinbuttons. Inline in compact mode with natural line length 820/720.

**Rust** (`window_layout.rs:50-102`): `gtk4::Box` "band-editor" below faders with selected band label, Mute/Solo toggles, Type dropdown, Frequency/Q/Gain spinbuttons. Matches upstream layout.

**Acceptance criterion**: Rust must include a shared selected-band editor (WrapBox or equivalent) below the fader grid with Mute/Solo/Type/Frequency/Q/Gain controls, matching upstream layout.

## Verified Utility Pane Divergence

**Upstream** (`window_utility.py:18-357`): 
- Preset section: Current Curve label + state chip, Load Preset dropdown, Scope row, Auto Preset switch, Fallback row (visible when unsaved changes), Save + More popover (Save As/Revert/Reset to Neutral/Import/Export/Delete)
- System section: Signal state chip, A/B bypass switch, Headroom panel (meter 260x14, 3 segments safe/tight/risk, detail label, Preamp scale -24..6, "Set Safe" button visible only in risk), Monitor strip (loudness meter + value label, summary label, settings popover: Smoothing 15-95%, Display Gain -12..32, Freeze toggle)

**Rust** (`window_utility.rs:24-317`): `UtilityPane` with `gtk4::ScrolledWindow` container containing preset section and system section. Preset section: Current Curve label + state chip, Load Preset dropdown, Scope row, Auto Preset switch, Fallback row (visible when unsaved changes), Save + More popover (Save As/Revert/Reset to Neutral/Import/Export/Delete). System section: Signal state chip, A/B bypass switch, Headroom panel (3-segment meter 260x14 with safe/tight/risk colors, detail label, Preamp scale -24..6, "Set Safe" button visible only in risk), Monitor strip (loudness meter + value label, summary label, settings popover: Smoothing 15-95%, Display Gain -12..32, Freeze toggle).

**Acceptance criterion**: Rust utility pane must include preset section with state chip + scope + auto-preset switch + fallback + save/revert/import/export/delete actions; system section with headroom panel with meter + preamp + Set Safe button; monitor strip with loudness meter + value + summary + smoothing/display gain controls.

## Verified Analyzer Divergence

**Upstream** (`window_analyzer.py`): `AnalyzerPlotWidget` overlay on graph, 30 log-band FFT spectrum, smoothing scale (15-95%), display gain scale (-12..32), freeze switch, LUFS momentary/short-term/integrated meter + value label, summary label ("On · -23 LUFS"), monitor settings popover.

**Rust** (`window_analyzer.rs:8-100`): Analyzer panel with FFT spectrum (64 bars), smoothing scale (15-95%), display gain scale (-12..32), freeze toggle, LUFS value label, summary label ("On · -23 LUFS"), enable analyzer toggle. Height adjustable via `set_height` (120px default, 80px compact).

**Acceptance criterion**: Rust analyzer must include smoothing scale, display gain scale, freeze toggle, LUFS meter with value label, and summary label matching upstream format.

## Verified Headroom Divergence

**Upstream** (`window_headroom.py:34-232`): Headroom panel with state label, peak chip, meter area (260x14, 3 segments safe/tight/risk with colors), detail label, Preamp scale (-24..6, 0.5 step), "Set Safe" button (visible only when risk), meter drawing with 3 colored segments.

**Rust** (`window_headroom.rs:6-195`): Headroom panel with state label with CSS classes (safe/tight/risk/bypass), 3-segment meter drawing (260x14 with colors), detail label, Preamp scale (-24..6, 0.5 step), "Set Safe" button (visible only when risk), peak label, state chip. State transitions based on peak dBFS value.

**Acceptance criterion**: Rust headroom panel must include meter drawing with 3 segments, state label with CSS classes (safe/tight/risk/bypass), Preamp scale, and "Set Safe" button visible only in risk state.

## Verified Presets Divergence

**Upstream** (`window_presets.py:1-1457`): Full preset lifecycle — load/save/save-as/revert/reset-to-neutral/import/export/delete, output-preset auto-linking by route key, fallback presets, file monitoring (Gio.FileMonitor), preset name existence checks, replace dialog.

**Rust** (`window_presets.rs:6-157`): Preset management widget with ListBox, add/delete/save buttons. Preset names listed from `.config/mini-eq/presets/`. Save writes `{}` JSON (no band data yet). No revert/reset/import/export/delete beyond basic ListBox operations. No output-preset linking.

**Acceptance criterion**: Rust must implement full preset lifecycle with payload serialization (band data + preamp), file monitoring, fallback presets, and output-preset route linking.

## Verified AutoEq Divergence

**Upstream** (`window_autoeq.py:1-880`): Full AutoEq dialog — search entry + refresh button + results list + curve preview drawing area + import button + async load + download + preview debounce (240ms) + import flow.

**Rust** (`window_autoeq.rs:10-27`): Placeholder `MessageDialog` with text "AutoEq presets will be implemented in a future update."

**Acceptance criterion**: Rust must include full AutoEq import dialog with search, results list, curve preview, and import functionality.

## Verified Preferences Divergence

**Upstream** (`window_preferences.py:1-215`): `Adw.PreferencesDialog` with Background group (Keep Running in Background toggle, Start at Login toggle, Enable System-wide EQ at Login toggle, with portal/autostart permission requests).

**Rust** (`window_preferences.rs:10-41`): Placeholder `Dialog` with text "Preferences will be implemented in a future update."

**Acceptance criterion**: Rust must include preferences dialog with background mode, start-at-login, and start-active-at-login toggles matching upstream behavior.

## Verified Window State Divergence

**Upstream** (`window.py:146-153`): Restores width/height/position via Gio.Settings; `initial_layout_height` from monitor geometry; `set_size_request(min_width, compact_min_height)` then `set_default_size(*initial_window_default_size())`.

**Rust** (`window_state.rs:5-13`, `window.rs:27-28`): Restores width/height from AppearanceSettings; `set_default_size(1360, 720)` fixed; no monitor geometry fallback.

**Acceptance criterion**: Rust must restore window geometry (width, height, position) from persisted settings matching upstream.

## Verified Backend Divergence

**Upstream** (`routing.py:101-1294`): `SystemWideEqController` — PipeWireBackend connection + `connect()`, `create_virtual_sink()`, `create_filter_chain()`, `configure_biquad_filters()`, `create_output_node()`, `link_nodes()`. Stream router with `enable()`, `disable()`, `route_output_streams()`, `restore_output_streams()`. Output analyzer with prepare/start/stop. Metadata-driven default sink following. Output-preset target resolution with route keys. Filter control param echo suppression (500us grace). Graph rate monitor. Full start/shutdown sequence.

**Rust** (`pipewire_backend.rs:28-323`): Creates virtual sink + filter chain + output node via `core.create_object`, links nodes (no-op), detects routes, `auto_route_to_sink` no-op. `update_band_coefficients` configures biquad nodes but never actually applies to running graph (no module load, no param set). `routing.rs` is empty. `pipewire_routes.rs` and `pipewire_stream_router.rs` are duplicate incomplete route abstractions with stub `auto_route_to_sink`.

**Acceptance criterion**: Rust backend must produce PipeWire nodes that can be linked, configure biquad filters via native `set_param`, support stream routing/auto-routing, and provide output analyzer with levels/loudness callbacks matching upstream signatures.

## Verified DSP / Core Divergence

**Upstream** (`core.py` constants + math): `total_response_db()`, `estimate_response_peak_db()`, `format_frequency()`, `stepped_response_frequencies()`, `FILTER_TYPES`/`FILTER_TYPE_ORDER`/`FILTER_TYPE_INDEX_BY_VALUE`, `MODE_ORDER`/`MODE_INDEX_BY_VALUE`, 12 filter types with native labels, 2 EQ modes ("Live PipeWire", "Auto Eq").

**Rust** (`core.rs:87-481`): `FilterType` with 12 types + `native_label()`, `SELECTABLE_FILTER_TYPES` (9 types, missing Notch/Resonance/LadderPass/LadderRej from selectable list), constants matching upstream. Missing: `total_response_db`, `estimate_response_peak_db`, `format_frequency`, `stepped_response_frequencies`, EQ modes/mode order maps, filter type index maps.

**Acceptance criterion**: Rust `core.rs` must include all 12 filter type index maps (`FILTER_TYPE_INDEX_BY_VALUE`, reverse lookup), EQ mode order maps (`MODE_INDEX_BY_VALUE`, `MODE_ORDER`), `total_response_db` and `total_response_db_at_frequencies` for graph response computation, and `format_frequency` for display.

## Verified EBU R128 Divergence

**Upstream** (`ebur128.py`): Wrapper around native meter.
**Rust** (`ebur128.rs:23-77`): Ported and verified with tests passing.

**Status**: ✅ Verified parity.

## Verified AutoEq / APO Divergence

**Upstream** (`autoeq.py`): APO parser, entry search, autoeq.app API client.
**Rust** (`autoeq.rs:1-704`): Ported and verified with tests passing.

**Status**: ✅ Verified parity.

## Verified Settings Divergence

**Upstream** (`settings.py`): JSON settings with version, monitor_enabled, appearance, background_mode, start_at_login, start_active_at_login.
**Rust** (`settings.rs:130-192`): Ported and verified with tests passing.

**Status**: ✅ Verified parity.

PLAN0ASSESSMENT
echo "Phase 0 assessment written"

## Phase 1: Backend Engine Parity

**Goal:** Rust PipeWire backend must be able to create a working system-wide EQ pipeline that produces correct biquad filter coefficients and links nodes.

**Scope:**
- `src/pipewire_backend.rs` — verify `core.create_object` node creation actually produces working virtual sink + filter-chain + output node on a live PipeWire session; remove placeholder APIs; confirm `update_band_coefficients` uses native PipeWire `set_param` or equivalent on running graph.
- `src/filter_chain.rs` — verify filter-chain configuration matches upstream module properties (biquad count, audio format, channel positions, capture/play ports).
- `src/routing.rs` — implement or delete; route-detection logic currently split between `pipewire_routes.rs` and `pipewire_stream_router.rs`; consolidate into one module with clear `enable()`/`disable()`/`route_output_streams()` signatures matching upstream.
- `src/pipewire_routes.rs` + `src/pipewire_stream_router.rs` — deduplicate; remove stub `auto_route_to_sink` and replace with upstream behavior: metadata-driven default sink following, output-preset target resolution by route key.
- `src/analyzer.rs` — ensure output analyzer `prepare/start/stop` callbacks match upstream signature and feed levels/loudness into UI.

**Risk:** Without a live PipeWire session, backend parity is best-effort; must be validated on target hardware before Phase 8.

**Acceptance:** `cargo test --lib` passes with PipeWire-adjacent tests; manual verification on desktop confirms EQ audible and system-wide.

## Phase 2: Graph and Fader Interactions

**Goal:** Rust graph and fader modules must match upstream drawing behavior and user interactions.

**Scope:**
- `src/window_graph.rs` — replace single DrawingArea with 3 overlay DrawingAreas (background grid + dB/freq markers, analyzer overlay, frequency response curve). Add click+drag editing: click selects band, shift+drag adjusts Q, drag adjusts frequency+gain for shelf filters; apply changes to engine on drag end.
- `src/window_graph.rs` — add selected-band vertical line + halo marker; default content width 900, height 196; compact 760x156; roomy 280.
- `src/window_band_fader.rs` — replace dense per-row controls above each fader with a single shared selected-band editor below the fader grid (WrapBox or equivalent) with Mute/Solo/Type/Frequency/Q/Gain controls. In compact mode, inline with natural line length 820/720.
- `src/window_layout.rs` — fader grid height responsive: default 208px, compact 164px, roomy 300px; scroller min-height adaptation; visible band count gating (up to 32 bands, default 10 visible).

**Acceptance:** Graph and fader interactions match original screenshots; drag editing feels identical; compact/roomy modes responsive.

## Phase 3: Adaptive Shell

**Goal:** Rust window must support upstream adaptive layout, breakpoints, and toolbar interactions.

**Scope:**
- `src/window_layout.rs` — replace fixed `Paned` with `Adw.OverlaySplitView` matching upstream (pinned sidebar, width fraction 0.24, min 268, max 320). Add breakpoints at 1320sp and 1080sp: collapse sidebar to END, show utility pane button, compact toolbar and graph/fader/analyzer heights. Add F9 binding to toggle sidebar. Add toast overlay above clamp.
- `src/window.rs` — toolbar must include EQ output dropdown (300px default, 240px compact) with sink labels, Main Menu button (import AutoEq, import APO, appearance light/dark, preferences, about), System-wide EQ route switch + label on right, Inspector Pane toggle button.
- `src/window_state.rs` — restore window geometry (width/height/position) from persisted settings; add monitor geometry fallback matching upstream `initial_layout_height`.

**Acceptance:** Window visually matches original screenshot at 1360x959; breakpoints collapse correctly at narrower widths; F9 toggles sidebar.

## Phase 4: Utility / Sidebar

**Goal:** Rust utility pane must include full preset section, headroom panel, and monitor strip matching upstream.

**Scope:**
- `src/window_utility.rs` — replace `gtk4::Stack` with upstream-style sections:
  - Preset section: Current Curve label + state chip, Load Preset dropdown, Scope row, Auto Preset switch, Fallback row (visible when unsaved changes), Save + More popover (Save As/Revert/Reset to Neutral/Import/Export/Delete).
  - System section: Signal state chip, A/B bypass switch, Headroom panel (meter 260x14, 3 segments safe/tight/risk with colors), detail label, Preamp scale -24..6 (0.5 step), "Set Safe" button visible only in risk state. Monitor strip (loudness meter + value label, summary label, settings popover: Smoothing 15-95%, Display Gain -12..32, Freeze toggle).

**Acceptance:** Utility pane matches upstream screenshot; preset save/load persists band data; headroom meter draws 3 colored segments; monitor settings popover works.

## Phase 5: Analyzer and Headroom

**Goal:** Rust analyzer must include LUFS metering and smooth spectrum; headroom must match upstream meter drawing.

**Scope:**
- `src/window_analyzer.rs` — replace standalone panel with analyzer overlay on graph; add smoothing scale (15-95%), display gain scale (-12..32), freeze switch, LUFS momentary/short-term/integrated meter + value label, summary label ("On · -23 LUFS"), monitor settings popover.
- `src/window_headroom.rs` — replace ProgressBar with meter drawing (260x14, 3 segments safe/tight/risk with colors); add state label with CSS classes (safe/tight/risk/bypass), Preamp scale (-24..6, 0.5 step), "Set Safe" button visible only in risk state.

**Acceptance:** Analyzer overlay shows live spectrum with smoothing/gain controls; headroom meter draws colored segments and reacts to peak levels; Set Safe button appears in risk state.

## Phase 6: Presets, AutoEq, Preferences

**Goal:** Rust must support full preset lifecycle, AutoEq import dialog, and preferences dialog.

**Scope:**
- `src/window_presets.rs` — implement full preset lifecycle: load/save/save-as/revert/reset-to-neutral/import/export/delete, output-preset auto-linking by route key, fallback presets, file monitoring (Gio.FileMonitor), preset name existence checks, replace dialog. Save must serialize band data + preamp to JSON.
- `src/window_autoeq.rs` — replace placeholder with full AutoEq dialog: search entry + refresh button + results list + curve preview drawing area + import button + async load + download + preview debounce (240ms) + import flow.
- `src/window_preferences.rs` — replace placeholder with `Adw.PreferencesDialog`: Background group (Keep Running in Background toggle, Start at Login toggle, Enable System-wide EQ at Login toggle, with portal/autostart permission requests).

**Acceptance:** Presets round-trip correctly; AutoEq search returns results and imports curves; preferences toggles persist and apply on restart.

## Phase 7: DSP / Core Math and Missing Constants

**Goal:** Rust `core.rs` must match upstream constants, index maps, and graph response helpers.

**Scope:**
- `src/core.rs` — add `FILTER_TYPE_INDEX_BY_VALUE`, reverse lookup, `MODE_INDEX_BY_VALUE`, `MODE_ORDER`, `total_response_db`, `total_response_db_at_frequencies`, `format_frequency`. Verify `SELECTABLE_FILTER_TYPES` includes all 12 upstream types.
- `src/core.rs` — add 2 EQ modes ("Live PipeWire", "Auto Eq") and mode-order maps.
- `src/analyzer.rs` — ensure FFT spectrum and EBU R128 LUFS computation match upstream smoothing behavior.
- `src/autoeq.rs` — verify APO parser, entry search, and autoeq.app API client behavior matches upstream edge cases.

**Acceptance:** `cargo test --lib` passes all DSP and core tests; graph response curve matches upstream for identical band configs.

## Phase 8: CI / Runtime Validation

**Goal:** Rust rewrite must build, test, and run identically to upstream on target hardware.

**Scope:**
- Build: `cargo build --release` with no warnings; `cargo fmt --all -- --check` clean; `cargo clippy --all-targets -- -D warnings` clean.
- Test: `cargo test --lib --no-fail-fast` passes; add missing integration tests for preset round-trip, graph response, and AutoEq import.
- Runtime: manual verification on PipeWire desktop; EQ audible system-wide; presets persist; analyzer/headroom/graph/utility all functional; adaptive layout at all breakpoints; F9 sidebar toggle; output dropdown routing works.
- Packaging: verify Flatpak build if required; update `docs/{date}-updates.md` and `docs/{date}-handover.md` for handoff.

**Acceptance:** All automated checks green; original screenshot visual contract matched at 1360x959; system-wide EQ active and verified by playback test.

---

## Implementation Order Summary

1. Phase 1: Backend engine parity (blocking all UI correctness)
2. Phase 2: Graph and fader interactions (visual parity)
3. Phase 3: Adaptive shell (layout parity)
4. Phase 4: Utility / sidebar (feature parity)
5. Phase 5: Analyzer and headroom (feature parity)
6. Phase 6: Presets, AutoEq, preferences (feature parity)
7. Phase 7: DSP / core math (correctness parity)
8. Phase 8: CI / runtime validation (release readiness)

Each phase should be reviewed and approved before the next begins. No code changes until this plan is approved.
