use std::cell::RefCell;
use std::ffi::CString;
use std::mem::MaybeUninit;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use log::{debug, info, warn};
use pipewire::node::Node;
use pipewire::spa::param::ParamType;
use pipewire::spa::pod::Pod;
use pipewire::spa::pod::builder::Builder;
use pipewire::{Error, context::ContextRc, core::CoreRc, loop_::Timeout, main_loop::MainLoopRc};
use pipewire_sys as pw_sys;

use crate::core::{EqBand, FILTER_OUTPUT_SUFFIX, VIRTUAL_SINK_BASE};
use crate::filter_chain;
use crate::routing::{OutputRoute, RoutingEngine};

/// Owns a `pw_impl_module` loaded through `pw_context_load_module`.
///
/// PipeWire's Rust bindings do not expose module loading, so the handle is kept
/// as a raw pointer and destroyed on drop.
struct ModuleHandle(*mut pw_sys::pw_impl_module);

impl Drop for ModuleHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            // SAFETY: the pointer came from `pw_context_load_module` and is
            // destroyed exactly once.
            unsafe { pw_sys::pw_impl_module_destroy(self.0) };
            self.0 = std::ptr::null_mut();
        }
    }
}

pub struct PipeWireBackend {
    mainloop: MainLoopRc,
    context: ContextRc,
    core: CoreRc,
    filter_chain_module: Option<ModuleHandle>,
    bands: Vec<EqBand>,
    preamp_gain: f64,
    running: Arc<Mutex<bool>>,
    routing: RoutingEngine,
    /// Live proxy for the filter-chain virtual sink node (`mini_eq_sink`).
    /// Captured by the registry listener once the module creates it; used by
    /// `apply_live_controls` to push `SPA_PARAM_Props` without a reload.
    filter_node: Rc<RefCell<Option<Node>>>,
    /// The band filter-types the currently-loaded module was built with.
    /// Native biquad labels are graph *topology*, not mutable controls: a
    /// type/preset change requires a module restart, while ordinary
    /// frequency/Q/gain edits stay live. Mirrors upstream `_engine_band_types`.
    engine_band_types: Vec<crate::core::FilterType>,
    /// Kept alive so the registry `global` listener stays registered for the
    /// backend's lifetime (listeners unregister themselves when dropped).
    _registry_listener: Option<pipewire::registry::Listener>,
    /// Live output spectrum analyzer (monitor). Owns the capture stream that
    /// taps the output sink's monitor ports. Mirrors upstream
    /// `output_analyzer`. Started/stopped via `start_monitor`/`stop_monitor`.
    analyzer: crate::analyzer::OutputSpectrumAnalyzer,
    /// Target sink for a monitor start whose port-linking is still pending
    /// negotiation. Driven forward by `pump_monitor_link` from the update
    /// loop so the GTK UI never blocks on the (multi-second) link wait.
    pending_monitor_target: Option<String>,
}

impl PipeWireBackend {
    pub fn new(bands: Vec<EqBand>) -> Result<Self, Error> {
        info!("Initializing PipeWire backend");

        pipewire::init();

        let mainloop = MainLoopRc::new(None)?;
        let context = ContextRc::new(&mainloop, None)?;
        let core = context.connect_rc(None)?;

        info!("Connected to PipeWire server");

        let routing = RoutingEngine::new(core.clone(), mainloop.clone());
        let analyzer =
            crate::analyzer::OutputSpectrumAnalyzer::new(core.clone(), crate::core::SAMPLE_RATE)?;

        let backend = PipeWireBackend {
            mainloop,
            context,
            core,
            filter_chain_module: None,
            bands,
            preamp_gain: 0.0,
            running: Arc::new(Mutex::new(true)),
            routing,
            filter_node: Rc::new(RefCell::new(None)),
            engine_band_types: Vec::new(),
            _registry_listener: None,
            analyzer,
            pending_monitor_target: None,
        };

        let registry_listener = backend.setup_registry_listener()?;
        let backend = PipeWireBackend {
            _registry_listener: Some(registry_listener),
            ..backend
        };

        Ok(backend)
    }

