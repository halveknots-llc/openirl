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
    /// Print maintainer checks.
    Handoff,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Task::Ci => ci(),
        Task::Handoff => {
            println!("OpenIRL maintainer checklist:");
            println!("- Initial feature areas: 8");
            println!("- Current schema revision: 38");
            println!("- Read docs/MAINTAINER_CHECKS.md");
            println!("- Run python3 scripts/static_validate.py before cargo checks");
            println!(
                "- Validate feature areas OBS reconciliation, local ingest, mobile profiles, dashboard, security, brownout, relay, NAT, public beta packaging, WebRTC preview, vertical clips, and plugin API before v1 release"
            );
            Ok(())
        }
    }
}

fn ci() -> Result<()> {
    run("python3", &["scripts/static_validate.py"])?;
    run("python3", &["scripts/audit/handoff_audit.py"])?;
    run("python3", &["scripts/security/security-audit-smoke.py"])?;
    run("cargo", &["deny", "check"])?;
    run("cargo", &["fmt", "--all", "--", "--check"])?;
    run(
        "cargo",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    run("cargo", &["test", "--workspace"])?;
    Ok(())
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
