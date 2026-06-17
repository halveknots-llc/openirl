//! OpenIRL local agent.

use anyhow::{Context, bail};
use axum::{
    Json, Router,
    extract::{ConnectInfo, FromRequestParts, Path, Request, State},
    http::{
        HeaderValue, Method, StatusCode,
        header::{AUTHORIZATION, CONTENT_TYPE},
        request::Parts,
    },
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use clap::{Parser, Subcommand, ValueEnum};
use openirl_alpha_validation::{
    AlphaEvidenceInput, build_alpha_validation_plan, build_operator_checklist,
    evaluate_alpha_evidence,
};
use openirl_artifacts::{
    SupportBundleExportRequest, alpha_source_package_layout, build_obs_scene_template_plan,
    default_fallback_asset_plan, export_field_report_markdown, export_support_bundle,
    materialize_alpha_source_layout, materialize_fallback_assets, materialize_obs_scene_template,
};
use openirl_auth::{AuthPolicy, auth_status, verify_authorization_header};
use openirl_config::{
    AppConfig, ConfigValidationReport, MetricsSourceKind, ObsAdapterKind,
    RelayProcessKind as ConfigRelayProcessKind, RelaySupervisorMode, load_config, validate_config,
};
use openirl_core::{
    EncoderKind, HealthDecision, HealthState, INITIAL_FEATURE_AREA_COUNT, Protocol, SceneBundle,
    SceneRole, StreamMetrics, feature_areas,
};
use openirl_desktop_shell::default_desktop_shell_plan;
use openirl_diagnostics::SupportBundleManifest;
use openirl_field_validation::{
    FieldEvidenceInput, build_device_checklists, build_field_validation_plan,
    evaluate_field_evidence, sample_field_evidence,
};
use openirl_health::HealthEngine;
use openirl_installer::{InstallPlanRequest, build_install_plan, default_windows_service_plan};
use openirl_metrics::{
    MetricsPoller, MetricsScenario, MetricsSourceConfig, RelayMetricsSnapshot, RelayMetricsState,
    now_ms, parse_prometheus_text, poll_http_text, simulated_relay_snapshot,
};
use openirl_obs::{DryRunObsController, ObsController, ObsWebSocketConfig, ObsWebSocketController};
use openirl_onboarding::{OnboardingRequest, onboarding_options, plan_onboarding};
use openirl_production::{
    ClipMarkerRequest, ClipMarkerStore, ModeratorCommandRequest, ModeratorRole,
    default_production_plan, evaluate_moderator_command,
};
use openirl_profiles::{GeneratedProfile, ProfileRequest, generate_profile, support_matrix};
use openirl_qr::{QrRenderRequest, render_qr_svg};
use openirl_relay_control::{
    RelayBackend, RelayEnvPair, RelayLaunchPlan, RelayProcessConfig as RuntimeRelayProcessConfig,
    RelayRuntimeStatus, RelaySupervisor, build_credential_plan,
};
use openirl_release::build_release_manifest;
use openirl_scene_templates::{SceneTemplateRequest, build_scene_materialization_plan};
use openirl_session::{SessionEventKind, SessionStore};
use openirl_v1::{
    V1EvidenceInput, build_v1_features, build_v1_implementation_summary, default_v1_package_layout,
    evaluate_v1_evidence, materialize_v1_package, sample_v1_evidence,
};
use openirl_vault::{redact_support_text, scrub_support_bundle_value};
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    net::SocketAddr,
    path::{Component, PathBuf},
    sync::Arc,
};
use time::OffsetDateTime;
use tokio::{
    sync::RwLock,
    time::{Duration, sleep},
};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};
use tracing_subscriber::{EnvFilter, fmt};

/// Current schema revision.
const OPENIRL_SCHEMA_REVISION: u16 = 38;

/// CLI args.
#[derive(Debug, Parser)]
#[command(name = "openirl-agent", about = "Local-first OpenIRL daemon")]
struct Cli {
    /// Command.
    #[command(subcommand)]
    command: Command,
}

/// Commands.
#[derive(Debug, Subcommand)]
enum Command {
    /// Serve the local API and static PWA shell.
    Serve {
        /// Optional config path.
        #[arg(long)]
        config: Option<PathBuf>,
        /// Bind address override.
        #[arg(long)]
        bind: Option<SocketAddr>,
        /// Optional OBS adapter override.
        #[arg(long, value_enum)]
        obs_adapter: Option<ObsAdapterArg>,
    },
    /// Validate the config and print a redacted readiness report.
    CheckConfig {
        /// Optional config path.
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Print a Windows-first installer plan.
    InstallPlan {
        /// Optional config path.
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Print the desktop/tray shell plan.
    DesktopPlan {
        /// Dashboard URL.
        #[arg(long, default_value = "http://127.0.0.1:7707/")]
        dashboard_url: String,
    },
    /// Build a first-run onboarding plan with profile and QR payload.
    Onboarding {
        /// Optional config path.
        #[arg(long)]
        config: Option<PathBuf>,
        /// Encoder.
        #[arg(long, value_enum, default_value = "moblin")]
        encoder: EncoderArg,
        /// Optional protocol override.
        #[arg(long, value_enum)]
        protocol: Option<ProtocolArg>,
        /// Deployment mode.
        #[arg(long, value_enum, default_value = "local-direct")]
        mode: DeploymentModeArg,
        /// Public host override.
        #[arg(long)]
        host: Option<String>,
        /// Do not render QR SVG.
        #[arg(long)]
        no_qr: bool,
    },
    /// Print release hardening manifest.
    ReleaseManifest {
        /// Optional config path.
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Print the feature areas Windows + OBS alpha validation plan.
    AlphaPlan,
    /// Evaluate feature areas alpha evidence flags.
    AlphaEvidence {
        /// Static validation passed.
        #[arg(long)]
        static_validation: bool,
        /// Rust cargo CI passed.
        #[arg(long)]
        rust_ci: bool,
        /// Config validation passed.
        #[arg(long)]
        config_ok: bool,
        /// Agent health endpoint passed.
        #[arg(long)]
        agent_health: bool,
        /// Dashboard loaded locally.
        #[arg(long)]
        dashboard_loaded: bool,
        /// OBS WebSocket connected.
        #[arg(long)]
        obs_connected: bool,
        /// OBS scene switching verified.
        #[arg(long)]
        obs_scenes: bool,
        /// OBS streaming start/stop verified.
        #[arg(long)]
        obs_stream_controls: bool,
        /// OBS replay save verified.
        #[arg(long)]
        replay_save: bool,
        /// OBS recording controls verified.
        #[arg(long)]
        recording_controls: bool,
        /// Real encoder profile QR tested.
        #[arg(long)]
        profile_qr: bool,
        /// Relay or demo metrics poll verified.
        #[arg(long)]
        metrics_poll: bool,
        /// Windows portable artifact built.
        #[arg(long)]
        portable_built: bool,
        /// MSI template reviewed or deferred.
        #[arg(long)]
        msi_reviewed: bool,
    },
    /// Print the feature areas real mobile field validation plan.
    FieldPlan,
    /// Print an editable feature areas field evidence JSON payload.
    FieldSampleEvidence,
    /// Evaluate feature areas real mobile field evidence flags.
    FieldEvidence {
        /// Static validation passed.
        #[arg(long)]
        static_validation: bool,
        /// Rust cargo CI passed.
        #[arg(long)]
        rust_ci: bool,
        /// feature areas Windows/OBS alpha baseline passed.
        #[arg(long)]
        windows_alpha_ready: bool,
        /// Moblin profile generated.
        #[arg(long)]
        moblin_profile: bool,
        /// Moblin QR/profile accepted on device.
        #[arg(long)]
        moblin_qr: bool,
        /// Moblin ingest observed.
        #[arg(long)]
        moblin_ingest: bool,
        /// IRL Pro profile generated.
        #[arg(long)]
        irlpro_profile: bool,
        /// IRL Pro QR/profile accepted on device.
        #[arg(long)]
        irlpro_qr: bool,
        /// IRL Pro ingest observed.
        #[arg(long)]
        irlpro_ingest: bool,
        /// BELABOX profile generated.
        #[arg(long)]
        belabox_profile: bool,
        /// BELABOX config reviewed.
        #[arg(long)]
        belabox_config: bool,
        /// BELABOX ingest observed.
        #[arg(long)]
        belabox_ingest: bool,
        /// MediaMTX SRT path active.
        #[arg(long)]
        mediamtx_path: bool,
        /// MediaMTX/relay metrics seen.
        #[arg(long)]
        mediamtx_metrics: bool,
        /// OBS connected.
        #[arg(long)]
        obs_connected: bool,
        /// OBS media source observed.
        #[arg(long)]
        obs_source: bool,
        /// Healthy health state observed.
        #[arg(long)]
        healthy_state: bool,
        /// Brownout health state observed.
        #[arg(long)]
        brownout_state: bool,
        /// BRB/fallback scene observed.
        #[arg(long)]
        brb_scene: bool,
        /// Recovery state observed.
        #[arg(long)]
        recovery_state: bool,
        /// Support bundle captured.
        #[arg(long)]
        support_bundle: bool,
        /// Artifacts were redacted.
        #[arg(long)]
        secrets_redacted: bool,
        /// Field report was written.
        #[arg(long)]
        field_report: bool,
    },
    /// Disk artifact planning and materialization helpers.
    Artifacts {
        /// Artifact subcommand.
        #[command(subcommand)]
        command: ArtifactCommand,
    },
    /// V1/public-beta feature helpers.
    V1 {
        /// V1 subcommand.
        #[command(subcommand)]
        command: V1Command,
    },
    /// Print the exact initial features.
    Features,
    /// Generate a sample encoder profile.
    Profile {
        /// Encoder.
        #[arg(long, value_enum, default_value = "moblin")]
        encoder: EncoderArg,
        /// Protocol.
        #[arg(long, value_enum, default_value = "srt")]
        protocol: ProtocolArg,
        /// Host.
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        /// Port.
        #[arg(long, default_value_t = 9000)]
        port: u16,
        /// Stream ID.
        #[arg(long, default_value = "main")]
        stream_id: String,
    },
    /// Metrics parsing, polling, and simulation helpers.
    Metrics {
        /// Metrics subcommand.
        #[command(subcommand)]
        command: MetricsCommand,
    },
    /// Relay/media-router planning helpers.
    Relay {
        /// Relay subcommand.
        #[command(subcommand)]
        command: RelayCommand,
    },
}

/// V1/public-beta CLI commands.
#[derive(Debug, Subcommand)]
enum V1Command {
    /// Print all public-beta feature areas.
    Features,
    /// Print the consolidated implementation summary.
    Summary,
    /// Print sample readiness evidence JSON.
    SampleEvidence,
    /// Evaluate readiness evidence from a JSON file, or sample evidence when omitted.
    Readiness {
        /// Optional evidence JSON file.
        #[arg(long)]
        file: Option<PathBuf>,
    },
    /// Print or materialize the v1/public-beta package layout.
    Package {
        /// Output root.
        #[arg(long, default_value = "artifacts/v1-public-beta")]
        root: String,
        /// Materialize the package layout.
        #[arg(long)]
        materialize: bool,
        /// Overwrite existing files.
        #[arg(long)]
        overwrite: bool,
    },
}

/// Artifact CLI commands.
#[derive(Debug, Subcommand)]
enum ArtifactCommand {
    /// Print the disk artifact plan.
    Plan {
        /// Optional config path.
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Materialize fallback/browser-source assets.
    MaterializeFallback {
        /// Optional config path.
        #[arg(long)]
        config: Option<PathBuf>,
    },
    /// Print or materialize OBS scene/source template JSON.
    ObsTemplate {
        /// Optional config path.
        #[arg(long)]
        config: Option<PathBuf>,
        /// Write the template under artifacts.obs_templates_dir.
        #[arg(long)]
        materialize: bool,
    },
    /// Export a static redacted support bundle from config context.
    SupportBundle {
        /// Optional config path.
        #[arg(long)]
        config: Option<PathBuf>,
        /// Optional field report markdown file.
        #[arg(long)]
        field_report: Option<PathBuf>,
    },
    /// Print or materialize the alpha source package layout.
    AlphaLayout {
        /// Optional config path.
        #[arg(long)]
        config: Option<PathBuf>,
        /// Write the package layout directories and sample files.
        #[arg(long)]
        materialize: bool,
    },
}

/// Metrics CLI commands.
#[derive(Debug, Subcommand)]
enum MetricsCommand {
    /// Print a deterministic simulated metrics snapshot.
    Simulate {
        /// Scenario: healthy, degraded, brownout, or offline.
        #[arg(long, default_value = "healthy")]
        scenario: String,
    },
    /// Parse Prometheus text from a file or stdin.
    Parse {
        /// Optional metrics text file. Reads stdin when omitted.
        #[arg(long)]
        file: Option<PathBuf>,
    },
}

/// Relay CLI commands.
#[derive(Debug, Subcommand)]
enum RelayCommand {
    /// Print the redacted relay plan for the current config.
    Plan {
        /// Optional config path.
        #[arg(long)]
        config: Option<PathBuf>,
    },
}

/// CLI OBS adapter enum.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum ObsAdapterArg {
    /// Dry-run adapter.
    DryRun,
    /// OBS WebSocket v5 adapter.
    WebSocket,
}

impl From<ObsAdapterArg> for ObsAdapterKind {
    fn from(value: ObsAdapterArg) -> Self {
        match value {
            ObsAdapterArg::DryRun => Self::DryRun,
            ObsAdapterArg::WebSocket => Self::WebSocket,
        }
    }
}

/// CLI encoder enum.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum EncoderArg {
    /// Moblin.
    Moblin,
    /// IRL Pro.
    IrlPro,
    /// Larix.
    Larix,
    /// BELABOX.
    Belabox,
}

impl From<EncoderArg> for EncoderKind {
    fn from(value: EncoderArg) -> Self {
        match value {
            EncoderArg::Moblin => Self::Moblin,
            EncoderArg::IrlPro => Self::IrlPro,
            EncoderArg::Larix => Self::Larix,
            EncoderArg::Belabox => Self::Belabox,
        }
    }
}

/// CLI protocol enum.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum ProtocolArg {
    /// SRT.
    Srt,
    /// SRTLA.
    Srtla,
    /// SRTLA2.
    Srtla2,
    /// RTMP.
    Rtmp,
    /// RTMPS.
    Rtmps,
    /// RIST.
    Rist,
    /// WHIP.
    Whip,
}

impl From<ProtocolArg> for Protocol {
    fn from(value: ProtocolArg) -> Self {
        match value {
            ProtocolArg::Srt => Self::Srt,
            ProtocolArg::Srtla => Self::Srtla,
            ProtocolArg::Srtla2 => Self::Srtla2,
            ProtocolArg::Rtmp => Self::Rtmp,
            ProtocolArg::Rtmps => Self::Rtmps,
            ProtocolArg::Rist => Self::Rist,
            ProtocolArg::Whip => Self::Whip,
        }
    }
}

/// CLI deployment mode enum.
#[derive(Debug, Clone, Copy, ValueEnum)]
enum DeploymentModeArg {
    /// Direct phone/backpack to local OBS host.
    LocalDirect,
    /// Friend/moderator relay.
    FriendRelay,
    /// Cheap VPS relay.
    VpsRelay,
    /// Backpack encoder workflow.
    BackpackEncoder,
}

impl From<DeploymentModeArg> for openirl_core::DeploymentMode {
    fn from(value: DeploymentModeArg) -> Self {
        match value {
            DeploymentModeArg::LocalDirect => Self::LocalDirect,
            DeploymentModeArg::FriendRelay => Self::FriendRelay,
            DeploymentModeArg::VpsRelay => Self::VpsRelay,
            DeploymentModeArg::BackpackEncoder => Self::BackpackEncoder,
        }
    }
}

/// API shared state.
#[derive(Clone)]
struct ApiState {
    started_at: OffsetDateTime,
    config: AppConfig,
    health_engine: Arc<RwLock<HealthEngine>>,
    obs: Arc<dyn ObsController>,
    relay: RelayRegistry,
    metrics_state: Arc<RwLock<RelayMetricsState>>,
    last_metrics_snapshot: Arc<RwLock<Option<RelayMetricsSnapshot>>>,
    scene_bundle: SceneBundle,
    session: Arc<RwLock<SessionStore>>,
    markers: Arc<RwLock<ClipMarkerStore>>,
}

/// Small multi-process relay registry used by relay endpoints.
#[derive(Clone)]
struct RelayRegistry {
    processes: Arc<Vec<Arc<RelaySupervisor>>>,
}

impl RelayRegistry {
    fn new(configs: Vec<RuntimeRelayProcessConfig>) -> Self {
        let processes = configs
            .into_iter()
            .map(|config| Arc::new(RelaySupervisor::new(config)))
            .collect();
        Self {
            processes: Arc::new(processes),
        }
    }