    fn setup_registry_listener(&self) -> Result<pipewire::registry::Listener, Error> {
        let registry = self.core.get_registry_rc()?;
        let filter_node = self.filter_node.clone();
        let registry_for_cb = registry.clone();
        let listener = registry.add_listener_local();
        let listener = listener.global(move |global| {
            debug!("Registry global: id={} type={:?}", global.id, global.type_);
            if global.type_.to_str() != pipewire::types::ObjectType::Node.to_str() {
                return;
            }
            let is_eq_sink = global
                .props
                .as_ref()
                .and_then(|p| p.get("node.name"))
                .map(|n| n == VIRTUAL_SINK_BASE)
                .unwrap_or(false);
            if is_eq_sink && filter_node.borrow().is_none() {
                match registry_for_cb.bind::<Node, _>(global) {
                    Ok(node) => {
                        info!(
                            "Captured live filter node proxy: {} (id={})",
                            VIRTUAL_SINK_BASE, global.id
                        );
                        *filter_node.borrow_mut() = Some(node);
                    }
                    Err(e) => warn!("Failed to bind filter node: {}", e),
                }
            }
        });
        let _listener = listener.register();
        Ok(_listener)
    }

    // ---------------------------------------------------------------------
    // Output monitor (spectrum analyzer + loudness meter)
    // ---------------------------------------------------------------------

    /// Start monitoring the processed output for the spectrum analyzer and
    /// loudness meter, mirroring upstream `ensure_output_analyzer`.
    ///
    /// Non-blocking: starts the capture stream and records the target so the
    /// port-linking (which must wait for stream negotiation) is driven forward
    /// by [`pump_monitor_link`] from the window's update loop. This keeps the
    /// GTK UI responsive instead of freezing on the multi-second link wait.
    pub fn start_monitor(&mut self, target_sink_name: &str) -> Result<(), Error> {
        info!("Starting output monitor of {target_sink_name}");
        self.analyzer.start_capture(target_sink_name, None)?;
        self.pending_monitor_target = Some(target_sink_name.to_string());
        Ok(())
    }

    /// Advance the pending monitor port-linking. Called every tick from the
    /// update loop. Once the capture stream has negotiated (Paused/Streaming)
    /// and the sink + our node are both visible in the registry, link the
    /// sink's monitor ports to our analyzer inputs with `pw-link` (the
    /// session manager otherwise routes capture to the default source, e.g.
    /// a microphone), then drop any foreign links it made meanwhile.
    ///
    /// Does a bounded amount of work per call so it never blocks the UI for
    /// long. Returns `true` when linking is finished (success or giving up).
    pub fn pump_monitor_link(&mut self) -> bool {
        let Some(target) = self.pending_monitor_target.clone() else {
            return true;
        };
        // Wait until the capture stream has finished negotiating.
        let negotiated = matches!(
            self.analyzer.stream_state(),
            Some(pipewire::stream::StreamState::Paused)
                | Some(pipewire::stream::StreamState::Streaming)
        );
        if !negotiated {
            return false;
        }
        let our_id = self.analyzer.stream_node_id();
        if our_id == 0 {
            return false;
        }
        let nodes = registry_node_names(&self.mainloop, &self.core).unwrap_or_default();
        let Some(hw_id) = nodes
            .iter()
            .find(|(name, _)| name == &target)
            .map(|(_, id)| *id)
        else {
            return false;
        };
        let snapshot = registry_port_snapshot(&self.mainloop, &self.core).unwrap_or_default();
        // Port ids collide per direction (playback_FL and monitor_FL are both
        // port.id 0), so link by port NAME.
        let hw_mon: Vec<String> = snapshot
            .iter()
            .filter(|p| p.node_id == hw_id && p.direction == "out" && p.path.contains("monitor"))
            .map(|p| p.port_name.clone())
            .collect();
        let our_in: Vec<String> = snapshot
            .iter()
            .filter(|p| {
                p.node_id == our_id && p.direction == "in" && p.port_name.starts_with("input")
            })
            .map(|p| p.port_name.clone())
            .collect();
        if hw_mon.len() < 2 || our_in.len() < 2 {
            return false;
        }
        for (out_port, in_port) in hw_mon.iter().zip(our_in.iter()).take(2) {
            let out_ref = format!("{target}:{out_port}");
            let in_ref = format!("{}:{in_port}", crate::analyzer::ANALYZER_NODE_NAME);
            let owned = [out_ref.clone(), in_ref.clone()];
            let refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
            match run_pw_link(&self.mainloop, &refs, std::time::Duration::from_secs(10)) {
                Some(o) if o.status.success() => info!("Linked monitor {out_ref} -> {in_ref}"),
                Some(o) => warn!(
                    "pw-link failed: {} {}",
                    o.status,
                    String::from_utf8_lossy(&o.stderr).trim()
                ),
                None => warn!("pw-link timed out"),
            }
        }
        self.drop_foreign_monitor_links(our_id, hw_id);
        self.pending_monitor_target = None;
        true
    }

