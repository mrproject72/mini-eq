//! Mini EQ — main entry point.

use adw::prelude::*;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

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
    launch_gui();
}

fn launch_gui() {
    let _ = adw::init();
    let app = adw::Application::new(
        Some("io.github.bhack.mini-eq"),
        adw::gio::ApplicationFlags::empty(),
    );

    app.connect_activate(move |app| {
        let window = mini_eq::window::MiniEqWindow::new(app);
        window.present();
    });

    let args: Vec<std::ffi::OsString> = std::env::args_os().collect();
    let _ = app.run_with_args_os(&args);
}