    async fn plans(&self) -> Vec<RelayLaunchPlan> {
        let mut plans = Vec::new();
        for process in self.processes.iter() {
            plans.push(process.plan().await);
        }
        plans
    }

    async fn statuses(&self) -> Vec<serde_json::Value> {
        let mut statuses = Vec::new();
        for process in self.processes.iter() {
            statuses.push(relay_result_json(process.status().await));
        }
        statuses
    }

    async fn start_all(&self) -> Vec<serde_json::Value> {
        let mut results = Vec::new();
        for process in self.processes.iter() {
            results.push(relay_result_json(process.start().await));
        }
        results
    }

    async fn stop_all(&self) -> Vec<serde_json::Value> {
        let mut results = Vec::new();
        for process in self.processes.iter() {
            results.push(relay_result_json(process.stop().await));
        }
        results
    }

    async fn restart_all(&self) -> Vec<serde_json::Value> {
        let mut results = Vec::new();
        for process in self.processes.iter() {
            results.push(relay_result_json(process.restart().await));
        }
        results
    }

    async fn by_name(&self, name: &str) -> Option<Arc<RelaySupervisor>> {
        for process in self.processes.iter() {
            if process.name().await == name {
                return Some(process.clone());
            }
        }
        None
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();
    let cli = Cli::parse();

    match cli.command {
        Command::Serve {
            config,
            bind,
            obs_adapter,
        } => serve(config, bind, obs_adapter).await,
        Command::CheckConfig { config } => check_config(config),
        Command::InstallPlan { config } => {
            let config = load_config_or_default(config)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&default_windows_service_plan(&config))?
            );
            Ok(())
        }
        Command::DesktopPlan { dashboard_url } => {
            println!(
                "{}",
                serde_json::to_string_pretty(&default_desktop_shell_plan(dashboard_url))?
            );
            Ok(())
        }
        Command::Onboarding {
            config,
            encoder,
            protocol,
            mode,
            host,
            no_qr,
        } => {
            let config = load_config_or_default(config)?;
            let request = OnboardingRequest {
                encoder: encoder.into(),
                protocol: protocol.map(Into::into),
                deployment_mode: mode.into(),
                public_host: host,
                bitrate_kbps: 4_500,
                include_qr_svg: !no_qr,
            };
            let plan = plan_onboarding(&config, &request)?;
            println!("{}", serde_json::to_string_pretty(&plan)?);
            Ok(())
        }
        Command::ReleaseManifest { config } => {
            let config = load_config_or_default(config)?;
            let manifest = build_release_manifest(
                env!("CARGO_PKG_VERSION"),
                OPENIRL_SCHEMA_REVISION,
                &validate_config(&config),
            );
            println!("{}", serde_json::to_string_pretty(&manifest)?);
            Ok(())
        }
        Command::AlphaPlan => {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "plan": build_alpha_validation_plan(OPENIRL_SCHEMA_REVISION),
                    "operator_checklist": build_operator_checklist()
                }))?
            );
            Ok(())
        }
        Command::AlphaEvidence {
            static_validation,
            rust_ci,
            config_ok,
            agent_health,
            dashboard_loaded,
            obs_connected,
            obs_scenes,
            obs_stream_controls,
            replay_save,
            recording_controls,
            profile_qr,
            metrics_poll,
            portable_built,
            msi_reviewed,
        } => {
            let evidence = AlphaEvidenceInput {
                static_validation_passed: static_validation,
                rust_ci_passed: rust_ci,
                config_ok,
                agent_health_ok: agent_health,
                dashboard_loaded,
                obs_connected,
                obs_scene_switch_verified: obs_scenes,
                obs_streaming_start_stop_verified: obs_stream_controls,
                replay_save_verified: replay_save,
                recording_controls_verified: recording_controls,
                profile_qr_tested: profile_qr,
                metrics_poll_verified: metrics_poll,
                windows_portable_built: portable_built,
                windows_msi_reviewed: msi_reviewed,
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&evaluate_alpha_evidence(&evidence))?
            );
            Ok(())
        }
        Command::FieldPlan => {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "plan": build_field_validation_plan(OPENIRL_SCHEMA_REVISION),
                    "device_checklists": build_device_checklists()
                }))?
            );
            Ok(())
        }
        Command::FieldSampleEvidence => {
            println!(
                "{}",
                serde_json::to_string_pretty(&sample_field_evidence())?
            );
            Ok(())
        }
        Command::FieldEvidence {
            static_validation,
            rust_ci,
            windows_alpha_ready,
            moblin_profile,
            moblin_qr,
            moblin_ingest,
            irlpro_profile,
            irlpro_qr,
            irlpro_ingest,
            belabox_profile,
            belabox_config,
            belabox_ingest,
            mediamtx_path,
            mediamtx_metrics,
            obs_connected,
            obs_source,
            healthy_state,
            brownout_state,
            brb_scene,
            recovery_state,
            support_bundle,
            secrets_redacted,
            field_report,
        } => {
            let evidence = FieldEvidenceInput {
                static_validation_passed: static_validation,
                rust_ci_passed: rust_ci,
                windows_alpha_ready,
                moblin_profile_generated: moblin_profile,
                moblin_qr_scanned: moblin_qr,
                moblin_ingest_seen: moblin_ingest,
                irlpro_profile_generated: irlpro_profile,
                irlpro_qr_scanned: irlpro_qr,
                irlpro_ingest_seen: irlpro_ingest,
                belabox_profile_generated: belabox_profile,
                belabox_config_reviewed: belabox_config,
                belabox_ingest_seen: belabox_ingest,
                mediamtx_srt_path_active: mediamtx_path,
                mediamtx_metrics_seen: mediamtx_metrics,
                obs_connected,
                obs_media_source_seen: obs_source,
                healthy_state_seen: healthy_state,
                brownout_state_seen: brownout_state,
                brb_scene_seen: brb_scene,
                recovery_state_seen: recovery_state,
                support_bundle_captured: support_bundle,
                secrets_redacted,
                field_report_written: field_report,
            };
            println!(
                "{}",
                serde_json::to_string_pretty(&evaluate_field_evidence(&evidence))?
            );
            Ok(())
        }
        Command::Artifacts { command } => match command {
            ArtifactCommand::Plan { config } => {
                let config = load_config_or_default(config)?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&artifact_plan_json(&config))?
                );
                Ok(())
            }
            ArtifactCommand::MaterializeFallback { config } => {
                let config = load_config_or_default(config)?;
                let plan = default_fallback_asset_plan(
                    &config.scene_bundle(),
                    config.artifacts.fallback_assets_dir.clone(),
                );
                let result = materialize_fallback_assets(&plan)?;
                println!("{}", serde_json::to_string_pretty(&result)?);
                Ok(())
            }
            ArtifactCommand::ObsTemplate {
                config,
                materialize,
            } => {
                let config = load_config_or_default(config)?;
                let plan = build_obs_scene_template_plan(
                    &config.scene_bundle(),
                    config.artifacts.fallback_assets_dir.clone(),
                    live_input_url_from_config(&config),
                );
                if materialize {
                    let result =
                        materialize_obs_scene_template(obs_template_output_path(&config), &plan)?;
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("{}", serde_json::to_string_pretty(&plan)?);
                }
                Ok(())
            }
            ArtifactCommand::SupportBundle {
                config,
                field_report,
            } => {
                let config = load_config_or_default(config)?;
                let field_report_markdown = read_optional_text(field_report)?.map(|report| {
                    redact_support_text(&report, config.security.support_bundle_redact_ips)
                });
                let payload = serde_json::json!({
                    "generated_from": "openirl-agent artifacts support-bundle",
                    "schema_revision": OPENIRL_SCHEMA_REVISION,
                    "config": config.redacted(),
                    "config_validation": validate_config(&config),
                });
                let payload =
                    scrub_support_bundle_value(payload, config.security.support_bundle_redact_ips);
                let request = SupportBundleExportRequest {
                    output_dir: config.artifacts.support_bundles_dir.clone(),
                    field_report_markdown,
                };
                let result = export_support_bundle(&request, &payload, None)?;
                println!("{}", serde_json::to_string_pretty(&result)?);
                Ok(())
            }
            ArtifactCommand::AlphaLayout {
                config,
                materialize,
            } => {
                let config = load_config_or_default(config)?;
                let layout =
                    alpha_source_package_layout(config.artifacts.alpha_package_dir.clone());
                if materialize {
                    let result = materialize_alpha_source_layout(&layout)?;
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("{}", serde_json::to_string_pretty(&layout)?);
                }
                Ok(())
            }
        },
        Command::V1 { command } => match command {
            V1Command::Features => {
                println!("{}", serde_json::to_string_pretty(&build_v1_features())?);
                Ok(())
            }
            V1Command::Summary => {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&build_v1_implementation_summary())?
                );
                Ok(())
            }
            V1Command::SampleEvidence => {
                println!("{}", serde_json::to_string_pretty(&sample_v1_evidence())?);
                Ok(())
            }
            V1Command::Readiness { file } => {
                let evidence = read_v1_evidence(file)?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&evaluate_v1_evidence(&evidence))?
                );
                Ok(())
            }
            V1Command::Package {
                root,
                materialize,
                overwrite,
            } => {
                let layout = default_v1_package_layout(root);
                if materialize {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&materialize_v1_package(&layout, overwrite)?)?
                    );
                } else {
                    println!("{}", serde_json::to_string_pretty(&layout)?);
                }
                Ok(())
            }
        },
        Command::Features => {
            println!("Exact initial feature count: {INITIAL_FEATURE_AREA_COUNT}");
            println!("Current schema revision: {OPENIRL_SCHEMA_REVISION}");
            println!("{}", serde_json::to_string_pretty(&feature_areas())?);
            Ok(())
        }
        Command::Profile {
            encoder,
            protocol,
            host,
            port,
            stream_id,
        } => {
            let request = ProfileRequest {
                encoder: encoder.into(),
                protocol: protocol.into(),
                host,
                port,
                stream_id,
                passphrase: Some("replace-me".to_string()),
                latency_ms: 1800,
                bitrate_kbps: 4500,
            };
            let profile = generate_profile(&request)?;
            println!("{}", serde_json::to_string_pretty(&profile)?);
            Ok(())
        }
        Command::Metrics { command } => match command {
            MetricsCommand::Simulate { scenario } => {
                let scenario = MetricsScenario::parse(&scenario)
                    .ok_or_else(|| anyhow::anyhow!("unknown metrics scenario: {scenario}"))?;
                let snapshot = simulated_relay_snapshot(scenario, now_ms());
                println!("{}", serde_json::to_string_pretty(&snapshot)?);
                Ok(())
            }
            MetricsCommand::Parse { file } => {
                let input = read_metrics_input(file)?;
                let document = parse_prometheus_text(&input)?;
                println!("{}", serde_json::to_string_pretty(&document)?);
                Ok(())
            }
        },
        Command::Relay { command } => match command {
            RelayCommand::Plan { config } => {
                let config = load_config_or_default(config)?;
                let registry = RelayRegistry::new(relay_process_configs_from_config(&config));
                println!("{}", serde_json::to_string_pretty(&registry.plans().await)?);
                Ok(())
            }
        },
    }
}