    /// Destroy links into our analyzer inputs that don't come from the
    /// monitored sink (e.g. session-manager microphone links).
    fn drop_foreign_monitor_links(&self, our_id: u32, hw_id: u32) {
        let links = registry_link_snapshot(&self.mainloop, &self.core).unwrap_or_default();
        for link in links {
            if link.input_node == our_id && link.output_node != hw_id {
                let id = link.id.to_string();
                let args = ["destroy", id.as_str()];
                if let Some(o) =
                    run_pw_cli_pumped(&self.mainloop, &args, std::time::Duration::from_secs(10))
                    && o.status.success()
                {
                    info!("Dropped foreign monitor link {}", link.id);
                }
            }
        }
    }

    /// Stop the output monitor.
    pub fn stop_monitor(&mut self) {
        self.pending_monitor_target = None;
        self.analyzer.stop_capture();
    }

    /// Normalized 0..1 spectrum levels from live captured audio.
    pub fn monitor_levels(&self) -> Vec<f64> {
        self.analyzer.display_levels()
    }

    /// Latest loudness snapshot from live captured audio, if any.
    pub fn monitor_loudness(&self) -> Option<crate::analyzer::AnalyzerLoudnessSnapshot> {
        self.analyzer.display_loudness()
    }

    /// Monitor diagnostics: (frames, active bands, peak, mean).
    pub fn monitor_stats(&self) -> (u64, usize, f32, f32) {
        self.analyzer.monitor_stats()
    }

    /// Whether the monitor capture is currently enabled.
    pub fn monitor_enabled(&self) -> bool {
        self.analyzer.is_enabled()
    }

