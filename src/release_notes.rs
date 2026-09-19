//! Release notes and changelog display for mini-eq.
//!
//! Manages version information and displays changelog to users.

use serde::{Deserialize, Serialize};

/// Application version information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    /// Version string (e.g., "1.0.0").
    pub version: String,
    /// Release date.
    pub date: Option<String>,
    /// Whether this is a pre-release.
    pub prerelease: bool,
}

/// Release note entry for a specific version.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseNote {
    /// Version number.
    pub version: String,
    /// Release date.
    pub date: Option<String>,
    /// New features in this release.
    pub features: Vec<String>,
    /// Bug fixes in this release.
    pub fixes: Vec<String>,
    /// Breaking changes or migration notes.
    pub notes: Vec<String>,
}

impl ReleaseNote {
    /// Create a new release note.
    pub fn new(version: &str) -> Self {
        Self {
            version: version.to_string(),
            date: None,
            features: vec![],
            fixes: vec![],
            notes: vec![],
        }
    }

    /// Add a feature to the release note.
    pub fn with_feature(mut self, feature: &str) -> Self {
        self.features.push(feature.to_string());
        self
    }

    /// Add a bug fix to the release note.
    pub fn with_fix(mut self, fix: &str) -> Self {
        self.fixes.push(fix.to_string());
        self
    }

    /// Add a note to the release note.
    pub fn with_note(mut self, note: &str) -> Self {
        self.notes.push(note.to_string());
        self
    }

    /// Set the release date.
    pub fn with_date(mut self, date: &str) -> Self {
        self.date = Some(date.to_string());
        self
    }
}

/// Release notes manager.
pub struct ReleaseNotesManager {
    /// Current application version.
    current_version: String,
    /// List of release notes.
    notes: Vec<ReleaseNote>,
}

impl Default for ReleaseNotesManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ReleaseNotesManager {
    /// Create a new release notes manager.
    pub fn new() -> Self {
        let mut manager = Self {
            current_version: env!("CARGO_PKG_VERSION").to_string(),
            notes: vec![],
        };

        // Initialize with built-in release notes
        manager.initialize_builtin_notes();
        manager
    }

    /// Initialize built-in release notes.
    fn initialize_builtin_notes(&mut self) {
        let v1_0 = ReleaseNote::new("1.0.0")
            .with_date("2025-01-15")
            .with_feature("Complete Rust rewrite with GTK4 UI")
            .with_feature("PipeWire virtual sink integration")
            .with_feature("FFT spectrum analyzer")
            .with_feature("AutoEq preset support")
            .with_feature("Background daemon mode")
            .with_feature("D-Bus remote control")
            .with_fix("Improved CPU efficiency vs Python original")
            .with_fix("Better memory management");

        let v0_9 = ReleaseNote::new("0.9.0")
            .with_date("2024-12-01")
            .with_feature("Beta release of Rust port")
            .with_feature("Basic EQ band controls")
            .with_feature("Preset management");

        self.notes = vec![v1_0, v0_9];
    }

    /// Get the current version.
    pub fn get_current_version(&self) -> &str {
        &self.current_version
    }

    /// Get all release notes.
    pub fn get_all_notes(&self) -> &[ReleaseNote] {
        &self.notes
    }

    /// Get release notes for a specific version.
    pub fn get_notes_for_version(&self, version: &str) -> Option<&ReleaseNote> {
        self.notes.iter().find(|n| n.version == version)
    }

    /// Format release notes as markdown.
    pub fn format_as_markdown(&self) -> String {
        let mut md = String::from("# Mini EQ Release Notes\n\n");

        for note in &self.notes {
            md.push_str(&format!("## Version {}{}\n\n", 
                note.version,
                note.date.as_ref().map(|d| format!(" ({})", d)).unwrap_or_default()
            ));

            if !note.features.is_empty() {
                md.push_str("### Features\n\n");
                for feature in &note.features {
                    md.push_str(&format!("- {}\n", feature));
                }
                md.push('\n');
            }

            if !note.fixes.is_empty() {
                md.push_str("### Bug Fixes\n\n");
                for fix in &note.fixes {
                    md.push_str(&format!("- {}\n", fix));
                }
                md.push('\n');
            }

            if !note.notes.is_empty() {
                md.push_str("### Notes\n\n");
                for n in &note.notes {
                    md.push_str(&format!("- {}\n", n));
                }
                md.push('\n');
            }
        }

        md
    }

    /// Get the latest release notes.
    pub fn get_latest(&self) -> Option<&ReleaseNote> {
        self.notes.first()
    }

    /// Check if there are new features since last viewed version.
    pub fn has_new_features(&self, last_viewed: &str) -> bool {
        self.notes.iter()
            .filter(|n| n.version > last_viewed)
            .any(|n| !n.features.is_empty())
    }

    /// Add a custom release note.
    pub fn add_note(&mut self, note: ReleaseNote) {
        // Insert at the beginning (most recent first)
        self.notes.insert(0, note);
    }
}

/// Get the application version string.
pub fn get_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Display release notes to stdout.
pub fn show_release_notes() {
    let manager = ReleaseNotesManager::new();
    println!("{}", manager.format_as_markdown());
}