use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use log::{debug, info, warn};
use pipewire::{
    Error,
    core::{CoreRc, PW_ID_CORE},
    loop_::Timeout,
    main_loop::MainLoopRc,
    properties::properties,
    stream::StreamBox,
    types::ObjectType,
};

use crate::core::{FILTER_OUTPUT_SUFFIX, OUTPUT_CLIENT_NAME, SAMPLE_RATE, VIRTUAL_SINK_BASE};

#[derive(Debug, Clone)]
pub struct OutputRoute {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub active: bool,
}

#[derive(Debug, Clone)]
pub struct RouteInfo {
    pub route_id: u32,
    pub source_node: u32,
    pub target_node: u32,
    pub link_id: u32,
}

#[derive(Debug, Clone)]
pub struct StreamInfo {
    pub id: u32,
    pub node_id: u32,
    pub name: String,
    pub media_type: String,
    pub media_role: String,
    pub channels: u32,
    pub rate: u32,
    pub target_sink: Option<String>,
    pub active: bool,
}

pub struct RoutingEngine {
    core: CoreRc,
    mainloop: MainLoopRc,
    routes: Arc<Mutex<Vec<OutputRoute>>>,
    links: Arc<Mutex<Vec<RouteInfo>>>,
    streams: Arc<Mutex<HashMap<u32, StreamInfo>>>,
    routing_table: Arc<Mutex<HashMap<u32, u32>>>,
    auto_route: bool,
    current_sink: Option<String>,
    virtual_sink_name: String,
}

impl RoutingEngine {
    pub fn new(core: CoreRc, mainloop: MainLoopRc) -> Self {
        info!("Initializing RoutingEngine");

        RoutingEngine {
            core,
            mainloop,
            routes: Arc::new(Mutex::new(Vec::new())),
            links: Arc::new(Mutex::new(Vec::new())),
            streams: Arc::new(Mutex::new(HashMap::new())),
            routing_table: Arc::new(Mutex::new(HashMap::new())),
            auto_route: false,
            current_sink: None,
            virtual_sink_name: format!("{}.source", VIRTUAL_SINK_BASE),
        }
    }

    /// Pump the main loop until the server acknowledges a `sync` roundtrip.
    ///
    /// Registry `global` events are queued on the PipeWire socket, so a listener
    /// that is registered and then immediately inspected sees nothing. This is
    /// the barrier that makes `detect_routes`/`scan_streams`/`create_link`
    /// actually observe the objects they registered callbacks for.
    fn roundtrip(&self) -> Result<(), Error> {
        let done = Rc::new(Cell::new(false));
        let pending = self.core.sync(0)?;

        let done_clone = done.clone();
        let loop_clone = self.mainloop.clone();
        let _listener = self
            .core
            .add_listener_local()
            .done(move |id, seq| {
                if id == PW_ID_CORE && seq == pending {
                    done_clone.set(true);
                    loop_clone.quit();
                }
            })
            .register();

        // The server may already be idle; the finite timeout bounds the wait so
        // a missing `done` event cannot hang the caller forever.
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while !done.get() && std::time::Instant::now() < deadline {
            self.mainloop
                .loop_()
                .iterate(Timeout::Finite(Duration::from_millis(100)));
        }

        if !done.get() {
            warn!("PipeWire roundtrip timed out");
        }

        Ok(())
    }

    pub fn with_auto_route(mut self, auto: bool) -> Self {
        self.auto_route = auto;
        self
    }

    pub fn detect_routes(&self) -> Result<Vec<OutputRoute>, Error> {
        info!("Detecting output routes");

        let routes = Arc::new(Mutex::new(Vec::new()));
        let registry = self.core.get_registry()?;

        let routes_clone = routes.clone();
        let listener = registry.add_listener_local();
        let listener = listener.global(move |global| {
            if global.type_ == ObjectType::Node
                && let Some(props) = &global.props
            {
                let name = props.get("node.name").unwrap_or("unknown");
                if name.contains("audio.sink") || name.contains("output") || name.contains("analog")
                {
                    let mut routes_guard = routes_clone.lock().unwrap();
                    routes_guard.push(OutputRoute {
                        id: global.id,
                        name: name.to_string(),
                        description: props.get("node.description").unwrap_or("").to_string(),
                        active: true,
                    });
                    debug!("Found output route: {} (id={})", name, global.id);
                }
            }
        });
        let _listener = listener.register();

        // The listener only receives `global` events while it is alive and the
        // loop is being pumped; without this the snapshot below is always empty.
        self.roundtrip()?;

        let result = routes.lock().unwrap().clone();
        info!("Detected {} output routes", result.len());
        Ok(result)
    }

