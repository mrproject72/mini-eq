use std::collections::HashMap;

use pipewire::{
    core::CoreRc,
    node::Node,
    properties::properties,
    Error,
};
use pipewire::keys;
use log::info;

use crate::core::{
    SAMPLE_RATE,
    EqBand, FilterType, BiquadCoefficients,
};

pub struct FilterChain {
    core: CoreRc,
    filter_node: Option<Node>,
    bands: Vec<EqBand>,
    preamp_gain: f64,
    filter_graph: HashMap<u32, FilterNode>,
    enabled: bool,
}

pub struct FilterNode {
    _id: u32,
    filter_type: FilterType,
    frequency: f64,
    gain_db: f64,
    q: f64,
    enabled: bool,
    coefficients: BiquadCoefficients,
    _node: Option<Node>,
}

impl FilterChain {
    pub fn new(core: CoreRc) -> Self {
        info!("Initializing FilterChain");

        FilterChain {
            core,
            filter_node: None,
            bands: Vec::new(),
            preamp_gain: 0.0,
            filter_graph: HashMap::new(),
            enabled: false,
        }
    }

    pub fn with_bands(mut self, bands: Vec<EqBand>) -> Self {
        self.bands = bands;
        self
    }

    pub fn create(&mut self) -> Result<(), Error> {
        info!("Creating PipeWire filter chain");

        let filter_props = properties! {
            *keys::NODE_NAME => "mini_eq_filter_chain",
            *keys::NODE_DESCRIPTION => "Mini EQ Filter Chain",
            "filter.chain" => "biquad",
            "audio.channels" => "2",
            "audio.rate" => SAMPLE_RATE.to_string().as_str(),
            "audio.format" => "f32le",
        };

        let filter_node = self.core.create_object::<Node>("filter-chain", &filter_props)?;
        self.filter_node = Some(filter_node);
        self.enabled = true;

        info!("Filter chain created");
        Ok(())
    }

    pub fn configure_bands(&mut self, bands: &[EqBand]) -> Result<(), Error> {
        info!("Configuring {} bands in filter chain", bands.len());

        self.bands = bands.to_vec();
        self.filter_graph.clear();

        for band in bands {
            if !band.is_effective() {
                continue;
            }

            let coeffs = crate::core::band_biquad_coefficients(band, SAMPLE_RATE, false);
            let filter_id = band.index as u32;

            let filter_node_info = FilterNode {
                _id: filter_id,
                filter_type: band.filter_type,
                frequency: band.frequency,
                gain_db: band.gain_db,
                q: band.q,
                enabled: band.enabled,
                coefficients: coeffs,
                _node: None,
            };

            self.filter_graph.insert(filter_id, filter_node_info);
        }

        info!("Configured {} active filter nodes", self.filter_graph.len());
        Ok(())
    }

    pub fn set_preamp(&mut self, gain_db: f64) -> Result<(), Error> {
        self.preamp_gain = gain_db.clamp(-24.0, 6.0);
        info!("Filter chain preamp set to {} dB", self.preamp_gain);
        Ok(())
    }

    pub fn get_preamp(&self) -> f64 {
        self.preamp_gain
    }

    pub fn update_coefficients(&mut self, bands: &[EqBand]) -> Result<(), Error> {
        for band in bands {
            if let Some(filter) = self.filter_graph.get_mut(&(band.index as u32)) {
                filter.coefficients = crate::core::band_biquad_coefficients(band, SAMPLE_RATE, false);
                filter.frequency = band.frequency;
                filter.gain_db = band.gain_db;
                filter.q = band.q;
                filter.filter_type = band.filter_type;
                filter.enabled = band.enabled;
            }
        }
        Ok(())
    }

    pub fn enable_filter(&mut self, band_index: usize, enabled: bool) -> Result<(), Error> {
        if let Some(filter) = self.filter_graph.get_mut(&(band_index as u32)) {
            filter.enabled = enabled;
            info!("Band {} filter {}", band_index, if enabled { "enabled" } else { "disabled" });
        }
        Ok(())
    }

    pub fn get_active_filters(&self) -> Vec<&FilterNode> {
        self.filter_graph.values().filter(|f| f.enabled).collect()
    }

    pub fn get_band(&self, index: usize) -> Option<&FilterNode> {
        self.filter_graph.get(&(index as u32))
    }

    pub fn get_all_bands(&self) -> &HashMap<u32, FilterNode> {
        &self.filter_graph
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    pub fn destroy(&mut self) -> Result<(), Error> {
        if let Some(node) = self.filter_node.take() {
            self.core.destroy_object(node)?;
        }
        self.filter_graph.clear();
        self.enabled = false;
        info!("Filter chain destroyed");
        Ok(())
    }
}

impl Default for FilterChain {
    fn default() -> Self {
        let mainloop = pipewire::main_loop::MainLoopRc::new(None).expect("Failed to create MainLoop");
        let context = pipewire::context::ContextRc::new(&mainloop, None).expect("Failed to create Context");
        let core = context.connect_rc(None).expect("Failed to connect to PipeWire");
        FilterChain::new(core)
    }
}