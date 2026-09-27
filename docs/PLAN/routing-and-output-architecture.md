# System EQ Routing & Output Architecture

Date: 2026-09-27
Status: routing + output-target fix landed; monitor (spectrum) port pending.

This document records how the Rust port is *supposed* to wire audio through the
EQ, the root causes of "sliders do nothing" / "System EQ mutes audio", and the
portable fix. It mirrors the upstream Python (`upstream/main` = `bhack/mini-eq`)
logic throughout. **Read this before touching `routing.rs` / `pipewire_backend.rs`.**

---

## 1. Intended audio topology

```
app playback streams (Stream/Output/Audio)
        │  (routed via default-metadata target.node / target.object)
        ▼
  mini_eq_sink            (Audio/Sink, created by filter-chain module capture.props)
        │  (internal filter-chain graph: biquad nodes)
        ▼
  mini_eq_sink_output     (Stream/Input/Audio, filter-chain playback.props, node.passive=true)
        │  (target.object = the user's DEFAULT physical sink)
        ▼
  <physical default sink> (e.g. alsa_output.pci-...analog-stereo)
```

Key points:
- `libpipewire-module-filter-chain` creates BOTH the sink (`capture.props`,
  `media.class=Audio/Sink`) and the playback node (`playback.props`,
  `node.passive=true`, `target.object=<physical default sink>`).
- The playback node is **passive**: its link to the physical sink only goes
  `active` when the physical sink is actually running. If the playback node
  targets a *suspended* sink, the link stays `paused` → **no audio** (mute).
- Apps are moved into `mini_eq_sink` by the **session manager (WirePlumber)**,
  driven by **default metadata**, NOT by manually creating `link-factory` links.

---

## 2. Root cause #1 — "sliders do nothing" (routing)

**Symptom:** moving Freq/Q/Gain sliders produced no audible change.

**Cause:** the Rust `auto_route_to_sink` used `link-factory`
(`core.create_object::<Link>("link-factory", …)`) to link app streams into
`mini_eq_sink`. **WirePlumber policy immediately overrides** manual links and
re-routes streams back to the default sink. The created links did not persist;
the stream's effective `target` stayed `none`.

**Correct mechanism (upstream `set_stream_target`):** set each stream's target
on the PipeWire **`default` metadata** object:
- `target.node`   = sink bound_id,   type `Spa:Id`  (legacy compat)
- `target.object` = sink object.serial, type `Spa:Id` (modern WirePlumber policy)

WirePlumber observes these and moves the stream itself. This is the same
mechanism `pactl move-sink-input` / pavucontrol use.

**Rust implementation (`routing.rs`):**
- Bind the `default` metadata: enumerate registry globals for
  `ObjectType::Metadata` with `metadata.name == "default"`, then
  `registry.bind::<Metadata, _>(global)` (synchronous in pipewire 0.10.1).
  - NOTE: `pw_core_get_metadata` is **not** exported by libpipewire 1.6 and is
    **not** in `pipewire-sys` bindings. Do NOT try to `extern` it — bind the
    global instead.
- `set_stream_target(stream_id, sink_bound_id, sink_serial)` →
  `md.set_property(stream_id, "target.node", Some("Spa:Id"), Some(id))` and
  `… "target.object" … serial`, then a core sync roundtrip.
- `clear_stream_target` / `unroute_all` → set the same keys to `None`
  (System EQ OFF returns streams to the default sink).

**Gotcha:** a `Metadata` `property` listener (or any listener) is
**unregistered when dropped**. Store it (see `metadata_listeners` field) or it
silently stops firing.

---

## 3. Root cause #2 — "System EQ mutes audio" (wrong output target)

**Symptom:** toggling System EQ ON made audio go silent.

**Cause:** the filter-chain **playback node's `target.object`** was set to the
**wrong physical sink**. The old `default_output_sink()` used
`detect_routes()`, which hardcoded `active: true` for *every* matching node and
returned the first one — on this machine that was the **Logitech USB headset**
(a suspended device), while the user's real default was the **PCI sink**.

Because the playback node targeted a suspended sink, its passive link stayed
`paused`. Apps routed into `mini_eq_sink` → filters → `mini_eq_sink_output` →
**suspended** Logitech → nothing reaches the PCI sink the user hears → mute.

**Portable fix (upstream `DEFAULT_AUDIO_SINK_KEY`):** read the user's real
default sink from the `default` metadata key **`default.audio.sink`**. Its
value is a JSON object: `{"name":"alsa_output.pci-…analog-stereo", …}`.
Parse the `name` field (`parse_metadata_node_name`) and use it as the
filter-chain playback `target.object`.