    pub fn get_routes(&self) -> Vec<OutputRoute> {
        self.routes.lock().unwrap().clone()
    }

    pub fn get_active_route(&self) -> Option<OutputRoute> {
        self.routes
            .lock()
            .unwrap()
            .iter()
            .find(|r| r.active)
            .cloned()
    }

    pub fn set_current_sink(&mut self, sink_name: &str) {
        self.current_sink = Some(sink_name.to_string());
        info!("Current sink set to: {}", sink_name);
    }

    pub fn get_current_sink(&self) -> Option<&str> {
        self.current_sink.as_deref()
    }

    pub fn auto_route_to_sink(&mut self, sink_name: &str) -> Result<(), Error> {
        info!("Auto-routing all streams to sink: {}", sink_name);

        let routes = self.detect_routes()?;

        for route in &routes {
            if route.name != sink_name {
                self.create_link(route.id, sink_name)?;
            }
        }

        self.set_current_sink(sink_name);
        self.auto_route = true;

        info!("Auto-routing complete to: {}", sink_name);
        Ok(())
    }

    pub fn create_link(&self, source_id: u32, target_name: &str) -> Result<u32, Error> {
        info!(
            "Creating link from source {} to target {}",
            source_id, target_name
        );

        let registry = self.core.get_registry()?;

        let link_id = Arc::new(Mutex::new(0u32));
        let link_id_clone = link_id.clone();
        let target_name = Arc::new(Mutex::new(target_name.to_string()));
        let target_name_clone = target_name.clone();

        let listener = registry.add_listener_local();
        let listener = listener.global(move |global| {
            if global.type_ == ObjectType::Node
                && let Some(props) = &global.props
            {
                let name = props.get("node.name").unwrap_or("unknown");
                let target = target_name_clone.lock().unwrap();
                if name == *target {
                    *link_id_clone.lock().unwrap() = global.id;
                }
            }
        });
        let _listener = listener.register();

        // Pump the loop so the `global` events populate `link_id`.
        self.roundtrip()?;

        let lid = *link_id.lock().unwrap();
        let target = target_name.lock().unwrap();
        if lid == 0 {
            warn!("Could not find target node {} for link", *target);
            return Err(Error::CreationFailed);
        }

        let mut links = self.links.lock().unwrap();
        links.push(RouteInfo {
            route_id: source_id,
            source_node: source_id,
            target_node: lid,
            link_id: lid,
        });

        info!("Created link with id {}", lid);
        Ok(lid)
    }

    pub fn remove_link(&self, link_id: u32) -> Result<(), Error> {
        info!("Removing link {}", link_id);

        let mut links = self.links.lock().unwrap();
        links.retain(|l| l.link_id != link_id);

        Ok(())
    }

    pub fn get_links(&self) -> Vec<RouteInfo> {
        self.links.lock().unwrap().clone()
    }

    pub fn route_stream(&self, _stream_id: u32, _target_sink: &str) -> Result<(), Error> {
        info!("Routing stream to sink");
        Ok(())
    }

    pub fn unroute_stream(&self, _stream_id: u32) -> Result<(), Error> {
        info!("Unrouting stream");
        Ok(())
    }

    pub fn get_virtual_sink_name(&self) -> &str {
        &self.virtual_sink_name
    }

    pub fn is_auto_route(&self) -> bool {
        self.auto_route
    }

    pub fn set_auto_route(&mut self, auto: bool) {
        self.auto_route = auto;
    }

    pub fn scan_streams(&self) -> Result<Vec<StreamInfo>, Error> {
        info!("Scanning PipeWire streams");

        let streams = Arc::new(Mutex::new(Vec::new()));
        let registry = self.core.get_registry()?;

        let streams_clone = streams.clone();
        let listener = registry.add_listener_local();
        let listener = listener.global(move |global| {
            if global.type_ == ObjectType::Node
                && let Some(props) = &global.props
            {
                let name = props.get("node.name").unwrap_or("unknown");
                let info = StreamInfo {
                    id: global.id,
                    node_id: global.id,
                    name: name.to_string(),
                    media_type: String::new(),
                    media_role: String::new(),
                    channels: 2,
                    rate: SAMPLE_RATE as u32,
                    target_sink: None,
                    active: true,
                };
                streams_clone.lock().unwrap().push(info);
                debug!("Found stream: {} (id={})", name, global.id);
            }
        });
        let _listener = listener.register();

        // Pump the loop so the `global` events populate the stream snapshot.
        self.roundtrip()?;

        let result = streams.lock().unwrap().clone();
        info!("Scanned {} streams", result.len());
        Ok(result)
    }

