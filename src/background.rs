//! Background daemon mode for mini-eq.
//!
//! Allows running the equalizer without a GUI, controlled via D-Bus or CLI.

use anyhow::Context;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::core::EqState;
use crate::pipewire_backend::PipeWireBackend;
use crate::settings::Settings;

/// Background daemon instance.
pub struct Daemon {
    /// Shared EQ state.
    eq_state: Arc<RwLock<EqState>>,
    /// PipeWire backend handle.
    backend: Option<PipeWireBackend>,
    /// Whether the daemon is running.
    running: bool,
}

impl Daemon {
    /// Create a new daemon instance.
    pub fn new() -> Self {
        Self {
            eq_state: Arc::new(RwLock::new(EqState::default())),
            backend: None,
            running: false,
        }
    }

    /// Initialize the daemon with settings.
    pub async fn init(&mut self, settings: &Settings) -> anyhow::Result<()> {
        log::info!("Initializing background daemon...");

        // Load EQ state from settings
        {
            let mut state = self.eq_state.write().await;
            state.preamp_gain_db = settings.preamp_gain_db;
            state.enabled = settings.eq_enabled;
        }

        // Initialize PipeWire backend
        let backend = PipeWireBackend::new()
            .context("Failed to create PipeWire backend")?;
        
        self.backend = Some(backend);
        self.running = true;

        log::info!("Background daemon initialized");
        Ok(())
    }

    /// Run the daemon main loop.
    pub async fn run(&mut self) -> anyhow::Result<()> {
        if !self.running {
            anyhow::bail!("Daemon not initialized");
        }

        log::info!("Starting daemon main loop...");

        // Main loop - in background mode we just keep the EQ active
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            
            // Check for shutdown signals or D-Bus commands here
            // For now, just keep running
        }
    }

    /// Get a clone of the shared EQ state.
    pub fn eq_state(&self) -> Arc<RwLock<EqState>> {
        self.eq_state.clone()
    }

    /// Stop the daemon.
    pub fn stop(&mut self) {
        log::info!("Stopping background daemon...");
        self.running = false;
        
        if let Some(ref mut backend) = self.backend {
            backend.cleanup();
        }
    }

    /// Check if daemon is running.
    pub fn is_running(&self) -> bool {
        self.running
    }
}

impl Default for Daemon {
    fn default() -> Self {
        Self::new()
    }
}

/// Run mini-eq in background/daemon mode.
pub async fn run_daemon() -> anyhow::Result<()> {
    log::info!("Starting mini-eq in background mode");

    let settings = Settings::load();
    let mut daemon = Daemon::new();
    
    daemon.init(&settings).await?;
    
    // Set up signal handlers for graceful shutdown
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap_or(());
        log::info!("Received shutdown signal");
    });

    daemon.run().await
}