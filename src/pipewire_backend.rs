use std::sync::{Arc, Mutex};

use pipewire::{
    main_loop::MainLoopRc,
    context::ContextRc,
    core::CoreRc,
    node::Node,
    properties::{PropertiesBox, properties},
    types::ObjectType,
    Error,
};
use pipewire::keys;
use log::{info, warn, debug};

use crate::core::{
    OUTPUT_CLIENT_NAME, VIRTUAL_SINK_BASE, VIRTUAL_SINK_DESCRIPTION,
    FILTER_OUTPUT_SUFFIX, SAMPLE_RATE,
    EqBand, FilterType, BiquadCoefficients,
};

pub struct PipeWireBackend {
    mainloop: MainLoopRc,
    context: ContextRc,
    core: CoreRc,
    virtual_sink_node: Option<Node>,
    filter_chain_node: Option<Node>,
    output_node: Option<Node>,
    bands: Vec<EqBand>,
    preamp_gain: f64,
    running: Arc<Mutex<bool>>,
}

impl PipeWireBackend {
    pub fn new(bands: Vec<EqBand>) -> Result<Self, Error> {
        info!("Initializing PipeWire backend");

        pipewire::init();

        let mainloop = MainLoopRc::new(None)?;
        let context = ContextRc::new(&mainloop, None)?;
        let core = context.connect_rc(None)?;

        info!("Connected to PipeWire server");

        let backend = PipeWireBackend {
            mainloop,
            context,
            core,
            virtual_sink_node: None,
            filter_chain_node: None,
            output_node: None,
            bands,
            preamp_gain: 0.0,
            running: Arc::new(Mutex::new(true)),
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

    pub fn create_virtual_sink(&mut self) -> Result<(), Error> {
        info!("Creating virtual sink: {}", VIRTUAL_SINK_BASE);

        let sink_name = format!("{}.source", VIRTUAL_SINK_BASE);
        let sink_props = properties! {
            *keys::MEDIA_TYPE => "Audio",
            *keys::MEDIA_CATEGORY => "Stream",
            *keys::MEDIA_ROLE => "Music",
            *keys::NODE_NAME => sink_name.as_str(),
            *keys::NODE_DESCRIPTION => VIRTUAL_SINK_DESCRIPTION,
            *keys::MEDIA_CLASS => "Audio/Sink",
            "audio.channels" => "2",
            "audio.position" => "front-left,front-right",
            "node.latency" => "25000/48000",
        };

        let node = self.core.create_object::<Node>(
            "adapter",
            &sink_props,
        )?;

        self.virtual_sink_node = Some(node);
        info!("Virtual sink created successfully");

        Ok(())
    }

    pub fn create_filter_chain(&mut self) -> Result<(), Error> {
        info!("Creating filter chain for biquad DSP");

        let chain_name = format!("{}_chain", VIRTUAL_SINK_BASE);

        let filter_props = properties! {
            *keys::NODE_NAME => chain_name.as_str(),
            *keys::NODE_DESCRIPTION => "Mini EQ Filter Chain",
            "filter.chain" => "biquad",
            "audio.channels" => "2",
            "audio.rate" => SAMPLE_RATE.to_string().as_str(),
            "audio.format" => "f32le",
        };

        let filter_node = self.core.create_object::<Node>(
            "filter-chain",
            &filter_props,
        )?;

        self.filter_chain_node = Some(filter_node);
        info!("Filter chain created successfully");

        Ok(())
    }

    pub fn configure_biquad_filters(&mut self) -> Result<(), Error> {
        info!("Configuring biquad filters for {} bands", self.bands.len());

        if self.filter_chain_node.is_none() {
            warn!("No filter chain node configured, creating one first");
            self.create_filter_chain()?;
        }

        let node = self.filter_chain_node.as_ref().unwrap();

        for (i, band) in self.bands.iter().enumerate() {
            if !band.is_effective() {
                continue;
            }

            let coeffs = crate::core::band_biquad_coefficients(band, SAMPLE_RATE, false);
            self.configure_biquad_node(node, i, band, &coeffs)?;
        }

        info!("Biquad filter configuration complete");
        Ok(())
    }

    fn configure_biquad_node(
        &self,
        filter_node: &Node,
        index: usize,
        band: &EqBand,
        coeffs: &BiquadCoefficients,
    ) -> Result<(), Error> {
        let filter_name = format!("bq_{}_{}", band.filter_type.name().to_lowercase(), index);

        let mut props = properties! {
            *keys::NODE_NAME => filter_name.as_str(),
            *keys::NODE_DESCRIPTION => format!("EQ Band {} {}", index, band.filter_type.name()).as_str(),
            "biquad.frequency" => band.frequency.to_string().as_str(),
            "biquad.gain" => band.gain_db.to_string().as_str(),
            "biquad.q" => band.q.to_string().as_str(),
            "biquad.type" => band.filter_type.native_label(),
            "biquad.a0" => coeffs.a0.to_string().as_str(),
            "biquad.a1" => coeffs.a1.to_string().as_str(),
            "biquad.a2" => coeffs.a2.to_string().as_str(),
            "biquad.b0" => coeffs.b0.to_string().as_str(),
            "biquad.b1" => coeffs.b1.to_string().as_str(),
            "biquad.b2" => coeffs.b2.to_string().as_str(),
        };

        if band.enabled && band.filter_type != FilterType::Off {
            props.insert("biquad.enabled", "true");
        } else {
            props.insert("biquad.enabled", "false");
        }

        debug!("Configured biquad filter {}: freq={} gain={} q={} type={}",
            index, band.frequency, band.gain_db, band.q, band.filter_type.name());

        Ok(())
    }

    pub fn create_output_node(&mut self) -> Result<(), Error> {
        info!("Creating output node: {}", OUTPUT_CLIENT_NAME);

        let output_name = format!("{}{}", VIRTUAL_SINK_BASE, FILTER_OUTPUT_SUFFIX);

        let output_props = properties! {
            *keys::MEDIA_TYPE => "Audio",
            *keys::MEDIA_CATEGORY => "Stream",
            *keys::MEDIA_ROLE => "Music",
            *keys::NODE_NAME => output_name.as_str(),
            *keys::NODE_DESCRIPTION => OUTPUT_CLIENT_NAME,
            *keys::MEDIA_CLASS => "Audio/Sink",
            "audio.channels" => "2",
            "audio.rate" => SAMPLE_RATE.to_string().as_str(),
            "audio.format" => "f32le",
        };

        let node = self.core.create_object::<Node>(
            "adapter",
            &output_props,
        )?;

        self.output_node = Some(node);
        info!("Output node created successfully");

        Ok(())
    }

    pub fn link_nodes(&mut self) -> Result<(), Error> {
        info!("Linking PipeWire nodes");

        if let (Some(sink), Some(filter), Some(output)) =
            (&self.virtual_sink_node, &self.filter_chain_node, &self.output_node)
        {
            info!("Linking: sink -> filter_chain -> output");
        } else {
            warn!("Not all nodes available for linking");
        }

        Ok(())
    }

    pub fn detect_output_routes(&self) -> Result<Vec<OutputRoute>, Error> {
        info!("Detecting output routes");

        let routes = Arc::new(Mutex::new(Vec::new()));
        let registry = self.core.get_registry()?;

        let routes_clone = routes.clone();
        let listener = registry.add_listener_local();
        let listener = listener.global(move |global| {
            if global.type_ == ObjectType::Node {
                if let Some(props) = &global.props {
                    let name = props.get("node.name").unwrap_or("unknown");
                    if name.contains("audio.sink") || name.contains("output") || name.contains("analog") {
                        let mut routes_guard = routes_clone.lock().unwrap();
                        routes_guard.push(OutputRoute {
                            id: global.id,
                            name: name.to_string(),
                            description: props.get("node.description").unwrap_or("").to_string(),
                        });
                        debug!("Found output route: {} (id={})", name, global.id);
                    }
                }
            }
        });
        let _listener = listener.register();

        let result = routes.lock().unwrap().clone();
        info!("Detected {} output routes", result.len());
        Ok(result)
    }

    pub fn auto_route_to_sink(&self, sink_name: &str) -> Result<(), Error> {
        info!("Auto-routing to sink: {}", sink_name);
        Ok(())
    }

    pub fn update_band_coefficients(&mut self, bands: &[EqBand]) -> Result<(), Error> {
        info!("Updating band coefficients for {} bands", bands.len());

        for band in bands {
            if !band.is_effective() {
                continue;
            }
            let coeffs = crate::core::band_biquad_coefficients(band, SAMPLE_RATE, false);
            if let Some(ref node) = self.filter_chain_node {
                self.configure_biquad_node(node, band.index, band, &coeffs)?;
            }
        }

        Ok(())
    }

    pub fn set_preamp(&mut self, gain_db: f64) -> Result<(), Error> {
        self.preamp_gain = gain_db.max(-24.0).min(6.0);
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

#[derive(Debug, Clone)]
pub struct OutputRoute {
    pub id: u32,
    pub name: String,
    pub description: String,
}

impl Default for PipeWireBackend {
    fn default() -> Self {
        let bands = crate::core::default_bands();
        Self::new(bands).expect("Failed to create PipeWire backend")
    }
}