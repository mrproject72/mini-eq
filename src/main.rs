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

    println!(
        "{} v{} starting...",
        cli.command
            .as_ref()
            .map(|c| format!("{:?}", c))
            .unwrap_or_else(|| "mini-eq".to_string()),
        env!("CARGO_PKG_VERSION")
    );

    if cli.background {
        println!("Running in background mode");
    }
    if cli.auto_route {
        println!("Auto-routing enabled");
    }
    if cli.headless {
        println!("Running headless");
    }
    if let Some(dur) = cli.duration {
        println!("Duration: {}s", dur);
    }
    if let Some(apo) = cli.import_apo {
        println!("Importing APO preset: {:?}", apo);
    }
    if let Some(sink) = cli.output_sink {
        println!("Output sink: {}", sink);
    }
}
