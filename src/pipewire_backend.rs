use std::ffi::CString;
use std::sync::{Arc, Mutex};

use log::{debug, info, warn};
use pipewire::{Error, context::ContextRc, core::CoreRc, main_loop::MainLoopRc};
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
        };

        backend.setup_registry_listener()?;

        Ok(backend)
    }

    fn setup_registry_listener(&self) -> Result<(), Error> {
        let registry = self.core.get_registry()?;
        let listener = registry.add_listener_local();
        let listener = listener.global(|global| {
            debug!("Registry global: id={} type={:?}", global.id, global.type_);
        });
        let _listener = listener.register();
        Ok(())
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
            // SAFETY: `handle.0` came from `pw_context_load_module` and is
            // destroyed exactly once, here.
            unsafe { pw_sys::pw_impl_module_destroy(handle.0) };
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