    /// Push the current band/preamp state to the live filter node via
    /// `SPA_PARAM_Props`, mirroring upstream `set_node_params` /
    /// `apply_state_to_engine`. This changes the DSP in milliseconds without
    /// tearing down (and re-linking) the graph.
    ///
    /// Returns `Ok(false)` if the live node proxy is not available yet (caller
    /// should fall back to a module reload).
    pub fn apply_live_controls(&self, eq_enabled: bool) -> Result<bool, Error> {
        let node_borrow = self.filter_node.borrow();
        let node = match node_borrow.as_ref() {
            Some(n) => n,
            None => return Ok(false),
        };

        let controls =
            filter_chain::native_biquad_control_values(&self.bands, self.preamp_gain, eq_enabled);
        if controls.is_empty() {
            return Ok(true);
        }

        // Build the SPA_PARAM_Props object whose `params` property is a struct
        // of alternating (control-name string, value double) pairs — the exact
        // wire format parsed by filter-graph's `parse_params`.
        let mut data: Vec<u8> = Vec::new();
        {
            let mut builder = Builder::new(&mut data);
            let mut obj_frame = MaybeUninit::zeroed();
            let mut struct_frame = MaybeUninit::zeroed();

            // SAFETY: frames are kept alive until popped; the builder owns its
            // data buffer and is dropped before we read `data`.
            unsafe {
                builder
                    .push_object(
                        &mut obj_frame,
                        pipewire::spa::utils::SpaTypes::ObjectParamProps.as_raw(),
                        ParamType::Props.as_raw(),
                    )
                    .map_err(|_| Error::CreationFailed)?;
                // SPA_Props_params == 0x80001 (524289), confirmed via
                // `pw-cli set-param` echo: `Props:params (524289)`. Using
                // 0x80000 makes filter-graph's parse_params never find the
                // control values (silent EQ).
                builder
                    .add_prop(0x80001, 0)
                    .map_err(|_| Error::CreationFailed)?;
                builder
                    .push_struct(&mut struct_frame)
                    .map_err(|_| Error::CreationFailed)?;
                for (name, value) in &controls {
                    builder
                        .add_string(name)
                        .map_err(|_| Error::CreationFailed)?;
                    builder
                        .add_double(*value)
                        .map_err(|_| Error::CreationFailed)?;
                }
                builder.pop(struct_frame.assume_init_mut());
                builder.pop(obj_frame.assume_init_mut());
            }
        }

        let pod = Pod::from_bytes(&data).ok_or(Error::CreationFailed)?;
        node.set_param(ParamType::Props, 0, pod);
        debug!(
            "apply_live_controls: pushed {} control(s) to {}",
            controls.len(),
            VIRTUAL_SINK_BASE
        );
        Ok(true)
    }

    /// Update the DSP for new bands WITHOUT a reload when the live node is
    /// available; otherwise fall back to a full module reload.
    ///
    /// A filter-type change is a graph topology change (the biquad `label`
    /// is fixed at module-load time), so it forces a restart instead of a
    /// live push — matching upstream `set_filter_controls`.
    pub fn update_state_live_or_reload(&mut self, output_sink: &str) -> Result<(), Error> {
        let current_types: Vec<crate::core::FilterType> =
            self.bands.iter().map(|b| b.filter_type).collect();
        let types_changed =
            !self.engine_band_types.is_empty() && current_types != self.engine_band_types;

        if types_changed {
            info!("Filter-type change detected: restarting engine (topology change)");
            // Drop the stale proxy so the registry listener re-captures the
            // freshly-created node after the reload.
            *self.filter_node.borrow_mut() = None;
            self.unload_filter_chain_module();
            return self.create_filter_chain(output_sink);
        }

        match self.apply_live_controls(true) {
            Ok(true) => Ok(()),
            _ => {
                let bands = self.bands.clone();
                self.update_band_coefficients(&bands, output_sink)
            }
        }
    }

    /// Build the filter-chain argument string for the current bands.
    pub fn filter_chain_args(&self, output_sink: &str, eq_enabled: bool) -> String {
        filter_chain::build_filter_chain_module_args(
            &self.bands,
            self.preamp_gain,
            eq_enabled,
            VIRTUAL_SINK_BASE,
            &format!("{}{}", VIRTUAL_SINK_BASE, FILTER_OUTPUT_SUFFIX),
            output_sink,
            true,
        )
    }

