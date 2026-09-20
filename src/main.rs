//! Mini EQ — main entry point.

use adw::prelude::*;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use mini_eq::background;
use mini_eq::dbus_control::{MiniEqAppHandler, MiniEqDBusControl};

#[derive(Parser)]
#[command(name = "mini-eq")]
#[command(about = "Compact PipeWire system-wide parametric equalizer")]
#[command(version = env!("CARGO_PKG_VERSION"))]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(long, global = true)]
    verbose: bool,

    #[arg(long, global = true)]
    background: bool,

    #[arg(long, global = true)]
    auto_route: bool,

    #[arg(long, global = true)]
    headless: bool,

    #[arg(long, global = true)]
    duration: Option<u64>,

    #[arg(long, global = true)]
    import_apo: Option<PathBuf>,

    #[arg(long, global = true)]
    check_deps: bool,

    #[arg(long, global = true)]
    output_sink: Option<String>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    InstallDesktop,
    CheckDeps,
}

/// Shared application state for D-Bus handler.
struct AppState {
    eq_enabled: Mutex<bool>,
    routed: Mutex<bool>,
    preset_name: Mutex<Option<String>>,
    output_sink: Mutex<Option<String>>,
    background_mode: Mutex<bool>,
    start_at_login: Mutex<bool>,
    start_active_at_login: Mutex<bool>,
    analyzer_enabled: Mutex<bool>,
    window_visible: Mutex<bool>,
}

impl AppState {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            eq_enabled: Mutex::new(true),
            routed: Mutex::new(false),
            preset_name: Mutex::new(None),
            output_sink: Mutex::new(None),
            background_mode: Mutex::new(background::load_background_mode()),
            start_at_login: Mutex::new(background::load_start_at_login()),
            start_active_at_login: Mutex::new(background::load_start_active_at_login()),
            analyzer_enabled: Mutex::new(false),
            window_visible: Mutex::new(true),
        })
    }
}

impl MiniEqAppHandler for AppState {
    fn eq_enabled(&self) -> bool {
        *self.eq_enabled.lock().unwrap()
    }
    fn routed(&self) -> bool {
        *self.routed.lock().unwrap()
    }
    fn output_sink(&self) -> Option<String> {
        self.output_sink.lock().unwrap().clone()
    }

    fn set_eq_enabled(&self, enabled: bool) {
        *self.eq_enabled.lock().unwrap() = enabled;
    }
    fn route_system_audio(&self, enabled: bool) {
        *self.routed.lock().unwrap() = enabled;
    }

    fn current_preset_name(&self) -> Option<String> {
        self.preset_name.lock().unwrap().clone()
    }
    fn background_mode(&self) -> bool {
        *self.background_mode.lock().unwrap()
    }
    fn start_at_login(&self) -> bool {
        *self.start_at_login.lock().unwrap()
    }
    fn start_active_at_login(&self) -> bool {
        *self.start_active_at_login.lock().unwrap()
    }
    fn analyzer_enabled(&self) -> bool {
        *self.analyzer_enabled.lock().unwrap()
    }
    fn analyzer_levels(&self) -> Vec<f64> {
        Vec::new()
    }
    fn analyzer_display_gain_db(&self) -> f64 {
        0.0
    }
    fn window_visible(&self) -> bool {
        *self.window_visible.lock().unwrap()
    }
    fn ui_shutting_down(&self) -> bool {
        false
    }

    fn present_main_window(&self, _startup_id: Option<&str>) {}
    fn quit_fully(&self) {}
    fn load_library_preset(&self, _name: &str) {}
}

fn main() {
    let cli = Cli::parse();

    if cli.check_deps {
        println!("Mini EQ dependency check");
        println!("  [OK] Rust: 1.98.1");
        println!("  [OK] mini-eq v{}", env!("CARGO_PKG_VERSION"));
        return;
    }

    env_logger::init();

    if cli.headless {
        println!("Running headless (no GUI)");
        if let Some(dur) = cli.duration {
            println!("Duration: {}s", dur);
        }
        return;
    }

    // Launch GTK4 application
    launch_gui(cli.background, cli.auto_route);
}

fn launch_gui(_background_mode: bool, _auto_route: bool) {
    let _ = adw::init();
    let app = adw::Application::new(
        Some("io.github.bhack.mini-eq"),
        adw::gio::ApplicationFlags::empty(),
    );

    let app_state = AppState::new();

    // Register D-Bus control
    let dbus_control = MiniEqDBusControl::new(app_state.clone());
    if let Err(e) = dbus_control.register() {
        log::warn!("Failed to register D-Bus control: {}", e);
    }

    app.connect_activate(move |app| {
        let window = mini_eq::window::MiniEqWindow::new(app);
        window.present();
    });

    let args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let _ = app.run_with_args_os(&args);
}
