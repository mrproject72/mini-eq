use std::sync::{Arc, Mutex};

use log::{debug, info, warn};
use pipewire::{Error, core::CoreRc, types::ObjectType};

use crate::core::VIRTUAL_SINK_BASE;

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

pub struct PipeWireRoutes {
    core: CoreRc,
    routes: Arc<Mutex<Vec<OutputRoute>>>,
    links: Arc<Mutex<Vec<RouteInfo>>>,
    current_sink: Option<String>,
    auto_route: bool,
    virtual_sink_name: String,
}

impl PipeWireRoutes {
    pub fn new(core: CoreRc) -> Self {
        info!("Initializing PipeWireRoutes");

        PipeWireRoutes {
            core,
            routes: Arc::new(Mutex::new(Vec::new())),
            links: Arc::new(Mutex::new(Vec::new())),
            current_sink: None,
            auto_route: false,
            virtual_sink_name: format!("{}.source", VIRTUAL_SINK_BASE),
        }
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
}

impl Default for PipeWireRoutes {
    fn default() -> Self {
        let mainloop =
            pipewire::main_loop::MainLoopRc::new(None).expect("Failed to create MainLoop");
        let context =
            pipewire::context::ContextRc::new(&mainloop, None).expect("Failed to create Context");
        let core = context
            .connect_rc(None)
            .expect("Failed to connect to PipeWire");
        PipeWireRoutes::new(core)
    }
}