    /// Load `libpipewire-module-filter-chain`, which creates the virtual sink
    /// (capture side), the DSP graph and the playback node as a single module.
    ///
    /// This replaces the previous per-node `create_object` calls: the
    /// filter-chain is a module, not an object factory, and the sink/output
    /// nodes are declared in its `capture.props`/`playback.props` sections.
    pub fn create_filter_chain(&mut self, output_sink: &str) -> Result<(), Error> {
        info!("Loading filter-chain module -> {}", output_sink);

        let args = self.filter_chain_args(output_sink, true);
        let c_name = CString::new(filter_chain::FILTER_CHAIN_MODULE_NAME)
            .map_err(|_| Error::CreationFailed)?;
        let c_args = CString::new(args).map_err(|_| Error::CreationFailed)?;

        // SAFETY: `self.context` outlives the module (the module is destroyed in
        // `unload_filter_chain_module` before the context drops), and both
        // strings are NUL-terminated for the duration of the call.
        let module = unsafe {
            pw_sys::pw_context_load_module(
                self.context.as_raw_ptr(),
                c_name.as_ptr(),
                c_args.as_ptr(),
                std::ptr::null_mut(),
            )
        };

        if module.is_null() {
            warn!("pw_context_load_module returned NULL");
            return Err(Error::CreationFailed);
        }

        self.filter_chain_module = Some(ModuleHandle(module));
        self.engine_band_types = self.bands.iter().map(|b| b.filter_type).collect();
        info!("Filter-chain module loaded");
        Ok(())
    }

    /// Tear down the loaded filter-chain module and its nodes.
    pub fn unload_filter_chain_module(&mut self) {
        if let Some(handle) = self.filter_chain_module.take() {
            // Take the raw pointer out and skip `ModuleHandle::drop`, which
            // would destroy the same module a second time (double free).
            let ptr = handle.0;
            std::mem::forget(handle);
            // SAFETY: `ptr` came from `pw_context_load_module` and is destroyed
            // exactly once, here.
            unsafe { pw_sys::pw_impl_module_destroy(ptr) };
            info!("Filter-chain module unloaded");
        }
    }

    /// Native biquad control values for the current bands.
    pub fn native_control_values(&self, eq_enabled: bool) -> Vec<(String, f64)> {
        filter_chain::native_biquad_control_values(&self.bands, self.preamp_gain, eq_enabled)
    }

    pub fn detect_output_routes(&self) -> Result<Vec<OutputRoute>, Error> {
        self.routing.detect_routes()
    }

    /// Pump pending PipeWire events without blocking.
    ///
    /// The PipeWire `MainLoop` is not run via `run()` in GUI mode; instead the
    /// UI update loop calls this each tick so registry events, sync roundtrips
    /// and module callbacks are dispatched on the GTK main thread.
    pub fn pump(&self) {
        self.mainloop.loop_().iterate(Timeout::None);
    }

    /// Name of the active physical output sink, falling back to the first
    /// The user's current default output sink node name, read from the
    /// PipeWire `default` metadata (`default.audio.sink`). This is the
    /// sink the filter-chain playback node targets so EQ'd audio reaches
    /// the speakers the user actually hears on — portable across any
    /// PipeWire machine (no hardcoded device assumptions).
    pub fn default_output_sink(&mut self) -> Option<String> {
        self.routing.default_audio_sink_name()
    }

    pub fn auto_route_to_sink(&mut self, sink_name: &str) -> Result<(), Error> {
        self.routing.auto_route_to_sink(sink_name)
    }

    /// Clear routing targets for all playback streams (System EQ off).
    pub fn unroute_all(&mut self) -> Result<(), Error> {
        self.routing.unroute_all()
    }

    /// Update the DSP graph for a new set of bands.
    ///
    /// The native filter-chain computes coefficients at the DSP clock rate, so
    /// live edits are applied by reloading the module with fresh Freq/Q/Gain
    /// control values rather than by pushing raw coefficients.
    pub fn update_band_coefficients(
        &mut self,
        bands: &[EqBand],
        output_sink: &str,
    ) -> Result<(), Error> {
        info!("Updating band coefficients for {} bands", bands.len());

        self.bands = bands.to_vec();
        self.unload_filter_chain_module();
        self.create_filter_chain(output_sink)
    }

    pub fn set_preamp(&mut self, gain_db: f64) -> Result<(), Error> {
        self.preamp_gain = gain_db.clamp(-24.0, 6.0);
        info!("Preamp gain set to {} dB", self.preamp_gain);
        Ok(())
    }

    pub fn get_bands(&self) -> &[EqBand] {
        &self.bands
    }

    pub fn get_bands_mut(&mut self) -> &mut Vec<EqBand> {
        &mut self.bands
    }