fn check_config(config_path: Option<PathBuf>) -> anyhow::Result<()> {
    let config = load_config_or_default(config_path)?;
    let report = validate_config(&config);
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "config": config.redacted(),
            "validation": report,
            "schema_revision": OPENIRL_SCHEMA_REVISION
        }))?
    );
    Ok(())
}

fn read_metrics_input(file: Option<PathBuf>) -> anyhow::Result<String> {
    match file {
        Some(path) => std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read metrics file at {}", path.display())),
        None => {
            let mut input = String::new();
            std::io::stdin().read_to_string(&mut input)?;
            Ok(input)
        }
    }
}

fn read_optional_text(path: Option<PathBuf>) -> anyhow::Result<Option<String>> {
    match path {
        Some(path) => std::fs::read_to_string(&path)
            .map(Some)
            .with_context(|| format!("failed to read text file at {}", path.display())),
        None => Ok(None),
    }
}

fn read_v1_evidence(path: Option<PathBuf>) -> anyhow::Result<V1EvidenceInput> {
    match path {
        Some(path) => {
            let raw = std::fs::read_to_string(&path).with_context(|| {
                format!("failed to read v1 evidence file at {}", path.display())
            })?;
            serde_json::from_str(&raw)
                .with_context(|| format!("failed to decode v1 evidence JSON at {}", path.display()))
        }
        None => Ok(sample_v1_evidence()),
    }
}

fn load_config_or_default(config_path: Option<PathBuf>) -> anyhow::Result<AppConfig> {
    match config_path {
        Some(path) => load_config(&path)
            .with_context(|| format!("failed to load config at {}", path.display())),
        None => Ok(AppConfig::default()),
    }
}

async fn serve(
    config_path: Option<PathBuf>,
    bind_override: Option<SocketAddr>,
    obs_adapter_override: Option<ObsAdapterArg>,
) -> anyhow::Result<()> {
    let mut config = load_config_or_default(config_path)?;

    if let Some(adapter) = obs_adapter_override {
        config.obs.adapter = adapter.into();
    }

    if let Some(bind) = bind_override {
        config.api.bind = bind;
    }

    let validation_report = validate_config(&config);
    log_validation_report(&validation_report);
    if !validation_report.ok {
        let codes = validation_report
            .issues
            .iter()
            .filter(|issue| issue.severity == openirl_config::ValidationSeverity::Error)
            .map(|issue| issue.code.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        bail!("refusing to start with unsafe config: {codes}");
    }

    let bind = config.api.bind;
    let scene_bundle = config.scene_bundle();
    let obs = build_obs_controller(&config);
    if config.obs.create_missing_scenes {
        obs.ensure_scene_bundle(&scene_bundle).await?;
    }

    let relay = RelayRegistry::new(relay_process_configs_from_config(&config));
    if config.relay.enabled && config.relay.auto_start {
        let auto_start_results = relay.start_all().await;
        tracing::info!(results = %serde_json::json!(auto_start_results), "relay auto-start attempted");
    }

    let history_limit = config
        .api
        .history_limit
        .max(config.runtime.history_limit)
        .max(1);
    let state = ApiState {
        started_at: OffsetDateTime::now_utc(),
        config,
        health_engine: Arc::new(RwLock::new(HealthEngine::new())),
        obs,
        relay,
        metrics_state: Arc::new(RwLock::new(RelayMetricsState::new())),
        last_metrics_snapshot: Arc::new(RwLock::new(None)),
        scene_bundle,
        session: Arc::new(RwLock::new(SessionStore::with_limit(history_limit))),
        markers: Arc::new(RwLock::new(ClipMarkerStore::new(history_limit))),
    };

    if state.config.metrics.enabled && state.config.metrics.auto_poll {
        spawn_metrics_poll_loop(state.clone());
    }

    let cors = cors_layer(&state.config);

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/state", get(api_state))
        .route("/api/config/redacted", get(api_config_redacted))
        .route("/api/config/validation", get(api_config_validation))
        .route("/api/runtime/readiness", get(api_runtime_readiness))
        .route("/api/auth/status", get(api_auth_status))
        .route("/api/auth/check", post(api_auth_check))
        .route(
            "/api/install/plan",
            get(api_install_plan).post(api_install_plan_custom),
        )
        .route("/api/desktop/plan", get(api_desktop_plan))
        .route("/api/onboarding/options", get(api_onboarding_options))
        .route("/api/onboarding/plan", post(api_onboarding_plan))
        .route(
            "/api/onboarding/quickstart",
            post(api_onboarding_quickstart),
        )
        .route("/api/relay/status", get(api_relay_status))
        .route("/api/relay/readiness", get(api_relay_readiness))
        .route("/api/relay/discovery", get(api_relay_discovery))
        .route("/api/relay/plan", get(api_relay_plan))
        .route("/api/relay/start", post(api_relay_start_all))
        .route("/api/relay/stop", post(api_relay_stop_all))
        .route("/api/relay/restart", post(api_relay_restart_all))
        .route("/api/relay/refresh", post(api_relay_refresh))
        .route("/api/relay/start/{name}", post(api_relay_start_named))
        .route("/api/relay/stop/{name}", post(api_relay_stop_named))
        .route("/api/relay/restart/{name}", post(api_relay_restart_named))
        .route("/api/metrics/sources", get(api_metrics_sources))
        .route("/api/metrics/latest", get(api_metrics_latest))
        .route("/api/metrics/scenarios", get(api_metrics_scenarios))
        .route(
            "/api/metrics/simulate/{scenario}",
            post(api_metrics_simulate),
        )
        .route(
            "/api/metrics/ingest-prometheus/{source}",
            post(api_metrics_ingest_prometheus),
        )
        .route(
            "/api/metrics/ingest-srtla-log/{source}",
            post(api_metrics_ingest_srtla_log),
        )
        .route("/api/metrics/poll", post(api_metrics_poll_default))
        .route("/api/metrics/poll/{source}", post(api_metrics_poll_named))
        .route(
            "/api/metrics/poll-api/{source}",
            post(api_metrics_poll_api_named),
        )
        .route("/api/features/areas", get(api_feature_areas))
        .route("/api/evaluate", post(api_evaluate))
        .route("/api/profile", post(api_profile))
        .route("/api/profile/qr", post(api_profile_qr))
        .route("/api/profiles/defaults", get(api_profile_defaults))
        .route("/api/obs/status", get(api_obs_status))
        .route("/api/obs/actions", get(api_obs_actions))
        .route("/api/obs/template", get(api_obs_template))
        .route(
            "/api/obs/template/materialize",
            post(api_obs_template_materialize),
        )
        .route("/api/obs/template/apply", post(api_obs_template_apply))
        .route("/api/obs/switch/{role}", post(api_obs_switch))
        .route("/api/obs/start", post(api_obs_start))
        .route("/api/obs/stop", post(api_obs_stop))
        .route("/api/production/plan", get(api_production_plan))
        .route("/api/production/markers", get(api_production_markers))
        .route("/api/production/marker", post(api_production_marker))
        .route(
            "/api/production/markers/clear",
            post(api_production_markers_clear),
        )
        .route(
            "/api/production/replay/save",
            post(api_production_save_replay),
        )
        .route(
            "/api/production/recording/start",
            post(api_production_start_recording),
        )
        .route(
            "/api/production/recording/stop",
            post(api_production_stop_recording),
        )
        .route("/api/moderation/command", post(api_moderation_command))
        .route("/api/release/manifest", get(api_release_manifest))
        .route("/api/release/gates", get(api_release_gates))
        .route("/api/release/smoke-plan", get(api_release_smoke_plan))
        .route("/api/alpha/validation-plan", get(api_alpha_validation_plan))
        .route(
            "/api/alpha/operator-checklist",
            get(api_alpha_operator_checklist),
        )
        .route("/api/alpha/readiness", get(api_alpha_readiness))
        .route("/api/alpha/evidence", post(api_alpha_evidence))
        .route("/api/field/validation-plan", get(api_field_validation_plan))
        .route(
            "/api/field/device-checklists",
            get(api_field_device_checklists),
        )
        .route("/api/field/sample-evidence", get(api_field_sample_evidence))
        .route("/api/field/readiness", get(api_field_readiness))
        .route("/api/field/evidence", post(api_field_evidence))
        .route("/api/field/report/export", post(api_field_report_export))
        .route("/api/artifacts/plan", get(api_artifacts_plan))
        .route(
            "/api/artifacts/fallback-assets/materialize",
            post(api_materialize_fallback_assets),
        )
        .route("/api/alpha/package-layout", get(api_alpha_package_layout))
        .route(
            "/api/alpha/package-layout/materialize",
            post(api_alpha_package_layout_materialize),
        )
        .route("/api/v1/features", get(api_v1_features))
        .route("/api/v1/summary", get(api_v1_summary))
        .route("/api/v1/sample-evidence", get(api_v1_sample_evidence))
        .route(
            "/api/v1/readiness",
            get(api_v1_readiness).post(api_v1_readiness_with_evidence),
        )
        .route("/api/v1/package-layout", get(api_v1_package_layout))
        .route(
            "/api/v1/package-layout/materialize",
            post(api_v1_package_layout_materialize),
        )
        .route("/api/session", get(api_session))
        .route("/api/session/report", get(api_session_report))
        .route("/api/session/support-bundle", get(api_support_bundle))
        .route(
            "/api/session/support-bundle/export",
            post(api_support_bundle_export),
        )
        .route("/api/session/reset", post(api_session_reset))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_api_auth,
        ))
        .fallback_service(ServeDir::new("apps/openirl-agent/static"))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    tracing::info!(%bind, "serving OpenIRL agent");
    let listener = tokio::net::TcpListener::bind(bind).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}

