use std::sync::{Arc, Mutex};

use log::{debug, info, warn};
use pipewire::{
    Error, context::ContextRc, core::CoreRc, link::Link, main_loop::MainLoopRc, node::Node,
    properties::properties, proxy::ProxyT,
};

use crate::core::{
    BiquadCoefficients, EqBand, FILTER_OUTPUT_SUFFIX, FilterType, OUTPUT_CLIENT_NAME, SAMPLE_RATE,
    VIRTUAL_SINK_BASE, VIRTUAL_SINK_DESCRIPTION,
};
use crate::routing::{OutputRoute, RoutingEngine};

pub struct PipeWireBackend {
    mainloop: MainLoopRc,
    _context: ContextRc,
    core: CoreRc,
    virtual_sink_node: Option<Node>,
    filter_chain_node: Option<Node>,
    output_node: Option<Node>,
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

        let routing = RoutingEngine::new(core.clone());

        let backend = PipeWireBackend {
            mainloop,
            _context: context,
            core,
            virtual_sink_node: None,
            filter_chain_node: None,
            output_node: None,
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

    pub fn create_virtual_sink(&mut self) -> Result<(), Error> {
        info!("Creating virtual sink: {}", VIRTUAL_SINK_BASE);

        let sink_name = format!("{}.source", VIRTUAL_SINK_BASE);
        let sink_props = properties! {
            *pipewire::keys::MEDIA_TYPE => "Audio",
            *pipewire::keys::MEDIA_CATEGORY => "Stream",
            *pipewire::keys::MEDIA_ROLE => "Music",
            *pipewire::keys::NODE_NAME => sink_name.as_str(),
            *pipewire::keys::NODE_DESCRIPTION => VIRTUAL_SINK_DESCRIPTION,
            *pipewire::keys::MEDIA_CLASS => "Audio/Sink",
            "audio.channels" => "2",
            "audio.position" => "front-left,front-right",
            "node.latency" => "25000/48000",
        };

        let node = self.core.create_object::<Node>("adapter", &sink_props)?;

        self.virtual_sink_node = Some(node);
        info!("Virtual sink created successfully");

        Ok(())
    }

    pub fn create_filter_chain(&mut self) -> Result<(), Error> {
        info!("Creating filter chain for biquad DSP");

        let chain_name = format!("{}_chain", VIRTUAL_SINK_BASE);

        let filter_props = properties! {
            *pipewire::keys::NODE_NAME => chain_name.as_str(),
            *pipewire::keys::NODE_DESCRIPTION => "Mini EQ Filter Chain",
            "filter.chain" => "biquad",
            "audio.channels" => "2",
            "audio.rate" => SAMPLE_RATE.to_string().as_str(),
            "audio.format" => "f32le",
        };

        let filter_node = self
            .core
            .create_object::<Node>("filter-chain", &filter_props)?;

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
            *pipewire::keys::NODE_NAME => filter_name.as_str(),
            *pipewire::keys::NODE_DESCRIPTION => format!("EQ Band {} {}", index, band.filter_type.name()).as_str(),
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

        let properties: Vec<pipewire::spa::pod::Property> = props
            .dict()
            .iter()
            .filter_map(|(key, value)| {
                value
                    .parse::<f64>()
                    .ok()
                    .map(|val| pipewire::spa::pod::Property {
                        key: Self::string_key_to_id(key),
                        flags: pipewire::spa::pod::PropertyFlags::empty(),
                        value: pipewire::spa::pod::Value::Double(val),
                    })
            })
            .collect();

        let pod_value = pipewire::spa::pod::Value::Object(pipewire::spa::pod::Object {
            type_: pipewire::spa::utils::SpaTypes::ObjectParamProps.as_raw(),
            id: pipewire::spa::param::ParamType::Props.as_raw(),
            properties,
        });

        let pod_bytes = pipewire::spa::pod::serialize::PodSerializer::serialize(
            std::io::Cursor::new(Vec::new()),
            &pod_value,
        )
        .map(|(cursor, _)| cursor.into_inner())
        .unwrap_or_default();

        let pod = pipewire::spa::pod::Pod::from_bytes(&pod_bytes)
            .expect("Failed to create Pod from bytes");

        filter_node.set_param(pipewire::spa::param::ParamType::Props, 0, pod);

        debug!(
            "Configured biquad filter {}: freq={} gain={} q={} type={}",
            index,
            band.frequency,
            band.gain_db,
            band.q,
            band.filter_type.name()
        );

        Ok(())
    }

    fn string_key_to_id(key: &str) -> u32 {
        let mut hash: u32 = 0;
        for byte in key.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as u32);
        }
        hash
    }

    pub fn create_output_node(&mut self) -> Result<(), Error> {
        info!("Creating output node: {}", OUTPUT_CLIENT_NAME);

        let output_name = format!("{}{}", VIRTUAL_SINK_BASE, FILTER_OUTPUT_SUFFIX);

        let output_props = properties! {
            *pipewire::keys::MEDIA_TYPE => "Audio",
            *pipewire::keys::MEDIA_CATEGORY => "Stream",
            *pipewire::keys::MEDIA_ROLE => "Music",
            *pipewire::keys::NODE_NAME => output_name.as_str(),
            *pipewire::keys::NODE_DESCRIPTION => OUTPUT_CLIENT_NAME,
            *pipewire::keys::MEDIA_CLASS => "Audio/Sink",
            "audio.channels" => "2",
            "audio.rate" => SAMPLE_RATE.to_string().as_str(),
            "audio.format" => "f32le",
        };

        let node = self.core.create_object::<Node>("adapter", &output_props)?;

        self.output_node = Some(node);
        info!("Output node created successfully");

        Ok(())
    }

    pub fn link_nodes(&mut self) -> Result<(), Error> {
        info!("Linking PipeWire nodes");

        let sink_id = self
            .virtual_sink_node
            .as_ref()
            .map(|n| n.upcast_ref().id())
            .ok_or_else(|| {
                warn!("Virtual sink node not found for linking");
                Error::CreationFailed
            })?;
        let filter_id = self
            .filter_chain_node
            .as_ref()
            .map(|n| n.upcast_ref().id())
            .ok_or_else(|| {
                warn!("Filter chain node not found for linking");
                Error::CreationFailed
            })?;
        let output_id = self
            .output_node
            .as_ref()
            .map(|n| n.upcast_ref().id())
            .ok_or_else(|| {
                warn!("Output node not found for linking");
                Error::CreationFailed
            })?;

        info!(
            "Found nodes: sink={}, filter={}, output={}",
            sink_id, filter_id, output_id
        );

        let _ = self.core.create_object::<Link>(
            "link-factory",
            &properties! {
                "link.output.port" => "0",
                "link.input.port" => "0",
                "link.output.node" => sink_id.to_string().as_str(),
                "link.input.node" => filter_id.to_string().as_str(),
            },
        );

        let _ = self.core.create_object::<Link>(
            "link-factory",
            &properties! {
                "link.output.port" => "0",
                "link.input.port" => "0",
                "link.output.node" => filter_id.to_string().as_str(),
                "link.input.node" => output_id.to_string().as_str(),
            },
        );

        info!("Linked sink -> filter_chain -> output");
        Ok(())
    }

    pub fn detect_output_routes(&self) -> Result<Vec<OutputRoute>, Error> {
        self.routing.detect_routes()
    }

    pub fn auto_route_to_sink(&mut self, sink_name: &str) -> Result<(), Error> {
        self.routing.auto_route_to_sink(sink_name)
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
