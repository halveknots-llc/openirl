//! Optional OpenIRL relay CLI and supervisor shell.

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use openirl_config::{AppConfig, RelayProcessKind as ConfigRelayProcessKind, load_config};
use openirl_core::DeploymentMode;
use openirl_relay_control::{
    RelayBackend, RelayEnvPair, RelayProcessConfig, RelayRuntimeStatus, RelaySupervisor,
};
use std::path::PathBuf;
use tracing_subscriber::{EnvFilter, fmt};

/// Relay CLI.
#[derive(Debug, Parser)]
#[command(name = "openirl-relay", about = "OpenIRL relay process supervisor")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

/// Relay commands.
#[derive(Debug, Subcommand)]
enum Command {
    /// Print redacted relay launch plans from config.
    Plan {
        /// Optional config path.
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Print relay process status snapshots without starting tools.
    Status {
        /// Optional config path.
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Start and supervise a process until Ctrl+C.
    Supervise {
        /// Optional config path.
        #[arg(long)]
        config: Option<PathBuf>,
        /// Optional process name.
        #[arg(long)]
        process: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).init();
    let cli = Cli::parse();

    match cli.command {
        Command::Plan { config } => {
            let config = load_config_or_default(config)?;
            let supervisors = supervisors_from_config(&config);
            let mut plans = Vec::new();
            for supervisor in &supervisors {
                plans.push(supervisor.plan().await);
            }
            println!("{}", serde_json::to_string_pretty(&plans)?);
        }
        Command::Status { config } => {
            let config = load_config_or_default(config)?;
            let supervisors = supervisors_from_config(&config);
            let mut statuses = Vec::new();
            for supervisor in &supervisors {
                statuses.push(status_or_error(supervisor.status().await));
            }
            println!("{}", serde_json::to_string_pretty(&statuses)?);
        }
        Command::Supervise { config, process } => {
            let config = load_config_or_default(config)?;
            let supervisors = supervisors_from_config(&config);
            let Some(supervisor) = select_supervisor(&supervisors, process.as_deref()).await else {
                anyhow::bail!("no matching relay process found");
            };
            let started = supervisor.start().await?;
            println!("{}", serde_json::to_string_pretty(&started)?);
            tokio::signal::ctrl_c().await?;
            let stopped = supervisor.stop().await?;
            println!("{}", serde_json::to_string_pretty(&stopped)?);
        }
    }
    Ok(())
}

fn load_config_or_default(config_path: Option<PathBuf>) -> Result<AppConfig> {
    match config_path {
        Some(path) => load_config(&path)
            .with_context(|| format!("failed to load config at {}", path.display())),
        None => Ok(AppConfig::default()),
    }
}

fn supervisors_from_config(config: &AppConfig) -> Vec<RelaySupervisor> {
    let mut supervisors = Vec::new();
    let mode = if config.relay.enabled {
        config.relay.mode
    } else {
        DeploymentMode::LocalDirect
    };

    for process in &config.relay.processes {
        supervisors.push(RelaySupervisor::new(RelayProcessConfig {
            name: process.name.clone(),
            enabled: config.relay.enabled && process.enabled,
            auto_start: config.relay.enabled && config.relay.auto_start && process.enabled,
            mode,
            backend: relay_backend_from_config(process.kind),
            executable: relay_executable_from_config(process),
            args: relay_args_from_config(config, process),
            working_dir: process.working_dir.as_ref().map(PathBuf::from),
            env: process
                .env
                .iter()
                .map(|env_var| RelayEnvPair {
                    key: env_var.key.clone(),
                    value: env_var.value.clone(),
                })
                .collect(),
            restart_on_exit: process.restart_on_exit,
            metrics_url: relay_metrics_url(config, process.kind),
            api_url: relay_api_url(config, process.kind),
            log_tail_limit: 200,
        }));
    }

    if supervisors.is_empty() {
        supervisors.push(RelaySupervisor::new(RelayProcessConfig::disabled()));
    }

    supervisors
}

async fn select_supervisor(
    supervisors: &[RelaySupervisor],
    name: Option<&str>,
) -> Option<RelaySupervisor> {
    match name {
        Some(name) => {
            for supervisor in supervisors {
                if supervisor.name().await == name {
                    return Some(supervisor.clone());
                }
            }
            None
        }
        None => supervisors.first().cloned(),
    }
}

fn status_or_error(
    result: Result<RelayRuntimeStatus, openirl_relay_control::RelayControlError>,
) -> serde_json::Value {
    match result {
        Ok(status) => serde_json::json!(status),
        Err(error) => serde_json::json!({ "error": error.to_string() }),
    }
}

fn relay_executable_from_config(process: &openirl_config::RelayProcessConfig) -> String {
    if process.executable_env.trim().is_empty() {
        return process.executable.clone();
    }

    std::env::var(&process.executable_env).unwrap_or_else(|_| process.executable.clone())
}

fn relay_args_from_config(
    config: &AppConfig,
    process: &openirl_config::RelayProcessConfig,
) -> Vec<String> {
    if process.kind == ConfigRelayProcessKind::MediaMtx && process.args.is_empty() {
        vec![config.relay.mediamtx_config_path.clone()]
    } else {
        process.args.clone()
    }
}

fn relay_backend_from_config(kind: ConfigRelayProcessKind) -> RelayBackend {
    match kind {
        ConfigRelayProcessKind::MediaMtx => RelayBackend::MediaMtx,
        ConfigRelayProcessKind::SrtlaReceive => RelayBackend::SrtlaReceive,
        ConfigRelayProcessKind::SrtlaSend => RelayBackend::SrtlaSend,
        ConfigRelayProcessKind::SrtLiveTransmit => RelayBackend::SrtLiveTransmit,
        ConfigRelayProcessKind::Custom => RelayBackend::Custom,
    }
}

fn relay_metrics_url(config: &AppConfig, kind: ConfigRelayProcessKind) -> Option<String> {
    match kind {
        ConfigRelayProcessKind::MediaMtx => Some(config.relay.mediamtx_metrics_url.clone()),
        ConfigRelayProcessKind::SrtlaReceive
        | ConfigRelayProcessKind::SrtlaSend
        | ConfigRelayProcessKind::SrtLiveTransmit
        | ConfigRelayProcessKind::Custom => None,
    }
}

fn relay_api_url(config: &AppConfig, kind: ConfigRelayProcessKind) -> Option<String> {
    match kind {
        ConfigRelayProcessKind::MediaMtx => Some(config.relay.mediamtx_api_url.clone()),
        ConfigRelayProcessKind::SrtlaReceive
        | ConfigRelayProcessKind::SrtlaSend
        | ConfigRelayProcessKind::SrtLiveTransmit
        | ConfigRelayProcessKind::Custom => None,
    }
}