fn log_validation_report(report: &ConfigValidationReport) {
    for issue in &report.issues {
        match issue.severity {
            openirl_config::ValidationSeverity::Info => {
                tracing::info!(code = %issue.code, message = %issue.message, "config validation info");
            }
            openirl_config::ValidationSeverity::Warning => {
                tracing::warn!(code = %issue.code, message = %issue.message, "config validation warning");
            }
            openirl_config::ValidationSeverity::Error => {
                tracing::error!(code = %issue.code, message = %issue.message, "config validation error");
            }
        }
    }
}

fn cors_layer(config: &AppConfig) -> CorsLayer {
    let allowed_origins = config
        .api
        .cors_allowed_origins
        .iter()
        .filter_map(|origin| HeaderValue::from_str(origin.trim()).ok())
        .collect::<Vec<_>>();

    if allowed_origins.is_empty() {
        CorsLayer::new()
    } else {
        CorsLayer::new()
            .allow_origin(AllowOrigin::list(allowed_origins))
            .allow_methods([Method::GET, Method::POST])
            .allow_headers([AUTHORIZATION, CONTENT_TYPE])
    }
}

fn build_obs_controller(config: &AppConfig) -> Arc<dyn ObsController> {
    match config.obs.adapter {
        ObsAdapterKind::DryRun => Arc::new(DryRunObsController::default()),
        ObsAdapterKind::WebSocket => {
            let password = std::env::var(&config.obs.password_env).ok();
            Arc::new(ObsWebSocketController::new(ObsWebSocketConfig {
                url: format!("ws://{}:{}", config.obs.host, config.obs.port),
                password,
                rpc_version: config.obs.rpc_version,
                request_timeout_ms: config.obs.request_timeout_ms,
            }))
        }
    }
}

fn relay_process_configs_from_config(config: &AppConfig) -> Vec<RuntimeRelayProcessConfig> {
    if config.relay.processes.is_empty() {
        return vec![RuntimeRelayProcessConfig::disabled()];
    }

    config
        .relay
        .processes
        .iter()
        .map(|process| RuntimeRelayProcessConfig {
            name: process.name.clone(),
            enabled: config.relay.enabled
                && config.relay.supervisor_mode == RelaySupervisorMode::Process
                && process.enabled,
            auto_start: config.relay.enabled
                && config.relay.supervisor_mode == RelaySupervisorMode::Process
                && config.relay.auto_start
                && process.enabled,
            mode: config.relay.mode,
            backend: backend_from_config_process(process.kind, &config.relay.media_router),
            executable: relay_executable_from_config_process(process),
            args: relay_args_from_config_process(config, process),
            working_dir: process.working_dir.as_ref().map(PathBuf::from),
            env: process
                .env
                .iter()
                .map(|env_pair| RelayEnvPair {
                    key: env_pair.key.clone(),
                    value: env_pair.value.clone(),
                })
                .collect(),
            restart_on_exit: process.restart_on_exit,
            metrics_url: media_metrics_url(config, process.kind),
            api_url: media_api_url(config, process.kind),
            log_tail_limit: 200,
            redact_logs: config.security.redact_logs,
        })
        .collect()
}

fn relay_executable_from_config_process(process: &openirl_config::RelayProcessConfig) -> String {
    if process.executable_env.trim().is_empty() {
        return process.executable.clone();
    }

    std::env::var(&process.executable_env).unwrap_or_else(|_| process.executable.clone())
}

fn relay_args_from_config_process(
    config: &AppConfig,
    process: &openirl_config::RelayProcessConfig,
) -> Vec<String> {
    if process.kind == ConfigRelayProcessKind::MediaMtx && process.args.is_empty() {
        vec![config.relay.mediamtx_config_path.clone()]
    } else {
        process.args.clone()
    }
}

fn relay_credential_plan_from_config(
    config: &AppConfig,
) -> openirl_relay_control::RelayCredentialPlan {
    build_credential_plan(
        config.ingest.public_host.clone(),
        config.ingest.srt_port,
        config.ingest.srtla_port,
        "openirl-main",
        config.relay.passphrase_env.clone(),
    )
}

fn media_metrics_url(config: &AppConfig, kind: ConfigRelayProcessKind) -> Option<String> {
    if kind == ConfigRelayProcessKind::MediaMtx {
        Some(config.relay.mediamtx_metrics_url.clone())
    } else {
        None
    }
}

fn media_api_url(config: &AppConfig, kind: ConfigRelayProcessKind) -> Option<String> {
    if kind == ConfigRelayProcessKind::MediaMtx {
        Some(config.relay.mediamtx_api_url.clone())
    } else {
        None
    }
}

fn backend_from_config_process(kind: ConfigRelayProcessKind, router: &str) -> RelayBackend {
    match kind {
        ConfigRelayProcessKind::MediaMtx => RelayBackend::MediaMtx,
        ConfigRelayProcessKind::SrtlaReceive => RelayBackend::SrtlaReceive,
        ConfigRelayProcessKind::SrtlaSend => RelayBackend::SrtlaSend,
        ConfigRelayProcessKind::SrtLiveTransmit => RelayBackend::SrtLiveTransmit,
        ConfigRelayProcessKind::Custom => RelayBackend::from_router_name(router),
    }
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    started_at: String,
    feature_area_count: u8,
    schema_revision: u16,
}

async fn health(State(state): State<ApiState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        started_at: state.started_at.to_string(),
        feature_area_count: INITIAL_FEATURE_AREA_COUNT,
        schema_revision: OPENIRL_SCHEMA_REVISION,
    })
}

async fn api_feature_areas() -> Json<Vec<openirl_core::FeatureArea>> {
    Json(feature_areas())
}

async fn api_config_redacted(
    State(state): State<ApiState>,
) -> Json<openirl_config::RedactedAppConfig> {
    Json(state.config.redacted())
}

async fn api_config_validation(State(state): State<ApiState>) -> Json<ConfigValidationReport> {
    Json(validate_config(&state.config))
}

async fn api_runtime_readiness(State(state): State<ApiState>) -> impl IntoResponse {
    let validation = validate_config(&state.config);
    let obs_status_result = state.obs.status().await;
    let relay_plans = state.relay.plans().await;
    let relay_statuses = state.relay.statuses().await;
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();

    if !validation.ok {
        blockers.push("config validation has blocking errors".to_string());
    }

    let obs_status = match obs_status_result {
        Ok(status) => serde_json::json!(status),
        Err(error) => {
            blockers.push(format!("OBS adapter is not ready: {error}"));
            serde_json::json!({ "error": error.to_string() })
        }
    };

    if matches!(state.config.obs.adapter, ObsAdapterKind::DryRun) {
        warnings.push(
            "OBS is in dry-run mode; live OBS automation has not been exercised.".to_string(),
        );
    }

    if state.config.runtime.demo_event_loop {
        warnings.push("demo event loop is enabled; disable it for real IRL sessions.".to_string());
    }

    if !state.config.metrics.enabled {
        warnings.push(
            "metrics ingestion is disabled; health samples must be posted manually.".to_string(),
        );
    }

    for plan in &relay_plans {
        if plan.enabled && !plan.executable.found {
            blockers.push(format!(
                "relay process {} is enabled but executable was not found",
                plan.name
            ));
        }
        if plan.enabled && !plan.auto_start {
            warnings.push(format!(
                "relay process {} is enabled but auto_start is off",
                plan.name
            ));
        }
    }

    let agent_ready = blockers.is_empty();
    let source_validated = false;
    let live_ready = false;
    warnings.push(
        "source_validated and live_ready are never inferred from runtime state; use validation commands and live smoke evidence."
            .to_string(),
    );

    Json(serde_json::json!({
        "ready": agent_ready,
        "ready_scope": "agent",
        "agent_ready": agent_ready,
        "source_validated": source_validated,
        "live_ready": live_ready,
        "schema_revision": OPENIRL_SCHEMA_REVISION,
        "config": validation,
        "obs": obs_status,
        "relay": {
            "enabled": state.config.relay.enabled,
            "plans": relay_plans,
            "statuses": relay_statuses
        },
        "metrics": {
            "enabled": state.config.metrics.enabled,
            "source": state.config.metrics.source,
            "latest": state.last_metrics_snapshot.read().await.clone(),
            "sources": metric_source_configs(&state).await
        },
        "blockers": blockers,
        "warnings": warnings,
        "scene_bundle": state.scene_bundle,
        "profile_presets_available": support_matrix().len()
    }))
}

#[derive(Debug, Clone, Deserialize)]
struct AuthCheckRequest {
    authorization: Option<String>,
    is_loopback_request: Option<bool>,
}

async fn api_auth_status(State(state): State<ApiState>) -> impl IntoResponse {
    let policy = auth_policy_from_config(&state.config);
    let token_value = std::env::var(&policy.token_env).ok();
    Json(auth_status(&policy, token_value.as_deref()))
}

async fn api_auth_check(
    State(state): State<ApiState>,
    Json(request): Json<AuthCheckRequest>,
) -> impl IntoResponse {
    let policy = auth_policy_from_config(&state.config);
    let token_value = std::env::var(&policy.token_env).ok();
    let decision = verify_authorization_header(
        &policy,
        token_value.as_deref(),
        request.authorization.as_deref(),
        request.is_loopback_request.unwrap_or(true),
    );
    let status = if decision.allowed {
        StatusCode::OK
    } else {
        StatusCode::UNAUTHORIZED
    };
    (status, Json(serde_json::json!(decision)))
}

fn auth_policy_from_config(config: &AppConfig) -> AuthPolicy {
    AuthPolicy {
        enabled: config.security.dashboard_auth_enabled,
        token_env: config.security.dashboard_token_env.clone(),
        allow_loopback_without_token: config.security.allow_loopback_without_token,
        require_for_lan: config.security.require_auth_outside_localhost,
    }
}

async fn require_api_auth(State(state): State<ApiState>, request: Request, next: Next) -> Response {
    let path = request.uri().path();
    if !path.starts_with("/api/") || matches!(path, "/api/auth/status" | "/api/auth/check") {
        return next.run(request).await;
    }

    let policy = auth_policy_from_config(&state.config);
    let token_value = std::env::var(&policy.token_env).ok();
    let authorization = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok());
    let is_loopback_request = request_is_loopback(request.extensions(), &state);
    let decision = verify_authorization_header(
        &policy,
        token_value.as_deref(),
        authorization,
        is_loopback_request,
    );

    if decision.allowed {
        next.run(request).await
    } else {
        (StatusCode::UNAUTHORIZED, Json(serde_json::json!(decision))).into_response()
    }
}

