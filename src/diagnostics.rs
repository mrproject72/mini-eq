//! Diagnostic and debugging tools for mini-eq.
//!
//! Provides runtime diagnostics, logging utilities, and debug information.

use anyhow::Context;
use std::collections::HashMap;

/// System diagnostic information.
#[derive(Debug, Clone)]
pub struct Diagnostics {
    /// PipeWire version string.
    pub pipewire_version: Option<String>,
    /// GTK version string.
    pub gtk_version: Option<String>,
    /// Whether running as Flatpak.
    pub is_flatpak: bool,
    /// Current memory usage in KB.
    pub memory_usage_kb: u64,
    /// Number of active EQ bands.
    pub active_bands: usize,
    /// Backend latency in samples.
    pub backend_latency_samples: u32,
}

impl Default for Diagnostics {
    fn default() -> Self {
        Self {
            pipewire_version: None,
            gtk_version: None,
            is_flatpak: false,
            memory_usage_kb: 0,
            active_bands: 0,
            backend_latency_samples: 0,
        }
    }
}

impl Diagnostics {
    /// Collect system diagnostics.
    pub fn collect() -> Self {
        let mut diag = Self::default();

        // Check if running as Flatpak
        diag.is_flatpak = std::path::Path::new("/.flatpak-info").exists();

        // Get memory usage
        diag.memory_usage_kb = Self::get_memory_usage();

        // Get PipeWire version
        diag.pipewire_version = Self::get_pipewire_version();

        // Get GTK version
        diag.gtk_version = Some(format!(
            "{}.{}.{}",
            gtk4::major_version(),
            gtk4::minor_version(),
            gtk4::micro_version()
        ));

        log::info!("Diagnostics collected: {:?}", diag);
        diag
    }

    /// Get current memory usage from /proc/self/status.
    fn get_memory_usage() -> u64 {
        if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
            for line in status.lines() {
                if line.starts_with("VmRSS:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        return parts[1].parse().unwrap_or(0);
                    }
                }
            }
        }
        0
    }

    /// Get PipeWire version via pw-cat or pactl.
    fn get_pipewire_version() -> Option<String> {
        // Try pw-cat first
        if let Ok(output) = std::process::Command::new("pw-cat")
            .arg("--version")
            .output()
        {
            if output.status.success() {
                return String::from_utf8_lossy(&output.stdout).trim().to_string().into();
            }
        }

        // Try pactl as fallback
        if let Ok(output) = std::process::Command::new("pactl")
            .arg("--version")
            .output()
        {
            if output.status.success() {
                return String::from_utf8_lossy(&output.stdout).trim().to_string().into();
            }
        }

        None
    }

    /// Generate a diagnostic report string.
    pub fn to_report(&self) -> String {
        let mut report = String::from("=== Mini EQ Diagnostic Report ===\n\n");

        report.push_str(&format!("Flatpak: {}\n", if self.is_flatpak { "Yes" } else { "No" }));
        report.push_str(&format!("Memory Usage: {} KB\n", self.memory_usage_kb));

        if let Some(ref ver) = self.pipewire_version {
            report.push_str(&format!("PipeWire: {}\n", ver));
        } else {
            report.push_str("PipeWire: Not detected\n");
        }

        if let Some(ref ver) = self.gtk_version {
            report.push_str(&format!("GTK: {}\n", ver));
        }

        report.push_str(&format!("Active EQ Bands: {}\n", self.active_bands));
        report.push_str(&format!("Backend Latency: {} samples\n", self.backend_latency_samples));

        report
    }

    /// Export diagnostics to JSON.
    pub fn to_json(&self) -> anyhow::Result<String> {
        serde_json::to_string_pretty(&serde_json::json!({
            "pipewire_version": self.pipewire_version,
            "gtk_version": self.gtk_version,
            "is_flatpak": self.is_flatpak,
            "memory_usage_kb": self.memory_usage_kb,
            "active_bands": self.active_bands,
            "backend_latency_samples": self.backend_latency_samples,
        }))
        .context("Failed to serialize diagnostics")
    }

    /// Update active band count.
    pub fn set_active_bands(&mut self, count: usize) {
        self.active_bands = count;
    }

    /// Update backend latency.
    pub fn set_backend_latency(&mut self, samples: u32) {
        self.backend_latency_samples = samples;
    }
}

/// Run diagnostics and print to stdout.
pub fn run_diagnostics() {
    let diag = Diagnostics::collect();
    println!("{}", diag.to_report());
}

/// Get quick system status summary.
pub fn get_status_summary() -> HashMap<String, String> {
    let diag = Diagnostics::collect();
    let mut map = HashMap::new();

    map.insert("flatpak".to_string(), diag.is_flatpak.to_string());
    map.insert("memory_kb".to_string(), diag.memory_usage_kb.to_string());
    
    if let Some(ver) = diag.pipewire_version {
        map.insert("pipewire".to_string(), ver);
    }
    
    if let Some(ver) = diag.gtk_version {
        map.insert("gtk".to_string(), ver);
    }

    map
}