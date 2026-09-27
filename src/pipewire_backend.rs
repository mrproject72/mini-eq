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
    /// Kept alive so the registry `global` listener stays registered for the
    /// backend's lifetime (listeners unregister themselves when dropped).
    _registry_listener: Option<pipewire::registry::Listener>,
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
            _registry_listener: None,
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
                // SPA_Props_params == 0x80000 (SPA StartOther base + Params).
                builder
                    .add_prop(0x80000, 0)
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
    pub fn update_state_live_or_reload(&mut self, output_sink: &str) -> Result<(), Error> {
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
    /// detected route. Used as the filter-chain playback target.
    pub fn default_output_sink(&self) -> Option<String> {
        self.routing.detect_routes().ok().and_then(|routes| {
            routes
                .iter()
                .find(|r| r.active)
                .or_else(|| routes.first())
                .map(|r| r.name.clone())
        })
    }

    pub fn auto_route_to_sink(&mut self, sink_name: &str) -> Result<(), Error> {
        self.routing.auto_route_to_sink(sink_name)
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