fn request_is_loopback(extensions: &axum::http::Extensions, state: &ApiState) -> bool {
    extensions
        .get::<ConnectInfo<SocketAddr>>()
        .map(|connect_info| connect_info.0.ip().is_loopback())
        .unwrap_or_else(|| state.config.api.bind.ip().is_loopback())
}

#[derive(Debug, Clone, Copy)]
struct ControlAuth {
    role: ModeratorRole,
}

impl FromRequestParts<ApiState> for ControlAuth {
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &ApiState,
    ) -> Result<Self, Self::Rejection> {
        let policy = auth_policy_from_config(&state.config);
        let token_value = std::env::var(&policy.token_env).ok();
        let authorization = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok());
        let is_loopback_request = request_is_loopback(&parts.extensions, state);
        let decision = verify_authorization_header(
            &policy,
            token_value.as_deref(),
            authorization,
            is_loopback_request,
        );

        if decision.allowed {
            Ok(Self {
                role: ModeratorRole::Owner,
            })
        } else {
            Err((StatusCode::UNAUTHORIZED, Json(serde_json::json!(decision))))
        }
    }
}

async fn api_install_plan(State(state): State<ApiState>) -> impl IntoResponse {
    Json(default_windows_service_plan(&state.config))
}

async fn api_install_plan_custom(
    State(state): State<ApiState>,
    _auth: ControlAuth,
    Json(request): Json<InstallPlanRequest>,
) -> impl IntoResponse {
    Json(build_install_plan(&state.config, &request))
}

async fn api_desktop_plan(State(state): State<ApiState>) -> impl IntoResponse {
    let dashboard_url = format!("http://{}/", state.config.api.bind);
    Json(default_desktop_shell_plan(dashboard_url))
}

async fn api_onboarding_options(State(state): State<ApiState>) -> impl IntoResponse {
    Json(serde_json::json!({
        "deployment_modes": onboarding_options(),
        "support_matrix": support_matrix(),
        "default_request": OnboardingRequest::default(),
        "ingest": state.config.ingest.clone(),
        "security": {
            "auth_status_endpoint": "/api/auth/status",
            "token_env": state.config.security.dashboard_token_env.clone()
        }
    }))
}

async fn api_onboarding_plan(
    State(state): State<ApiState>,
    _auth: ControlAuth,
    Json(request): Json<OnboardingRequest>,
) -> impl IntoResponse {
    match plan_onboarding(&state.config, &request) {
        Ok(plan) => (StatusCode::OK, Json(serde_json::json!(plan))),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        ),
    }
}

async fn api_onboarding_quickstart(
    State(state): State<ApiState>,
    _auth: ControlAuth,
) -> impl IntoResponse {
    let request = OnboardingRequest::default();
    match plan_onboarding(&state.config, &request) {
        Ok(plan) => (StatusCode::OK, Json(serde_json::json!(plan))),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        ),
    }
}

async fn api_relay_status(State(state): State<ApiState>) -> impl IntoResponse {
    Json(serde_json::json!({
        "enabled": state.config.relay.enabled,
        "statuses": state.relay.statuses().await
    }))
}

async fn api_relay_readiness(State(state): State<ApiState>) -> impl IntoResponse {
    let plans = state.relay.plans().await;
    let statuses = state.relay.statuses().await;
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();

    if !state.config.relay.enabled {
        warnings.push("relay is disabled in config; local direct mode only".to_string());
    }

    for plan in &plans {
        if plan.enabled && !plan.executable.found {
            blockers.push(format!("{} executable was not found", plan.name));
        }
    }

    Json(serde_json::json!({
        "ready": blockers.is_empty(),
        "enabled": state.config.relay.enabled,
        "plans": plans,
        "statuses": statuses,
        "blockers": blockers,
        "warnings": warnings
    }))
}

async fn api_relay_discovery(State(state): State<ApiState>) -> impl IntoResponse {
    let plans = state.relay.plans().await;
    let discovery = plans
        .iter()
        .map(|plan| {
            serde_json::json!({
                "name": &plan.name,
                "backend": plan.backend,
                "enabled": plan.enabled,
                "executable": &plan.executable
            })
        })
        .collect::<Vec<_>>();
    Json(serde_json::json!({ "executables": discovery }))
}

async fn api_relay_plan(State(state): State<ApiState>) -> impl IntoResponse {
    Json(serde_json::json!({
        "enabled": state.config.relay.enabled,
        "plans": state.relay.plans().await,
        "credentials": relay_credential_plan_from_config(&state.config)
    }))
}

async fn api_relay_start_all(
    State(state): State<ApiState>,
    _auth: ControlAuth,
) -> impl IntoResponse {
    if !state.config.relay.enabled {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "relay is disabled in config" })),
        );
    }
    (
        StatusCode::OK,
        Json(serde_json::json!(state.relay.start_all().await)),
    )
}

async fn api_relay_stop_all(
    State(state): State<ApiState>,
    _auth: ControlAuth,
) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(serde_json::json!(state.relay.stop_all().await)),
    )
}

async fn api_relay_restart_all(
    State(state): State<ApiState>,
    _auth: ControlAuth,
) -> impl IntoResponse {
    if !state.config.relay.enabled {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "relay is disabled in config" })),
        );
    }
    (
        StatusCode::OK,
        Json(serde_json::json!(state.relay.restart_all().await)),
    )
}

async fn api_relay_refresh(State(state): State<ApiState>, _auth: ControlAuth) -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(serde_json::json!(state.relay.statuses().await)),
    )
}

async fn api_relay_start_named(
    State(state): State<ApiState>,
    _auth: ControlAuth,
    Path(name): Path<String>,
) -> impl IntoResponse {
    relay_named_action(&state.relay, &name, RelayAction::Start).await
}

async fn api_relay_stop_named(
    State(state): State<ApiState>,
    _auth: ControlAuth,
    Path(name): Path<String>,
) -> impl IntoResponse {
    relay_named_action(&state.relay, &name, RelayAction::Stop).await
}

async fn api_relay_restart_named(
    State(state): State<ApiState>,
    _auth: ControlAuth,
    Path(name): Path<String>,
) -> impl IntoResponse {
    relay_named_action(&state.relay, &name, RelayAction::Restart).await
}

enum RelayAction {
    Start,
    Stop,
    Restart,
}

async fn relay_named_action(
    registry: &RelayRegistry,
    name: &str,
    action: RelayAction,
) -> (StatusCode, Json<serde_json::Value>) {
    let Some(process) = registry.by_name(name).await else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": format!("unknown relay process: {name}") })),
        );
    };

    let result = match action {
        RelayAction::Start => process.start().await,
        RelayAction::Stop => process.stop().await,
        RelayAction::Restart => process.restart().await,
    };

    match result {
        Ok(status) => (StatusCode::OK, Json(serde_json::json!(status))),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": error.to_string() })),
        ),
    }
}

fn spawn_metrics_poll_loop(state: ApiState) {
    tokio::spawn(async move {
        loop {
            if !state.config.metrics.enabled || !state.config.metrics.auto_poll {
                break;
            }

            match collect_default_metric_snapshot(&state).await {
                Ok(snapshot) => {
                    if let Err(apply_error) = apply_metrics_snapshot(&state, snapshot).await {
                        tracing::warn!(
                            "metrics auto-poll sample could not be applied: {apply_error}"
                        );
                    }
                }
                Err(poll_error) => tracing::warn!("metrics auto-poll failed: {poll_error}"),
            }

            sleep(Duration::from_millis(
                state.config.metrics.poll_interval_ms.max(500),
            ))
            .await;
        }
    });
}

async fn api_metrics_sources(State(state): State<ApiState>) -> impl IntoResponse {
    Json(serde_json::json!({
        "enabled": state.config.metrics.enabled,
        "config": state.config.metrics.clone(),
        "sources": metric_source_configs(&state).await,
        "scenario_labels": MetricsScenario::labels(),
        "accumulator": state.metrics_state.read().await.snapshot()
    }))
}

async fn api_metrics_latest(State(state): State<ApiState>) -> impl IntoResponse {
    Json(serde_json::json!({
        "latest": state.last_metrics_snapshot.read().await.clone(),
        "accumulator": state.metrics_state.read().await.snapshot(),
        "session": state.session.read().await.snapshot()
    }))
}

async fn api_metrics_scenarios() -> impl IntoResponse {
    Json(serde_json::json!({ "scenarios": MetricsScenario::labels() }))
}

async fn api_metrics_simulate(
    State(state): State<ApiState>,
    _auth: ControlAuth,
    Path(scenario): Path<String>,
) -> impl IntoResponse {
    if !state.config.metrics.allow_demo_samples {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "demo metrics samples are disabled in config" })),
        );
    }

    let Some(scenario) = MetricsScenario::parse(&scenario) else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": format!("unknown metrics scenario: {scenario}"),
                "valid_scenarios": MetricsScenario::labels()
            })),
        );
    };

    let snapshot = simulated_relay_snapshot(scenario, now_ms());
    metrics_response_tuple(apply_metrics_snapshot(&state, snapshot).await)
}

async fn api_metrics_ingest_prometheus(
    State(state): State<ApiState>,
    _auth: ControlAuth,
    Path(source): Path<String>,
    body: String,
) -> impl IntoResponse {
    if !state.config.metrics.enabled {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "metrics ingestion is disabled in config" })),
        );
    }

    let snapshot = {
        let mut metrics_state = state.metrics_state.write().await;
        metrics_state.update_from_prometheus_text(source, &body, now_ms())
    };
    match snapshot {
        Ok(snapshot) => metrics_response_tuple(apply_metrics_snapshot(&state, snapshot).await),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        ),
    }
}

async fn api_metrics_ingest_srtla_log(
    State(state): State<ApiState>,
    _auth: ControlAuth,
    Path(source): Path<String>,
    body: String,
) -> impl IntoResponse {
    if !state.config.metrics.enabled {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": "metrics ingestion is disabled in config" })),
        );
    }

    let snapshot = {
        let mut metrics_state = state.metrics_state.write().await;
        metrics_state.update_from_srtla_log_line(source, &body, now_ms())
    };
    metrics_response_tuple(apply_metrics_snapshot(&state, snapshot).await)
}

async fn api_metrics_poll_default(
    State(state): State<ApiState>,
    _auth: ControlAuth,
) -> impl IntoResponse {
    match collect_default_metric_snapshot(&state).await {
        Ok(snapshot) => metrics_response_tuple(apply_metrics_snapshot(&state, snapshot).await),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error })),
        ),
    }
}

async fn api_metrics_poll_named(
    State(state): State<ApiState>,
    _auth: ControlAuth,
    Path(source_name): Path<String>,
) -> impl IntoResponse {
    let Some(source) = find_metric_source(&state, &source_name).await else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": format!("unknown metrics source: {source_name}") })),
        );
    };
    match poll_metrics_source(&state, source).await {
        Ok(snapshot) => metrics_response_tuple(apply_metrics_snapshot(&state, snapshot).await),
        Err(error) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "error": error })),
        ),
    }
}

async fn api_metrics_poll_api_named(
    State(state): State<ApiState>,
    _auth: ControlAuth,
    Path(source_name): Path<String>,
) -> impl IntoResponse {
    let Some(source) = find_metric_source(&state, &source_name).await else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": format!("unknown metrics source: {source_name}") })),
        );
    };

    match MetricsPoller::new().poll_mediamtx_api(&source).await {
        Ok(snapshot) => metrics_response_tuple(apply_metrics_snapshot(&state, snapshot).await),
        Err(error) => (
            StatusCode::BAD_GATEWAY,
            Json(serde_json::json!({ "error": error.to_string(), "source": source.name })),
        ),
    }
}

