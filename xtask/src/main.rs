//! Repo automation.

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use std::process::Command;

/// xtask CLI.
#[derive(Debug, Parser)]
#[command(name = "xtask")]
struct Cli {
    #[command(subcommand)]
    command: Task,
}

/// Tasks.
#[derive(Debug, Subcommand)]
enum Task {
    /// Run local CI commands.
    Ci,
    /// Print handoff checks.
    Handoff,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Task::Ci => ci(),
        Task::Handoff => {
            println!("OpenIRL Rust Kit handoff checklist:");
            println!("- Initial handoff feature areas: 8");
            println!("- Current schema revision: 38");
            println!("- Read docs/CODEX_HANDOFF.md");
            println!("- Run python3 scripts/static_validate.py before cargo checks");
            println!(
                "- Validate feature areas OBS reconciliation, local ingest, mobile profiles, dashboard, security, brownout, relay, NAT, public beta packaging, WebRTC preview, vertical clips, and plugin API before v1 release"
            );
            Ok(())
        }
    }
}

fn ci() -> Result<()> {
    run_optional("python3", &["scripts/static_validate.py"])?;
    run("cargo", &["fmt", "--all", "--", "--check"])?;
    run("cargo", &["clippy", "--workspace", "--all-targets"])?;
    run("cargo", &["test", "--workspace"])?;
    Ok(())
}

fn run_optional(program: &str, args: &[&str]) -> Result<()> {
    match Command::new(program).args(args).status() {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => bail!("command failed: {program} {} with {status}", args.join(" ")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            println!(
                "optional command unavailable: {program}; skipping {}",
                args.join(" ")
            );
            Ok(())
        }
        Err(error) => Err(error).with_context(|| format!("failed to start {program}")),
    }
}

fn run(program: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(program)
        .args(args)
        .status()
        .with_context(|| format!("failed to start {program}"))?;
    if !status.success() {
        bail!("command failed: {program} {}", args.join(" "));
    }
    Ok(())
}
