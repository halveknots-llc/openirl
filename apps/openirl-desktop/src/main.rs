//! Minimal Rust desktop shell entrypoint.
//!
//! This binary is intentionally GUI-toolkit-light for feature areas. It provides the
//! tray/menu contract that a Tauri shell or native tray crate can wire to OS UI
//! without changing OpenIRL agent APIs.

use anyhow::Result;
use clap::{Parser, Subcommand};
use openirl_desktop_shell::default_desktop_shell_plan;

/// CLI args.
#[derive(Debug, Parser)]
#[command(name = "openirl-desktop", about = "OpenIRL desktop/tray shell")]
struct Cli {
    /// Command.
    #[command(subcommand)]
    command: Command,
}

/// Desktop shell commands.
#[derive(Debug, Subcommand)]
enum Command {
    /// Print the desktop shell plan.
    Plan {
        /// Dashboard URL.
        #[arg(long, default_value = "http://127.0.0.1:7707/")]
        dashboard_url: String,
    },
    /// Print the dashboard URL that a tray click should open.
    OpenDashboard {
        /// Dashboard URL.
        #[arg(long, default_value = "http://127.0.0.1:7707/")]
        dashboard_url: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Plan { dashboard_url } => {
            let plan = default_desktop_shell_plan(dashboard_url);
            println!("{}", serde_json::to_string_pretty(&plan)?);
        }
        Command::OpenDashboard { dashboard_url } => {
            println!("{dashboard_url}");
        }
    }
    Ok(())
}