async fn collect_default_metric_snapshot(state: &ApiState) -> Result<RelayMetricsSnapshot, String> {
    match state.config.metrics.source {
        MetricsSourceKind::Disabled => {
            if state.config.metrics.allow_demo_samples {
                Ok(simulated_relay_snapshot(MetricsScenario::Healthy, now_ms()))
            } else {
                Err("metrics source is disabled".to_string())
            }
        }
        MetricsSourceKind::Demo => {
            let ingested = state.metrics_state.read().await.snapshot().ingested_samples;
            let scenario = match ingested % 4 {
                0 => MetricsScenario::Healthy,
                1 => MetricsScenario::Degraded,
                2 => MetricsScenario::Brownout,
                _ => MetricsScenario::Offline,
            };
            Ok(simulated_relay_snapshot(scenario, now_ms()))
        }
        MetricsSourceKind::MediaMtxPrometheus => {
            poll_metrics_source(
                state,
                MetricsSourceConfig {
                    name: "mediamtx".to_string(),
                    metrics_url: Some(state.config.metrics.mediamtx_metrics_url.clone()),
                    api_url: Some(state.config.relay.mediamtx_api_url.clone()),
                    timeout_ms: state.config.metrics.request_timeout_ms,
                },
            )
            .await
        }
        MetricsSourceKind::SrtlaLog => Err(
            "automatic SRTLA log polling is not available; post a line to /api/metrics/ingest-srtla-log/{source}"
                .to_string(),
        ),
    }
}

async fn poll_metrics_source(
    state: &ApiState,
    source: MetricsSourceConfig,
) -> Result<RelayMetricsSnapshot, String> {
    let metrics_url = source
        .metrics_url
        .clone()
        .filter(|url| !url.trim().is_empty())
        .ok_or_else(|| format!("metrics source {} has no metrics_url", source.name))?;
    let body = poll_http_text(&metrics_url, source.timeout_ms)
        .await
        .map_err(|error| error.to_string())?;
    let mut metrics_state = state.metrics_state.write().await;
    metrics_state
        .update_from_prometheus_text(source.name, &body, now_ms())
        .map_err(|error| error.to_string())
}

#[derive(Debug, Clone, Serialize)]
struct MetricsEvaluationResponse {
    snapshot: RelayMetricsSnapshot,
    stream_metrics: StreamMetrics,
    decision: HealthDecision,
    scene_switched: bool,
}

async fn apply_metrics_snapshot(
    state: &ApiState,
    snapshot: RelayMetricsSnapshot,
) -> Result<MetricsEvaluationResponse, String> {
    let stream_metrics = snapshot.to_stream_metrics();
    let decision = {
        let mut engine = state.health_engine.write().await;
        engine
            .evaluate(&stream_metrics)
            .map_err(|error| error.to_string())?
    };

    let mut scene_switched = false;
    if state.config.metrics.auto_switch_scenes {
        state
            .obs
            .switch_scene(&state.scene_bundle, decision.recommended_scene)
            .await
            .map_err(|error| error.to_string())?;
        scene_switched = true;
    }

    {
        let mut session = state.session.write().await;
        session.push_sample(stream_metrics.clone(), decision.clone());
        if scene_switched {
            session.record_scene_switch(decision.recommended_scene);
        }
    }

    *state.last_metrics_snapshot.write().await = Some(snapshot.clone());

    Ok(MetricsEvaluationResponse {
        snapshot,
        stream_metrics,
        decision,
        scene_switched,
    })
}

fn metrics_response_tuple(
    result: Result<MetricsEvaluationResponse, String>,
) -> (StatusCode, Json<serde_json::Value>) {
    match result {
        Ok(response) => (StatusCode::OK, Json(serde_json::json!(response))),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": error })),
        ),
    }
}

async fn metric_source_configs(state: &ApiState) -> Vec<MetricsSourceConfig> {
    let mut sources = state
        .relay
        .plans()
        .await
        .into_iter()
        .filter_map(|plan| {
            plan.metrics_url.map(|metrics_url| MetricsSourceConfig {
                name: plan.name,
                metrics_url: Some(metrics_url),
                api_url: plan.api_url,
                timeout_ms: state.config.metrics.request_timeout_ms,
            })
        })
        .collect::<Vec<_>>();

    if sources.is_empty()
        && matches!(
            state.config.metrics.source,
            MetricsSourceKind::MediaMtxPrometheus
        )
    {
        sources.push(MetricsSourceConfig {
            name: "mediamtx".to_string(),
            metrics_url: Some(state.config.metrics.mediamtx_metrics_url.clone()),
            api_url: Some(state.config.relay.mediamtx_api_url.clone()),
            timeout_ms: state.config.metrics.request_timeout_ms,
        });
    }

    if sources.is_empty() {
        sources.push(MetricsSourceConfig::disabled());
    }

    sources
}

async fn find_metric_source(state: &ApiState, source_name: &str) -> Option<MetricsSourceConfig> {
    metric_source_configs(state)
        .await
        .into_iter()
        .find(|source| source.name == source_name)
}

async fn api_state(State(state): State<ApiState>) -> impl IntoResponse {
    let obs_status = obs_status_json(&state).await;
    let session = state.session.read().await.snapshot();
    Json(serde_json::json!({
        "app": "openirl-agent",
        "started_at": state.started_at.to_string(),
        "bind": state.config.api.bind,
        "allow_lan": state.config.api.allow_lan,
        "scene_bundle": state.scene_bundle,
        "obs": obs_status,
        "obs_adapter": state.config.obs.adapter,
        "relay": {
            "enabled": state.config.relay.enabled,
            "plans": state.relay.plans().await,
            "statuses": state.relay.statuses().await,
            "credentials": relay_credential_plan_from_config(&state.config)
        },
        "metrics": {
            "config": state.config.metrics.clone(),
            "sources": metric_source_configs(&state).await,
            "latest": state.last_metrics_snapshot.read().await.clone(),
            "accumulator": state.metrics_state.read().await.snapshot()
        },
        "config_validation": validate_config(&state.config),
        "production": {
            "markers": state.markers.read().await.markers(),
            "plan_endpoint": "/api/production/plan"
        },
        "session": session,
        "features": {
            "feature_area_count": INITIAL_FEATURE_AREA_COUNT,
            "source_readiness_area_count": 8,
            "schema_revision": OPENIRL_SCHEMA_REVISION
        },
        "alpha": {
            "validation_plan": "/api/alpha/validation-plan",
            "readiness": "/api/alpha/readiness"
        },
        "field": {
            "validation_plan": "/api/field/validation-plan",
            "device_checklists": "/api/field/device-checklists",
            "sample_evidence": "/api/field/sample-evidence",
            "readiness": "/api/field/readiness",
            "evidence": "/api/field/evidence"
        },
        "artifacts": artifact_plan_json(&state.config)
    }))
}

async fn api_evaluate(
    State(state): State<ApiState>,
    _auth: ControlAuth,
    Json(metrics): Json<StreamMetrics>,
) -> impl IntoResponse {
    match evaluate_and_switch(&state, metrics).await {
        Ok(decision) => (StatusCode::OK, Json(serde_json::json!(decision))),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error })),
        ),
    }
}

async fn evaluate_and_switch(
    state: &ApiState,
    metrics: StreamMetrics,
) -> Result<HealthDecision, String> {
    let decision = {
        let mut engine = state.health_engine.write().await;
        engine
            .evaluate(&metrics)
            .map_err(|error| error.to_string())?
    };

    state
        .obs
        .switch_scene(&state.scene_bundle, decision.recommended_scene)
        .await
        .map_err(|error| error.to_string())?;

    {
        let mut session = state.session.write().await;
        session.push_sample(metrics, decision.clone());
        session.record_scene_switch(decision.recommended_scene);
    }

    Ok(decision)
}

async fn api_profile(_auth: ControlAuth, Json(request): Json<ProfileRequest>) -> impl IntoResponse {
    match generate_profile(&request) {
        Ok(profile) => (StatusCode::OK, Json(serde_json::json!(profile))),
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        ),
    }
}

#[derive(Debug, Clone, Serialize)]
struct ProfileQrResponse {
    profile: GeneratedProfile,
    qr: openirl_qr::QrRender,
}

async fn api_profile_qr(
    _auth: ControlAuth,
    Json(request): Json<ProfileRequest>,
) -> impl IntoResponse {
    match generate_profile(&request) {
        Ok(profile) => match render_qr_svg(&QrRenderRequest::new(
            profile.contribution_url.clone(),
            format!("{} {} profile", profile.encoder, profile.protocol),
        )) {
            Ok(qr) => (
                StatusCode::OK,
                Json(serde_json::json!(ProfileQrResponse { profile, qr })),
            ),
            Err(error) => (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": error.to_string() })),
            ),
        },
        Err(error) => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": error.to_string() })),
        ),
    }
}

#[derive(Debug, Clone, Serialize)]
struct ProfilePresetResponse {
    encoder: EncoderKind,
    preferred_protocol: Protocol,
    profiles: Vec<GeneratedProfile>,
}

async fn api_profile_defaults(State(state): State<ApiState>) -> Json<Vec<ProfilePresetResponse>> {
    let mut response = Vec::new();
    for support in support_matrix() {
        let encoder = support.encoder;
        let preferred_protocol = support.preferred_protocol;
        let mut profiles = Vec::new();
        for protocol in support.protocols {
            let request = profile_request_from_config(&state.config, encoder, protocol);
            if let Ok(profile) = generate_profile(&request) {
                profiles.push(profile);
            }
        }
        response.push(ProfilePresetResponse {
            encoder,
            preferred_protocol,
            profiles,
        });
    }
    Json(response)
}

async fn api_obs_status(State(state): State<ApiState>) -> impl IntoResponse {
    Json(obs_status_json(&state).await)
}

async fn api_obs_actions(State(state): State<ApiState>) -> impl IntoResponse {
    match state.obs.action_log().await {
        Ok(actions) => (
            StatusCode::OK,
            Json(serde_json::json!({ "actions": actions })),
        ),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": error.to_string() })),
        ),
    }
}

async fn api_obs_template(State(state): State<ApiState>) -> impl IntoResponse {
    Json(build_obs_scene_template_plan(
        &state.scene_bundle,
        state.config.artifacts.fallback_assets_dir.clone(),
        live_input_url_from_config(&state.config),
    ))
}

async fn api_obs_template_materialize(
    State(state): State<ApiState>,
    _auth: ControlAuth,
) -> impl IntoResponse {
    let plan = build_obs_scene_template_plan(
        &state.scene_bundle,
        state.config.artifacts.fallback_assets_dir.clone(),
        live_input_url_from_config(&state.config),
    );
    match materialize_obs_scene_template(obs_template_output_path(&state.config), &plan) {
        Ok(file) => (
            StatusCode::OK,
            Json(serde_json::json!({ "file": file, "plan": plan })),
        ),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": error.to_string() })),
        ),
    }
}

async fn api_obs_template_apply(
    State(state): State<ApiState>,
    _auth: ControlAuth,
) -> impl IntoResponse {
    let request = scene_template_request_from_config(&state.config);
    let plan = build_scene_materialization_plan(&state.scene_bundle, &request);
    match state.obs.ensure_scene_materialization(&plan).await {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({ "applied": true, "plan": plan })),
        ),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": error.to_string(), "plan": plan })),
        ),
    }
}

async fn api_obs_switch(
    State(state): State<ApiState>,
    _auth: ControlAuth,
    Path(role): Path<String>,
) -> impl IntoResponse {
    let role = match role.parse::<SceneRole>() {
        Ok(role) => role,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": error.to_string() })),
            );
        }
    };

    match state.obs.switch_scene(&state.scene_bundle, role).await {
        Ok(()) => {
            state.session.write().await.record_scene_switch(role);
            (
                StatusCode::OK,
                Json(serde_json::json!({ "switched": role })),
            )
        }
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": error.to_string() })),
        ),
    }
}

async fn api_obs_start(State(state): State<ApiState>, _auth: ControlAuth) -> impl IntoResponse {
    match state.obs.start_streaming().await {
        Ok(()) => {
            state.session.write().await.record_control(
                SessionEventKind::StartStreaming,
                "operator requested stream start",
            );
            (
                StatusCode::OK,
                Json(serde_json::json!({ "streaming": true })),
            )
        }
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": error.to_string() })),
        ),
    }
}

async fn api_obs_stop(State(state): State<ApiState>, _auth: ControlAuth) -> impl IntoResponse {
    match state.obs.stop_streaming().await {
        Ok(()) => {
            state.session.write().await.record_control(
                SessionEventKind::StopStreaming,
                "operator requested stream stop",
            );
            (
                StatusCode::OK,
                Json(serde_json::json!({ "streaming": false })),
            )
        }
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": error.to_string() })),
        ),
    }
}