    pub fn get_preamp(&self) -> f64 {
        self.preamp_gain
    }

    pub fn is_running(&self) -> bool {
        *self.running.lock().unwrap()
    }

    pub fn stop(&mut self) {
        *self.running.lock().unwrap() = false;
        self.unload_filter_chain_module();
        info!("PipeWire backend stopping");
    }

    pub fn run(&mut self) {
        info!("Entering PipeWire main loop");
        self.mainloop.run();
    }

    pub fn quit(&mut self) {
        self.mainloop.quit();
        info!("PipeWire main loop quit");
    }
}

impl Default for PipeWireBackend {
    fn default() -> Self {
        let bands = crate::core::default_bands();
        Self::new(bands).expect("Failed to create PipeWire backend")
    }
}

// ---------------------------------------------------------------------------
// Monitor port-linking helpers (ported from the session branch).
//
// The session manager routes capture streams to the default *source* (e.g. a
// microphone) regardless of `target.object`, so to monitor an output sink we
// must link its monitor ports to our analyzer inputs explicitly with
// `pw-link`, and destroy any foreign links the manager made meanwhile. These
// helpers scrape the registry for the needed ids/names and run the CLI tools
// while pumping our loop so daemon round-trips can complete.
// ---------------------------------------------------------------------------

/// Run `pw-link` while pumping our loop, so daemon round-trips that need our
/// own objects to answer can complete. Loop-thread only.
fn run_pw_link(
    mainloop: &MainLoopRc,
    args: &[&str],
    timeout: std::time::Duration,
) -> Option<std::process::Output> {
    use pipewire::loop_::Timeout;

    let owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
        let output = std::process::Command::new("pw-link")
            .args(&refs)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .ok();
        let _ = sender.send(output);
    });
    let deadline = std::time::Instant::now() + timeout;
    loop {
        match receiver.try_recv() {
            Ok(result) => return result,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => return None,
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }
        if std::time::Instant::now() >= deadline {
            warn!("pw-link timed out after {timeout:?}");
            return None;
        }
        mainloop
            .loop_()
            .iterate(Timeout::Finite(std::time::Duration::from_millis(20)));
    }
}

/// Run `pw-cli` with a timeout, returning `None` on spawn failure or timeout.
fn run_pw_cli(args: &[&str], timeout: std::time::Duration) -> Option<std::process::Output> {
    let child = std::process::Command::new("pw-cli")
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .ok()?;
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let result = child.wait_with_output();
        let _ = sender.send(result);
    });
    match receiver.recv_timeout(timeout) {
        Ok(Ok(output)) => Some(output),
        Ok(Err(e)) => {
            warn!("pw-cli failed: {e}");
            None
        }
        Err(_) => {
            warn!("pw-cli timed out after {timeout:?}, leaving child to exit");
            None
        }
    }
}

/// Run `pw-cli` while pumping our loop, for calls whose daemon round-trip
/// needs our own objects to answer (destroy involving our stream). Loop only.
fn run_pw_cli_pumped(
    mainloop: &MainLoopRc,
    args: &[&str],
    timeout: std::time::Duration,
) -> Option<std::process::Output> {
    use pipewire::loop_::Timeout;

    let owned: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let refs: Vec<&str> = owned.iter().map(|s| s.as_str()).collect();
        let result = run_pw_cli(&refs, timeout);
        let _ = sender.send(result);
    });
    let deadline = std::time::Instant::now() + timeout;
    loop {
        match receiver.try_recv() {
            Ok(result) => return result,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => return None,
            Err(std::sync::mpsc::TryRecvError::Empty) => {}
        }
        if std::time::Instant::now() >= deadline {
            return None;
        }
        mainloop
            .loop_()
            .iterate(Timeout::Finite(std::time::Duration::from_millis(20)));
    }
}

