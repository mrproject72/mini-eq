//! High-level routing logic for mini-eq.
//!
//! Manages stream routing decisions and output device selection.

use anyhow::Context;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::EqState;
use crate::pipewire_stream_router::StreamRouter;

/// Routing manager for audio streams.
pub struct RoutingManager {
    /// Shared EQ state.
    eq_state: Arc<RwLock<EqState>>,
    /// Stream router instance.
    router: Option<StreamRouter>,
    /// Whether auto-routing is enabled.
    auto_route_enabled: bool,
}

impl RoutingManager {
    /// Create a new routing manager.
    pub fn new(eq_state: Arc<RwLock<EqState>>) -> Self {
        Self {
            eq_state,
            router: None,
            auto_route_enabled: false,
        }
    }

    /// Initialize the routing manager.
    pub async fn init(&mut self) -> anyhow::Result<()> {
        log::info!("Initializing routing manager...");

        let router = StreamRouter::new()
            .context("Failed to create stream router")?;

        self.router = Some(router);
        log::info!("Routing manager initialized");
        Ok(())
    }

    /// Enable or disable auto-routing.
    pub fn set_auto_route(&mut self, enabled: bool) {
        self.auto_route_enabled = enabled;
        log::info!("Auto-routing {}", if enabled { "enabled" } else { "disabled" });
    }

    /// Check if auto-routing is enabled.
    pub fn is_auto_route_enabled(&self) -> bool {
        self.auto_route_enabled
    }

    /// Route all streams to the virtual sink.
    pub async fn route_all_to_sink(&mut self) -> anyhow::Result<()> {
        if let Some(ref mut router) = self.router {
            router.route_all_streams().await?;
            log::info!("All streams routed to virtual sink");
        } else {
            anyhow::bail!("Router not initialized");
        }
        Ok(())
    }

    /// Route a specific stream by ID.
    pub async fn route_stream(&mut self, stream_id: u32) -> anyhow::Result<()> {
        if let Some(ref mut router) = self.router {
            router.route_stream(stream_id).await?;
            log::info!("Stream {} routed to virtual sink", stream_id);
        } else {
            anyhow::bail!("Router not initialized");
        }
        Ok(())
    }

    /// Unroute a specific stream.
    pub async fn unroute_stream(&mut self, stream_id: u32) -> anyhow::Result<()> {
        if let Some(ref mut router) = self.router {
            router.unroute_stream(stream_id).await?;
            log::info!("Stream {} unrouted", stream_id);
        } else {
            anyhow::bail!("Router not initialized");
        }
        Ok(())
    }

    /// Get list of routable streams.
    pub async fn get_routable_streams(&self) -> Vec<String> {
        if let Some(ref router) = self.router {
            router.get_routable_streams().await
        } else {
            vec![]
        }
    }

    /// Restore stream routes from preset.
    pub async fn restore_routes(&mut self, routes: &[(u32, String)]) -> anyhow::Result<()> {
        if let Some(ref mut router) = self.router {
            for (stream_id, target) in routes {
                if let Err(e) = router.route_stream_to_target(*stream_id, target).await {
                    log::warn!("Failed to restore route for stream {}: {}", stream_id, e);
                }
            }
        }
        Ok(())
    }

    /// Clear all routes.
    pub async fn clear_all_routes(&mut self) -> anyhow::Result<()> {
        if let Some(ref mut router) = self.router {
            router.clear_all_routes().await?;
            log::info!("All routes cleared");
        }
        Ok(())
    }
}

impl Default for RoutingManager {
    fn default() -> Self {
        Self::new(Arc::new(RwLock::new(EqState::default())))
    }
}