async fn api_production_plan(State(state): State<ApiState>) -> impl IntoResponse {
    Json(default_production_plan(&state.scene_bundle))
}

async fn api_production_markers(State(state): State<ApiState>) -> impl IntoResponse {
    Json(serde_json::json!({ "markers": state.markers.read().await.markers() }))
}

async fn api_production_marker(
    State(state): State<ApiState>,
    _auth: ControlAuth,
    Json(request): Json<ClipMarkerRequest>,
) -> impl IntoResponse {
    let health_state = state.session.read().await.snapshot().current_decision.state;
    let marker = state
        .markers
        .write()
        .await
        .add_marker(request, health_state);
    state.session.write().await.record_control(
        SessionEventKind::OperatorControl,
        format!("clip marker added: {}", marker.title),
    );
    (StatusCode::OK, Json(serde_json::json!(marker)))
}

async fn api_production_markers_clear(
    State(state): State<ApiState>,
    _auth: ControlAuth,
) -> impl IntoResponse {
    state.markers.write().await.clear();
    state
        .session
        .write()
        .await
        .record_control(SessionEventKind::OperatorControl, "clip markers cleared");
    (StatusCode::OK, Json(serde_json::json!({ "cleared": true })))
}

async fn api_production_save_replay(
    State(state): State<ApiState>,
    _auth: ControlAuth,
) -> impl IntoResponse {
    match state.obs.save_replay_buffer().await {
        Ok(()) => {
            state.session.write().await.record_control(
                SessionEventKind::OperatorControl,
                "replay buffer save requested",
            );
            (StatusCode::OK, Json(serde_json::json!({ "saved": true })))
        }
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": error.to_string() })),
        ),
    }
}

async fn api_production_start_recording(
    State(state): State<ApiState>,
    _auth: ControlAuth,
) -> impl IntoResponse {
    match state.obs.start_recording().await {
        Ok(()) => {
            state.session.write().await.record_control(
                SessionEventKind::OperatorControl,
                "recording start requested",
            );
            (
                StatusCode::OK,
                Json(serde_json::json!({ "recording": true })),
            )
        }
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": error.to_string() })),
        ),
    }
}

async fn api_production_stop_recording(
    State(state): State<ApiState>,
    _auth: ControlAuth,
) -> impl IntoResponse {
    match state.obs.stop_recording().await {
        Ok(()) => {
            state.session.write().await.record_control(
                SessionEventKind::OperatorControl,
                "recording stop requested",
            );
            (
                StatusCode::OK,
                Json(serde_json::json!({ "recording": false })),
            )
        }
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": error.to_string() })),
        ),
    }
}

async fn api_moderation_command(
    State(state): State<ApiState>,
    auth: ControlAuth,
    Json(mut request): Json<ModeratorCommandRequest>,
) -> impl IntoResponse {
    request.role = auth.role;
    let decision = evaluate_moderator_command(&request);
    if !decision.allowed {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "decision": decision })),
        );
    }

    let mut result = serde_json::json!({ "decision": decision, "executed": false });
    match request.action.trim().to_ascii_lowercase().as_str() {
        "status" => {
            result["executed"] = serde_json::json!(true);
            result["status"] = obs_status_json(&state).await;
        }
        "privacy" => {
            if let Err(error) = state
                .obs
                .switch_scene(&state.scene_bundle, SceneRole::Privacy)
                .await
            {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": error.to_string() })),
                );
            }
            state
                .session
                .write()
                .await
                .record_scene_switch(SceneRole::Privacy);
            result["executed"] = serde_json::json!(true);
        }
        "switch-scene" => {
            let Some(argument) = request.argument.as_deref() else {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({ "error": "switch-scene requires argument" })),
                );
            };
            let role = match argument.parse::<SceneRole>() {
                Ok(role) => role,
                Err(error) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(serde_json::json!({ "error": error.to_string() })),
                    );
                }
            };
            if let Err(error) = state.obs.switch_scene(&state.scene_bundle, role).await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": error.to_string() })),
                );
            }
            state.session.write().await.record_scene_switch(role);
            result["executed"] = serde_json::json!(true);
        }
        "save-replay" => {
            if let Err(error) = state.obs.save_replay_buffer().await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": error.to_string() })),
                );
            }
            result["executed"] = serde_json::json!(true);
        }
        "add-marker" => {
            let title = request
                .argument
                .clone()
                .unwrap_or_else(|| "Moderator marker".to_string());
            let health_state = state.session.read().await.snapshot().current_decision.state;
            let marker = state.markers.write().await.add_marker(
                ClipMarkerRequest {
                    title,
                    note: Some("created from moderator command".to_string()),
                    tags: vec!["mod".to_string()],
                },
                health_state,
            );
            result["marker"] = serde_json::json!(marker);
            result["executed"] = serde_json::json!(true);
        }
        "start-recording" => {
            if let Err(error) = state.obs.start_recording().await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": error.to_string() })),
                );
            }
            result["executed"] = serde_json::json!(true);
        }
        "stop-recording" => {
            if let Err(error) = state.obs.stop_recording().await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": error.to_string() })),
                );
            }
            result["executed"] = serde_json::json!(true);
        }
        "start-stream" => {
            if let Err(error) = state.obs.start_streaming().await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": error.to_string() })),
                );
            }
            result["executed"] = serde_json::json!(true);
        }
        "stop-stream" => {
            if let Err(error) = state.obs.stop_streaming().await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "error": error.to_string() })),
                );
            }
            result["executed"] = serde_json::json!(true);
        }
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "unknown moderation action" })),
            );
        }
    }
    (StatusCode::OK, Json(result))
}

async fn api_release_manifest(State(state): State<ApiState>) -> impl IntoResponse {
    Json(build_release_manifest(
        env!("CARGO_PKG_VERSION"),
        OPENIRL_SCHEMA_REVISION,
        &validate_config(&state.config),
    ))
}

async fn api_release_gates(State(state): State<ApiState>) -> impl IntoResponse {
    let manifest = build_release_manifest(
        env!("CARGO_PKG_VERSION"),
        OPENIRL_SCHEMA_REVISION,
        &validate_config(&state.config),
    );
    Json(serde_json::json!({ "gates": manifest.gates }))
}

async fn api_release_smoke_plan(State(state): State<ApiState>) -> impl IntoResponse {
    let manifest = build_release_manifest(
        env!("CARGO_PKG_VERSION"),
        OPENIRL_SCHEMA_REVISION,
        &validate_config(&state.config),
    );
    Json(
        serde_json::json!({ "smoke_tests": manifest.smoke_tests, "compatibility": manifest.compatibility }),
    )
}

async fn api_alpha_validation_plan() -> impl IntoResponse {
    Json(build_alpha_validation_plan(OPENIRL_SCHEMA_REVISION))
}

async fn api_alpha_operator_checklist() -> impl IntoResponse {
    Json(serde_json::json!({ "steps": build_operator_checklist() }))
}

async fn api_alpha_readiness(State(state): State<ApiState>) -> impl IntoResponse {
    let evidence = alpha_evidence_from_runtime(&state).await;
    let report = evaluate_alpha_evidence(&evidence);
    Json(serde_json::json!({
        "evidence": evidence,
        "report": report,
        "plan": build_alpha_validation_plan(OPENIRL_SCHEMA_REVISION)
    }))
}

async fn api_alpha_evidence(
    _auth: ControlAuth,
    Json(evidence): Json<AlphaEvidenceInput>,
) -> impl IntoResponse {
    Json(evaluate_alpha_evidence(&evidence))
}

async fn api_field_validation_plan() -> impl IntoResponse {
    Json(build_field_validation_plan(OPENIRL_SCHEMA_REVISION))
}

async fn api_field_device_checklists() -> impl IntoResponse {
    Json(serde_json::json!({ "devices": build_device_checklists() }))
}

async fn api_field_sample_evidence() -> impl IntoResponse {
    Json(sample_field_evidence())
}

async fn api_field_readiness(State(state): State<ApiState>) -> impl IntoResponse {
    let evidence = field_evidence_from_runtime(&state).await;
    Json(serde_json::json!({
        "evidence": evidence,
        "report": evaluate_field_evidence(&evidence),
        "plan": build_field_validation_plan(OPENIRL_SCHEMA_REVISION)
    }))
}

async fn api_field_evidence(
    _auth: ControlAuth,
    Json(evidence): Json<FieldEvidenceInput>,
) -> impl IntoResponse {
    Json(evaluate_field_evidence(&evidence))
}

async fn field_evidence_from_runtime(state: &ApiState) -> FieldEvidenceInput {
    let validation = validate_config(&state.config);
    let obs_status = state.obs.status().await.ok();
    let latest_metrics = state.last_metrics_snapshot.read().await.clone();
    let session = state.session.read().await.snapshot();
    let healthy_state_seen = session
        .recent_samples
        .iter()
        .any(|sample| sample.decision.state == HealthState::Healthy);
    let brownout_state_seen = session
        .recent_samples
        .iter()
        .any(|sample| sample.decision.state == HealthState::Brownout);
    let recovery_state_seen = session
        .recent_samples
        .iter()
        .any(|sample| sample.decision.state == HealthState::RecoveryPending);
    let brb_scene_seen = session.last_scene == SceneRole::Brb
        || session
            .recent_samples
            .iter()
            .any(|sample| sample.decision.recommended_scene == SceneRole::Brb);
    let obs_connected = obs_status
        .as_ref()
        .map(|status| status.connected)
        .unwrap_or(false);

    FieldEvidenceInput {
        static_validation_passed: false,
        rust_ci_passed: false,
        windows_alpha_ready: obs_connected && validation.ok,
        moblin_profile_generated: false,
        moblin_qr_scanned: false,
        moblin_ingest_seen: false,
        irlpro_profile_generated: false,
        irlpro_qr_scanned: false,
        irlpro_ingest_seen: false,
        belabox_profile_generated: false,
        belabox_config_reviewed: false,
        belabox_ingest_seen: false,
        mediamtx_srt_path_active: latest_metrics
            .as_ref()
            .map(|snapshot| snapshot.stream_metrics.connected_links > 0)
            .unwrap_or(false),
        mediamtx_metrics_seen: latest_metrics.is_some(),
        obs_connected,
        obs_media_source_seen: obs_connected && latest_metrics.is_some(),
        healthy_state_seen,
        brownout_state_seen,
        brb_scene_seen,
        recovery_state_seen,
        support_bundle_captured: false,
        secrets_redacted: state.config.security.support_bundle_redact_ips,
        field_report_written: false,
    }
}

async fn alpha_evidence_from_runtime(state: &ApiState) -> AlphaEvidenceInput {
    let obs_status = state.obs.status().await.ok();
    let validation = validate_config(&state.config);
    let latest_metrics = state.last_metrics_snapshot.read().await.clone();
    AlphaEvidenceInput {
        static_validation_passed: false,
        rust_ci_passed: false,
        config_ok: validation.ok,
        agent_health_ok: true,
        dashboard_loaded: false,
        obs_connected: obs_status
            .as_ref()
            .map(|status| status.connected)
            .unwrap_or(false),
        obs_scene_switch_verified: false,
        obs_streaming_start_stop_verified: false,
        replay_save_verified: false,
        recording_controls_verified: false,
        profile_qr_tested: false,
        metrics_poll_verified: latest_metrics.is_some(),
        windows_portable_built: false,
        windows_msi_reviewed: false,
    }
}

#[derive(Debug, Clone, Deserialize)]
struct SupportBundleExportApiRequest {
    output_dir: Option<String>,
    field_report_markdown: Option<String>,
}

async fn api_artifacts_plan(State(state): State<ApiState>) -> impl IntoResponse {
    Json(artifact_plan_json(&state.config))
}

async fn api_materialize_fallback_assets(
    State(state): State<ApiState>,
    _auth: ControlAuth,
) -> impl IntoResponse {
    let plan = default_fallback_asset_plan(
        &state.scene_bundle,
        state.config.artifacts.fallback_assets_dir.clone(),
    );
    match materialize_fallback_assets(&plan) {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": error.to_string() })),
        ),
    }
}