This is **machine-agnostic**: on any PipeWire system, `default.audio.sink` is
the authoritative current default output. No hardcoded device names, no
"first active route" guessing.

**Rust implementation (`routing.rs`):**
- `ensure_default_metadata` registers a `property` listener that captures:
  - `default.audio.sink`            → `default_audio_sink`
  - `default.configured.audio.sink` → `configured_audio_sink` (fallback)
  The server **replays** current properties when the metadata client binds, so
  the values land during the bind roundtrip (pump a few extra iterations to be
  safe — see `default_audio_sink_name`).
- `default_audio_sink_name()` returns `default_audio_sink` or falls back to
  `configured_audio_sink`.
- `pipewire_backend::default_output_sink()` now delegates to
  `routing.default_audio_sink_name()` (was the broken `detect_routes` hack).

**Verified live:**
```
metadata property default.audio.sink = Some("{\"name\":\"alsa_output.pci-0000_04_00.6.analog-stereo\"}")
  -> Some("alsa_output.pci-0000_04_00.6.analog-stereo")
Loading filter-chain module -> alsa_output.pci-0000_04_00.6.analog-stereo
mini_eq_sink_output -> alsa_output.pci-0000_04_00.6.analog-stereo
```

---

## 4. Live control path (already fixed, for reference)

- Slider edits (Freq/Q/Gain) push `SPA_PARAM_Props` to the live `mini_eq_sink`
  node proxy via `apply_live_controls` (mirrors upstream `set_node_params`).
- Control name format: `band_<side>_<idx>_filter:Freq`, `band_<side>_<idx>:Gain 1`, etc.
- **`SPA_Props_params` key = `0x80001` (524289)**, confirmed via `pw-cli`
  echo `Props:params (524289)`. (The old `0x80000` was wrong.)
- Filter-**type** changes are topology changes → restart the filter-chain module
  (not a live push). Type changes recreate `mini_eq_sink`.

---

## 5. Remaining work

1. **Monitor / spectrum analyzer** — the fix branch's `analyzer.rs` has the DSP
   math but is **missing the `OutputSpectrumAnalyzer` capture struct**. The
   `test/session-2026-09-22` branch has it complete (1223-line `analyzer.rs`:
   `start_capture`/`stop_capture`/`display_levels`/`display_loudness`/
   `monitor_stats`, capture stream `Stream/Input/Audio` with
   `target.object=<sink>`, FFT + LUFS in `process_capture_buffers`).
   **Port that struct + `start_monitor`/`link_monitor_to_sink`/`monitor_levels`
   into the fix branch** and feed `monitor_levels()` into
   `EqGraph::update()`'s `analyzer_levels` arg (currently `&[]`).
2. **Re-target filter output on default change** — currently the playback
   target is set at module load. If the user switches default sinks at runtime,
   the filter output should follow (upstream listens for `default.audio.sink`
   changes and re-targets). The property listener is already wired; add the
   re-target action.
3. **Re-route after filter-type restart** — a type change recreates
   `mini_eq_sink`; re-run `auto_route_to_sink` afterward so streams don't need
   a manual System-EQ re-toggle.
4. **`node.dont-move` / foreign-target guards** — upstream skips streams that
   are marked `node.dont-move` or already have a foreign `target.object`
   (`_is_internal_stream`, `_has_foreign_target_object`). Port these so we
   don't hijack streams the user pinned elsewhere.

---

## 6. Upstream reference map

| Concern | Upstream (Python) | Rust port |
|---|---|---|
| default sink key | `pipewire_backend.py` `DEFAULT_AUDIO_SINK_KEY` | `routing.rs` `default_audio_sink_name` |
| parse JSON name | `parse_metadata_node_name` | `routing.rs` `parse_metadata_node_name` |
| set stream target | `set_stream_target` / `move_stream_to_target` | `routing.rs` `set_stream_target` |
| restore/unroute | `restore_stream_target` / `restore_output_streams` | `routing.rs` `clear_stream_target` / `unroute_all` |
| live EQ controls | `set_node_params` + `build_props_controls_param` | `pipewire_backend.rs` `apply_live_controls` |
| stream router loop | `pipewire_stream_router.py` `route_output_streams` | `routing.rs` `auto_route_to_sink` |
| monitor/analyzer | `analyzer.py` `OutputSpectrumAnalyzer` | **TODO: port from session branch** |