/// (node.name, id) records scraped from a registry snapshot. Loop-thread only:
/// pumps a few iterations so globals arrive before reading.
fn registry_node_names(mainloop: &MainLoopRc, core: &CoreRc) -> Result<Vec<(String, u32)>, Error> {
    use pipewire::loop_::Timeout;
    use std::sync::{Arc, Mutex};

    let names: Arc<Mutex<Vec<(String, u32)>>> = Arc::new(Mutex::new(Vec::new()));
    let names_clone = names.clone();
    let registry = core.get_registry()?;
    let _listener = registry
        .add_listener_local()
        .global(move |global| {
            if global.type_ == pipewire::types::ObjectType::Node {
                if let Some(props) = &global.props {
                    let name = props.get("node.name").unwrap_or("").to_string();
                    if !name.is_empty() {
                        names_clone.lock().unwrap().push((name, global.id));
                    }
                }
            }
        })
        .register();
    for _ in 0..5 {
        mainloop
            .loop_()
            .iterate(Timeout::Finite(std::time::Duration::from_millis(20)));
    }
    Ok(names.lock().unwrap().drain(..).collect())
}

/// Port record scraped from a registry snapshot.
struct PortRecord {
    node_id: u32,
    port_name: String,
    direction: String,
    path: String,
}

/// Snapshot ports from our registry. Loop-thread only.
fn registry_port_snapshot(mainloop: &MainLoopRc, core: &CoreRc) -> Result<Vec<PortRecord>, Error> {
    use pipewire::loop_::Timeout;
    use std::sync::{Arc, Mutex};

    let ports: Arc<Mutex<Vec<PortRecord>>> = Arc::new(Mutex::new(Vec::new()));
    let ports_clone = ports.clone();
    let registry = core.get_registry()?;
    let _listener = registry
        .add_listener_local()
        .global(move |global| {
            if global.type_ == pipewire::types::ObjectType::Port {
                if let Some(props) = &global.props {
                    let get = |k: &str| props.get(k).unwrap_or("").to_string();
                    if let (Some(node_id), Some(_port_id)) = (
                        get("node.id").parse::<u32>().ok(),
                        get("port.id").parse::<u32>().ok(),
                    ) {
                        ports_clone.lock().unwrap().push(PortRecord {
                            node_id,
                            port_name: get("port.name"),
                            direction: get("port.direction"),
                            path: get("object.path"),
                        });
                    }
                }
            }
        })
        .register();
    for _ in 0..5 {
        mainloop
            .loop_()
            .iterate(Timeout::Finite(std::time::Duration::from_millis(20)));
    }
    Ok(ports.lock().unwrap().drain(..).collect())
}

/// Link record scraped from a registry snapshot.
struct LinkRecord {
    id: u32,
    output_node: u32,
    input_node: u32,
}

/// Snapshot links from our registry. Loop-thread only.
fn registry_link_snapshot(mainloop: &MainLoopRc, core: &CoreRc) -> Result<Vec<LinkRecord>, Error> {
    use pipewire::loop_::Timeout;
    use std::sync::{Arc, Mutex};

    let links: Arc<Mutex<Vec<LinkRecord>>> = Arc::new(Mutex::new(Vec::new()));
    let links_clone = links.clone();
    let registry = core.get_registry()?;
    let _listener = registry
        .add_listener_local()
        .global(move |global| {
            if global.type_ == pipewire::types::ObjectType::Link {
                if let Some(props) = &global.props {
                    let get = |k: &str| props.get(k).unwrap_or("").to_string();
                    if let (Some(output_node), Some(input_node)) = (
                        get("link.output.node").parse::<u32>().ok(),
                        get("link.input.node").parse::<u32>().ok(),
                    ) {
                        links_clone.lock().unwrap().push(LinkRecord {
                            id: global.id,
                            output_node,
                            input_node,
                        });
                    }
                }
            }
        })
        .register();
    for _ in 0..5 {
        mainloop
            .loop_()
            .iterate(Timeout::Finite(std::time::Duration::from_millis(20)));
    }
    Ok(links.lock().unwrap().drain(..).collect())
}