async fn api_alpha_package_layout(State(state): State<ApiState>) -> impl IntoResponse {
    Json(alpha_source_package_layout(
        state.config.artifacts.alpha_package_dir.clone(),
    ))
}

async fn api_alpha_package_layout_materialize(
    State(state): State<ApiState>,
    _auth: ControlAuth,
) -> impl IntoResponse {
    let layout = alpha_source_package_layout(state.config.artifacts.alpha_package_dir.clone());
    match materialize_alpha_source_layout(&layout) {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": error.to_string() })),
        ),
    }
}

async fn api_support_bundle_export(
    State(state): State<ApiState>,
    _auth: ControlAuth,
    Json(request): Json<SupportBundleExportApiRequest>,
) -> impl IntoResponse {
    let payload = support_bundle_payload(&state).await;
    let report = state.session.read().await.report();
    let output_dir = match support_bundle_api_output_dir(
        &state.config.artifacts.support_bundles_dir,
        request.output_dir,
    ) {
        Ok(path) => path,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": error })),
            );
        }
    };
    let export_request = SupportBundleExportRequest {
        output_dir,
        field_report_markdown: request.field_report_markdown.map(|report| {
            redact_support_text(&report, state.config.security.support_bundle_redact_ips)
        }),
    };

    match export_support_bundle(&export_request, &payload, Some(report)) {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": error.to_string() })),
        ),
    }
}

fn support_bundle_api_output_dir(
    configured_root: &str,
    requested: Option<String>,
) -> Result<String, String> {
    let Some(raw) = requested else {
        return Ok(configured_root.to_string());
    };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("support bundle output_dir is empty".to_string());
    }

    let requested_path = PathBuf::from(trimmed);
    if requested_path.is_absolute() {
        return Err(
            "support bundle API output_dir must be relative to the configured bundle root"
                .to_string(),
        );
    }

    let mut relative = PathBuf::new();
    for component in requested_path.components() {
        match component {
            Component::Normal(part) => relative.push(part),
            Component::CurDir => {}
            _ => {
                return Err(
                    "support bundle API output_dir cannot contain parent or prefix components"
                        .to_string(),
                );
            }
        }
    }

    if relative.as_os_str().is_empty() {
        return Err("support bundle output_dir is empty".to_string());
    }

    Ok(PathBuf::from(configured_root)
        .join(relative)
        .to_string_lossy()
        .into_owned())
}

async fn api_field_report_export(
    State(state): State<ApiState>,
    _auth: ControlAuth,
    body: String,
) -> impl IntoResponse {
    let fallback_report = state.session.read().await.report().summary;
    let field_report_markdown = if body.trim().is_empty() {
        format!(
            "# OpenIRL Field Report\n\nImplementation feature: {OPENIRL_SCHEMA_REVISION}\n\n{fallback_report}\n"
        )
    } else {
        body
    };
    let field_report_markdown = redact_support_text(
        &field_report_markdown,
        state.config.security.support_bundle_redact_ips,
    );
    let field_report_result = match export_field_report_markdown(
        &state.config.artifacts.field_reports_dir,
        &field_report_markdown,
    ) {
        Ok(result) => result,
        Err(error) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": error.to_string() })),
            );
        }
    };
    let payload = support_bundle_payload(&state).await;
    let request = SupportBundleExportRequest {
        output_dir: state.config.artifacts.support_bundles_dir.clone(),
        field_report_markdown: Some(field_report_markdown),
    };
    let report = state.session.read().await.report();
    match export_support_bundle(&request, &payload, Some(report)) {
        Ok(result) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "field_report": field_report_result,
                "support_bundle": result
            })),
        ),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": error.to_string() })),
        ),
    }
}

async fn api_v1_features() -> Json<Vec<openirl_v1::V1Feature>> {
    Json(build_v1_features())
}

async fn api_v1_summary() -> impl IntoResponse {
    Json(build_v1_implementation_summary())
}

async fn api_v1_sample_evidence() -> Json<V1EvidenceInput> {
    Json(sample_v1_evidence())
}

async fn api_v1_readiness() -> impl IntoResponse {
    Json(evaluate_v1_evidence(&sample_v1_evidence()))
}

async fn api_v1_readiness_with_evidence(
    _auth: ControlAuth,
    Json(payload): Json<V1EvidenceInput>,
) -> impl IntoResponse {
    Json(evaluate_v1_evidence(&payload))
}

async fn api_v1_package_layout() -> impl IntoResponse {
    Json(default_v1_package_layout("artifacts/v1-public-beta"))
}

async fn api_v1_package_layout_materialize(_auth: ControlAuth) -> impl IntoResponse {
    let layout = default_v1_package_layout("artifacts/v1-public-beta");
    match materialize_v1_package(&layout, false) {
        Ok(result) => (StatusCode::OK, Json(serde_json::json!(result))).into_response(),
        Err(error) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

async fn api_session(State(state): State<ApiState>) -> impl IntoResponse {
    Json(serde_json::json!(state.session.read().await.snapshot()))
}

async fn api_session_report(State(state): State<ApiState>) -> impl IntoResponse {
    let report = state.session.read().await.report();
    Json(serde_json::json!(report))
}

async fn api_support_bundle(
    State(state): State<ApiState>,
    _auth: ControlAuth,
) -> impl IntoResponse {
    Json(support_bundle_payload(&state).await)
}

async fn support_bundle_payload(state: &ApiState) -> serde_json::Value {
    let report = state.session.read().await.report();
    let manifest =
        SupportBundleManifest::from_report(report, state.config.security.support_bundle_redact_ips);
    let obs_actions: Vec<String> = state.obs.action_log().await.unwrap_or_default();
    let token_value = std::env::var(&state.config.security.dashboard_token_env).ok();
    let payload = serde_json::json!({
        "manifest": manifest,
        "config": state.config.redacted(),
        "config_validation": validate_config(&state.config),
        "obs_actions": obs_actions,
        "relay": {
            "plans": state.relay.plans().await,
            "statuses": state.relay.statuses().await,
            "credentials": relay_credential_plan_from_config(&state.config)
        },
        "metrics": {
            "latest": state.last_metrics_snapshot.read().await.clone(),
            "accumulator": state.metrics_state.read().await.snapshot(),
            "sources": metric_source_configs(state).await
        },
        "production": {
            "plan": default_production_plan(&state.scene_bundle),
            "markers": state.markers.read().await.markers()
        },
        "artifacts": artifact_plan_json(&state.config),
        "alpha_validation": {
            "plan": build_alpha_validation_plan(OPENIRL_SCHEMA_REVISION),
            "operator_checklist": build_operator_checklist(),
            "readiness": evaluate_alpha_evidence(&alpha_evidence_from_runtime(state).await)
        },
        "field_validation": {
            "plan": build_field_validation_plan(OPENIRL_SCHEMA_REVISION),
            "device_checklists": build_device_checklists(),
            "readiness": evaluate_field_evidence(&field_evidence_from_runtime(state).await)
        },
        "release": build_release_manifest(env!("CARGO_PKG_VERSION"), OPENIRL_SCHEMA_REVISION, &validate_config(&state.config)),
        "auth": auth_status(&auth_policy_from_config(&state.config), token_value.as_deref()),
        "profile_support_matrix": support_matrix()
    });
    scrub_support_bundle_value(payload, state.config.security.support_bundle_redact_ips)
}

async fn api_session_reset(State(state): State<ApiState>, _auth: ControlAuth) -> impl IntoResponse {
    state.session.write().await.clear_samples();
    (StatusCode::OK, Json(serde_json::json!({ "reset": true })))
}

async fn obs_status_json(state: &ApiState) -> serde_json::Value {
    match state.obs.status().await {
        Ok(status) => serde_json::json!(status),
        Err(error) => serde_json::json!({ "error": error.to_string() }),
    }
}

fn relay_result_json(
    result: Result<RelayRuntimeStatus, openirl_relay_control::RelayControlError>,
) -> serde_json::Value {
    match result {
        Ok(status) => serde_json::json!(status),
        Err(error) => serde_json::json!({ "error": error.to_string() }),
    }
}

fn profile_request_from_config(
    config: &AppConfig,
    encoder: EncoderKind,
    protocol: Protocol,
) -> ProfileRequest {
    ProfileRequest {
        encoder,
        protocol,
        host: config.ingest.public_host.clone(),
        port: port_for_protocol(config, protocol),
        stream_id: default_stream_id(encoder),
        passphrase: Some("replace-me".to_string()),
        latency_ms: config.ingest.default_latency_ms,
        bitrate_kbps: 4500,
    }
}

fn default_stream_id(encoder: EncoderKind) -> String {
    match encoder {
        EncoderKind::Moblin => "moblin-main".to_string(),
        EncoderKind::IrlPro => "irlpro-main".to_string(),
        EncoderKind::Larix => "larix-main".to_string(),
        EncoderKind::Belabox => "belabox-main".to_string(),
        EncoderKind::Obs => "obs-main".to_string(),
        EncoderKind::LiveuLike => "liveu-main".to_string(),
        EncoderKind::Custom => "custom-main".to_string(),
    }
}

fn port_for_protocol(config: &AppConfig, protocol: Protocol) -> u16 {
    match protocol {
        Protocol::Srt => config.ingest.srt_port,
        Protocol::Srtla | Protocol::Srtla2 => config.ingest.srtla_port,
        Protocol::Rtmp | Protocol::Rtmps => config.ingest.rtmp_port,
        Protocol::Rist => config.ingest.srt_port,
        Protocol::Whip | Protocol::Whep | Protocol::EnhancedRtmp => 443,
    }
}

fn scene_template_request_from_config(config: &AppConfig) -> SceneTemplateRequest {
    SceneTemplateRequest {
        asset_root_dir: config.artifacts.fallback_assets_dir.clone(),
        local_srt_host: config.ingest.public_host.clone(),
        srt_port: config.ingest.srt_port,
        backup_srt_port: config.ingest.srt_port.saturating_add(10),
        rtmp_port: config.ingest.rtmp_port,
        latency_ms: config.ingest.default_latency_ms,
        overwrite_existing_assets: config.artifacts.overwrite_existing,
    }
}

fn live_input_url_from_config(config: &AppConfig) -> String {
    format!(
        "srt://{}:{}?mode=listener&latency={}",
        config.ingest.public_host, config.ingest.srt_port, config.ingest.default_latency_ms
    )
}

fn obs_template_output_path(config: &AppConfig) -> PathBuf {
    PathBuf::from(&config.artifacts.obs_templates_dir).join("obs-scene-template.json")
}

fn artifact_plan_json(config: &AppConfig) -> serde_json::Value {
    let bundle = config.scene_bundle();
    serde_json::json!({
        "schema_revision": OPENIRL_SCHEMA_REVISION,
        "fallback_assets": default_fallback_asset_plan(&bundle, config.artifacts.fallback_assets_dir.clone()),
        "obs_template": build_obs_scene_template_plan(
            &bundle,
            config.artifacts.fallback_assets_dir.clone(),
            live_input_url_from_config(config),
        ),
        "obs_scene_materialization": build_scene_materialization_plan(&bundle, &scene_template_request_from_config(config)),
        "support_bundle_dir": config.artifacts.support_bundles_dir.clone(),
        "field_reports_dir": config.artifacts.field_reports_dir.clone(),
        "alpha_source_layout": alpha_source_package_layout(config.artifacts.alpha_package_dir.clone()),
    })
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn support_bundle_api_output_dir_stays_under_configured_root() {
        let resolved =
            support_bundle_api_output_dir("artifacts/support-bundles", Some("issue-42".into()));
        assert!(resolved.is_ok());
        assert_eq!(
            resolved.ok().as_deref(),
            Some("artifacts/support-bundles/issue-42")
        );

        assert!(support_bundle_api_output_dir("artifacts/support-bundles", None).is_ok());
        assert!(
            support_bundle_api_output_dir("artifacts/support-bundles", Some("../escape".into()))
                .is_err()
        );
        assert!(
            support_bundle_api_output_dir("artifacts/support-bundles", Some("/tmp/export".into()))
                .is_err()
        );
    }
}
