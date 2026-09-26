//! Mini EQ — main entry point.

use std::sync::Arc;
use std::sync::Mutex;

use adw::prelude::*;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use mini_eq::background;
use mini_eq::core::default_bands;
use mini_eq::dbus_control::{MiniEqAppHandler, MiniEqDBusControl};
use mini_eq::pipewire_backend::PipeWireBackend;

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
///
/// The D-Bus handler is invoked from the GLib main thread only, so this struct
/// deliberately does not implement `Send`/`Sync`. The PipeWire backend is *not*
/// stored here — it is owned locally by `launch_gui`, because `PipeWireBackend`
/// wraps `MainLoopRc` and is neither `Send` nor `Sync`.
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

    if let Some(command) = &cli.command {
        match command {
            Commands::InstallDesktop => {
                match mini_eq::desktop_integration::install_desktop_integration() {
                    Ok(()) => println!("Installed desktop launcher and app icons."),
                    Err(e) => {
                        eprintln!("Failed to install desktop integration: {}", e);
                        std::process::exit(1);
                    }
                }
                return;
            }
            Commands::CheckDeps => {
                print_dependency_report();
                return;
            }
        }
    }

    if cli.check_deps {
        print_dependency_report();
        return;
    }

    env_logger::init();

    if cli.headless {
        run_headless(cli.duration, cli.import_apo.as_deref());
        return;
    }

    // Launch GTK4 application
    launch_gui(cli.background, cli.auto_route, cli.output_sink);
}

fn print_dependency_report() {
    println!("Mini EQ dependency check");
    println!("  [OK] mini-eq v{}", env!("CARGO_PKG_VERSION"));
    println!("  [OK] GTK4 / Libadwaita (linked at build time)");

    let module = std::path::Path::new(
        "/usr/lib/x86_64-linux-gnu/pipewire-0.3/libpipewire-module-filter-chain.so",
    );
    let builtin = std::path::Path::new(
        "/usr/lib/x86_64-linux-gnu/spa-0.2/filter-graph/libspa-filter-graph-plugin-builtin.so",
    );
    println!(
        "  [{}] libpipewire-module-filter-chain",
        if module.exists() { "OK" } else { "MISSING" }
    );
    println!(
        "  [{}] libspa-filter-graph-plugin-builtin",
        if builtin.exists() { "OK" } else { "MISSING" }
    );

    let socket = std::env::var("XDG_RUNTIME_DIR")
        .map(|dir| std::path::PathBuf::from(dir).join("pipewire-0"))
        .ok();
    let running = socket.as_deref().map(|p| p.exists()).unwrap_or(false);
    println!(
        "  [{}] PipeWire daemon socket",
        if running { "OK" } else { "MISSING" }
    );
}

fn run_headless(duration: Option<u64>, import_apo: Option<&std::path::Path>) {
    println!("Running headless (no GUI)");
    let bands = match import_apo {
        Some(path) => match mini_eq::autoeq::parse_apo_file(path) {
            Ok((preamp, bands)) => {
                println!(
                    "Imported APO preset: {} band(s), preamp {:.1} dB",
                    bands.len(),
                    preamp
                );
                bands
            }
            Err(e) => {
                eprintln!("Failed to import APO preset: {}", e);
                std::process::exit(1);
            }
        },
        None => default_bands(),
    };
    let mut backend = match PipeWireBackend::new(bands) {
        Ok(backend) => backend,
        Err(e) => {
            eprintln!("Failed to initialise PipeWire backend: {}", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = backend.create_filter_chain("mini_eq_sink_output") {
        log::warn!("Failed to create filter chain: {}", e);
    }

    match duration {
        Some(secs) => {
            println!("Duration: {}s", secs);
            std::thread::sleep(std::time::Duration::from_secs(secs));
        }
        None => println!("Running until interrupted"),
    }
}

#[allow(clippy::collapsible_if)]
fn launch_gui(_background_mode: bool, auto_route: bool, output_sink: Option<String>) {
    let _ = adw::init();
    let app = adw::Application::new(
        Some("io.github.bhack.mini-eq"),
        adw::gio::ApplicationFlags::empty(),
    );

    let app_state = AppState::new();

    // Initialize PipeWire backend. It is owned here rather than in `AppState`
    // because it wraps `MainLoopRc` and is neither `Send` nor `Sync`.
    let mut backend = PipeWireBackend::new(default_bands()).ok();
    if let Some(backend) = backend.as_mut() {
        let sink = output_sink.as_deref().unwrap_or("mini_eq_sink_output");
        // The filter-chain module creates the virtual sink and output node;
        // routing is configured separately.
        if let Err(e) = backend.create_filter_chain(sink) {
            log::warn!("Failed to load filter chain: {}", e);
        }
        if auto_route {
            if let Err(e) = backend.auto_route_to_sink(sink) {
                log::warn!("Failed to auto-route: {}", e);
            }
        }
    }

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