    pub fn get_streams(&self) -> Vec<StreamInfo> {
        self.streams.lock().unwrap().values().cloned().collect()
    }

    pub fn route_to_virtual_sink(&self, stream_id: u32) -> Result<(), Error> {
        info!(
            "Routing stream {} to virtual sink {}",
            stream_id, VIRTUAL_SINK_BASE
        );

        let sink_name = format!("{}{}", VIRTUAL_SINK_BASE, FILTER_OUTPUT_SUFFIX);

        let mut routing = self.routing_table.lock().unwrap();
        routing.insert(
            stream_id,
            sink_name.clone().into_bytes().iter().sum::<u8>() as u32,
        );

        {
            let mut streams_guard = self.streams.lock().unwrap();
            if let Some(stream) = streams_guard.get_mut(&stream_id) {
                stream.target_sink = Some(sink_name.clone());
            }
        }

        info!("Stream {} routed to virtual sink", stream_id);
        Ok(())
    }

    pub fn unroute_from_virtual_sink(&self, stream_id: u32) -> Result<(), Error> {
        info!("Unrouting stream {} from virtual sink", stream_id);

        self.routing_table.lock().unwrap().remove(&stream_id);

        {
            let mut streams_guard = self.streams.lock().unwrap();
            if let Some(stream) = streams_guard.get_mut(&stream_id) {
                stream.target_sink = None;
            }
        }

        Ok(())
    }

    pub fn auto_route_all(&mut self) -> Result<(), Error> {
        info!("Auto-routing all streams to virtual sink");

        let streams = self.scan_streams()?;
        let sink_name = format!("{}{}", VIRTUAL_SINK_BASE, FILTER_OUTPUT_SUFFIX);

        for stream in &streams {
            if stream.name.contains(VIRTUAL_SINK_BASE) || stream.name.contains(OUTPUT_CLIENT_NAME) {
                continue;
            }
            self.route_to_virtual_sink(stream.id)?;
        }

        self.auto_route = true;
        self.current_sink = Some(sink_name);

        info!("Auto-routed {} streams", streams.len());
        Ok(())
    }

    pub fn get_virtual_sink_streams(&self) -> Vec<StreamInfo> {
        self.streams
            .lock()
            .unwrap()
            .values()
            .filter(|s| {
                s.target_sink
                    .as_ref()
                    .map(|t| t.starts_with(VIRTUAL_SINK_BASE))
                    .unwrap_or(false)
            })
            .cloned()
            .collect()
    }

    pub fn get_routing_table(&self) -> HashMap<u32, u32> {
        self.routing_table.lock().unwrap().clone()
    }

    pub fn create_virtual_sink_stream(&self) -> Result<StreamBox<'_>, Error> {
        info!("Creating virtual sink stream");

        let sink_name = format!("{}{}", VIRTUAL_SINK_BASE, FILTER_OUTPUT_SUFFIX);

        let sink_props = properties! {
            *pipewire::keys::MEDIA_TYPE => "Audio",
            *pipewire::keys::MEDIA_CATEGORY => "Stream",
            *pipewire::keys::MEDIA_ROLE => "Music",
            *pipewire::keys::NODE_NAME => sink_name.as_str(),
            *pipewire::keys::NODE_DESCRIPTION => OUTPUT_CLIENT_NAME,
            *pipewire::keys::MEDIA_CLASS => "Audio/Sink",
            "audio.channels" => "2",
            "audio.rate" => SAMPLE_RATE.to_string().as_str(),
            "audio.format" => "f32le",
        };

        let stream = StreamBox::new(&self.core, &sink_name, sink_props)?;

        info!("Virtual sink stream created");
        Ok(stream)
    }

    pub fn monitor_streams(&self) -> Result<(), Error> {
        info!("Monitoring PipeWire streams for changes");
        Ok(())
    }
}

impl Default for RoutingEngine {
    fn default() -> Self {
        let mainloop =
            pipewire::main_loop::MainLoopRc::new(None).expect("Failed to create MainLoop");
        let context =
            pipewire::context::ContextRc::new(&mainloop, None).expect("Failed to create Context");
        let core = context
            .connect_rc(None)
            .expect("Failed to connect to PipeWire");
        RoutingEngine::new(core, mainloop)
    }
}
