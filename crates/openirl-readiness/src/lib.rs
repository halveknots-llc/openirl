//! Scoped readiness reports and deterministic first-run demo evidence.

use openirl_config::{AppConfig, MetricsSourceKind, ObsAdapterKind, RelaySupervisorMode};
use openirl_core::{EncoderKind, HealthDecision, Protocol, StreamMetrics};
use openirl_health::{HealthEngine, HealthError};
use openirl_metrics::{MetricsScenario, RelayMetricsSnapshot, simulated_relay_snapshot};
use openirl_profiles::{ProfileError, ProfileRequest, generate_profile};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use thiserror::Error;

/// Version of the readiness report contract.
pub const READINESS_REPORT_REVISION: u16 = 1;
/// Version of the deterministic demo scenario.
pub const DEMO_SCENARIO_REVISION: u16 = 1;
const DEMO_START_TIMESTAMP_MS: u64 = 1_700_000_000_000;
const DEMO_STEP_INTERVAL_MS: u64 = 10_000;

/// Readiness report construction errors.
#[derive(Debug, Error)]
pub enum ReadinessError {
    /// Demo health evaluation failed.
    #[error(transparent)]
    Health(#[from] HealthError),
    /// Synthetic profile generation failed.
    #[error(transparent)]
    Profile(#[from] ProfileError),
}

/// Runtime mode represented by a readiness report.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RuntimeMode {
    /// Normal operator configuration.
    Standard,
    /// Deterministic, local-only first-run demonstration.
    Demo,
}

/// Network exposure represented without disclosing a configured address or port.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ListenerExposure {
    /// Bound to a loopback interface with LAN access disabled.
    LoopbackOnly,
    /// LAN access was explicitly enabled.
    LanOptIn,
    /// A non-loopback bind was configured without the LAN opt-in flag.
    NonLoopback,
}

/// Share-safe readiness configuration containing roles and state, never endpoint identities.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReadinessConfigSummary {
    /// API listener exposure class.
    pub api_listener: ListenerExposure,
    /// Whether dashboard authentication is enabled.
    pub dashboard_auth_enabled: bool,
    /// Whether broader-than-loopback access requires authentication.
    pub auth_required_outside_loopback: bool,
    /// Number of explicitly allowed browser origins, without origin values.
    pub allowed_origin_count: usize,
    /// OBS integration mode, without host or port.
    pub obs_adapter: ObsAdapterKind,
    /// Whether relay behavior is enabled.
    pub relay_enabled: bool,
    /// Relay supervision mode, without commands, paths, or endpoints.
    pub relay_supervisor_mode: RelaySupervisorMode,
    /// Number of enabled relay processes.
    pub enabled_relay_process_count: usize,
    /// Whether metrics ingestion is enabled.
    pub metrics_enabled: bool,
    /// Metrics source kind, without service URLs.
    pub metrics_source: MetricsSourceKind,
    /// Whether every configured artifact destination is nonempty.
    pub artifact_destinations_configured: bool,
}

/// Evidence scope. Results from one scope never imply another scope passed.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceScope {
    /// Repository and workspace checks.
    Source,
    /// Local process and API behavior.
    LocalRuntime,
    /// Real third-party tools, devices, networks, or target hosts.
    LiveEnvironment,
}

/// Highest evidence maturity established for a feature or compatibility row.
#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceMaturity {
    /// Interfaces or workflows are modeled but not yet exercised as source behavior.
    Modeled,
    /// Source contracts and automated tests passed.
    SourceValidated,
    /// A local process or deterministic runtime path passed without a real dependency.
    LocalRuntimeValidated,
    /// The named real dependency passed in a controlled integration environment.
    IntegrationValidated,
    /// A real operator/device/network field session passed.
    FieldValidated,
    /// A downloadable release carries matching verification evidence.
    Released,
}

/// State of one readiness check.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReadinessStatus {
    /// Evidence passed the named check.
    Passed,
    /// Synthetic evidence demonstrated behavior without external dependencies.
    Demonstrated,
    /// No evidence was supplied or inferred.
    NotRun,
    /// A local prerequisite failed.
    Blocked,
}

impl ReadinessStatus {
    const fn satisfied(self) -> bool {
        matches!(self, Self::Passed | Self::Demonstrated)
    }
}

/// One scoped readiness check.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReadinessCheck {
    /// Stable machine-readable identifier.
    pub id: String,
    /// Evidence scope.
    pub scope: EvidenceScope,
    /// Current status.
    pub status: ReadinessStatus,
    /// Human-readable check label.
    pub label: String,
    /// Reproducible command or evidence source.
    pub evidence: String,
    /// Boundary or remediation note.
    pub note: String,
}

/// Counts for one evidence scope.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub struct ScopeSummary {
    /// Total checks in this scope.
    pub total: usize,
    /// Passed or safely demonstrated checks.
    pub satisfied: usize,
    /// Checks that have not run.
    pub not_run: usize,
    /// Checks blocked by a failed prerequisite.
    pub blocked: usize,
}

/// Readiness counts split by proof boundary.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReadinessSummary {
    /// Source checks.
    pub source: ScopeSummary,
    /// Local runtime checks.
    pub local_runtime: ScopeSummary,
    /// Live environment checks.
    pub live_environment: ScopeSummary,
}

/// One deterministic metric and health step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DemoStep {
    /// Sequence number.
    pub sequence: usize,
    /// Named synthetic scenario.
    pub scenario: MetricsScenario,
    /// Fixed sample timestamp.
    pub timestamp_ms: u64,
    /// Health-engine input.
    pub metrics: StreamMetrics,
    /// Health-engine decision.
    pub decision: HealthDecision,
}

/// Public-safe synthetic profile preview.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct DemoProfilePreview {
    /// Encoder represented by the sample.
    pub encoder: EncoderKind,
    /// Contribution protocol represented by the sample.
    pub protocol: Protocol,
    /// Redacted localhost contribution URL.
    pub display_url: String,
    /// Confirms the profile contains synthetic data only.
    pub synthetic: bool,
}

/// Deterministic evidence bundled into demo-mode reports.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DemoEvidence {
    /// Demo scenario contract revision.
    pub scenario_revision: u16,
    /// True when repeated construction yields the same payload.
    pub deterministic: bool,
    /// True when an outbound network request was made to build the evidence.
    pub outbound_network_requests_made: bool,
    /// True when an external media process was started.
    pub external_processes_started: bool,
    /// True when a real credential is required.
    pub credentials_required: bool,
    /// Synthetic metric and health sequence.
    pub steps: Vec<DemoStep>,
    /// Redacted synthetic encoder profile.
    pub profile: DemoProfilePreview,
}

/// Readiness report that keeps source, runtime, and live proof separate.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReadinessReport {
    /// OpenIRL API schema revision.
    pub schema_revision: u16,
    /// Readiness report contract revision.
    pub report_revision: u16,
    /// Runtime mode.
    pub mode: RuntimeMode,
    /// Share-safe configuration summary without private topology or paths.
    pub config: ReadinessConfigSummary,
    /// Scoped checks.
    pub checks: Vec<ReadinessCheck>,
    /// Counts by proof boundary.
    pub summary: ReadinessSummary,
    /// Demo evidence, present only in demo mode.
    pub demo: Option<DemoEvidence>,
    /// Non-negotiable interpretation boundaries.
    pub limitations: Vec<String>,
}

/// Builds a local-only configuration for the first-run demo.
#[must_use]
pub fn demo_config(bind: SocketAddr) -> AppConfig {
    let mut config = AppConfig::default();
    config.api.bind = bind;
    config.api.allow_lan = false;
    config.api.cors_allowed_origins.clear();
    config.runtime.demo_event_loop = false;
    config.obs.adapter = ObsAdapterKind::DryRun;
    config.relay.enabled = false;
    config.relay.auto_start = false;
    config.relay.supervisor_mode = RelaySupervisorMode::DryRun;
    for process in &mut config.relay.processes {
        process.enabled = false;
    }
    config.metrics.enabled = true;
    config.metrics.source = MetricsSourceKind::Demo;
    config.metrics.auto_poll = false;
    config.metrics.allow_demo_samples = true;
    config.security.dashboard_auth_enabled = false;
    config.security.allow_loopback_without_token = true;
    config
}

/// Builds fixed demo snapshots for seeding the local dashboard.
#[must_use]
pub fn demo_snapshots() -> Vec<RelayMetricsSnapshot> {
    [
        MetricsScenario::Healthy,
        MetricsScenario::Degraded,
        MetricsScenario::Brownout,
        MetricsScenario::Offline,
        MetricsScenario::Healthy,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, scenario)| {
        simulated_relay_snapshot(
            scenario,
            DEMO_START_TIMESTAMP_MS + index as u64 * DEMO_STEP_INTERVAL_MS,
        )
    })
    .collect()
}

/// Builds a readiness report without executing source or live dependency checks.
pub fn build_readiness_report(
    config: &AppConfig,
    schema_revision: u16,
    mode: RuntimeMode,
    agent_serving: bool,
) -> Result<ReadinessReport, ReadinessError> {
    let validation = config.validate();
    let config_status = if validation.ok {
        ReadinessStatus::Passed
    } else {
        ReadinessStatus::Blocked
    };
    let demo_status = if mode == RuntimeMode::Demo {
        ReadinessStatus::Demonstrated
    } else {
        ReadinessStatus::NotRun
    };
    let checks = vec![
        check(
            "source-static-validation",
            EvidenceScope::Source,
            ReadinessStatus::NotRun,
            "Static repository validation",
            "python3 scripts/static_validate.py",
            "The agent never infers a source result from runtime state.",
        ),
        check(
            "source-workspace-ci",
            EvidenceScope::Source,
            ReadinessStatus::NotRun,
            "Rust workspace validation",
            "cargo xtask ci",
            "Record the exact commit and command result separately.",
        ),
        check(
            "local-config",
            EvidenceScope::LocalRuntime,
            config_status,
            "Configuration safety validation",
            "openirl-agent readiness",
            if validation.ok {
                "The share-safe configuration summary has no blocking validation findings."
            } else {
                "Resolve blocking configuration findings before serving the agent."
            },
        ),
        check(
            "local-agent",
            EvidenceScope::LocalRuntime,
            if agent_serving {
                ReadinessStatus::Passed
            } else {
                ReadinessStatus::NotRun
            },
            "Local agent and readiness API",
            "GET /api/readiness",
            "This proves only the local process and API response.",
        ),
        check(
            "local-demo-scenario",
            EvidenceScope::LocalRuntime,
            demo_status,
            "Deterministic health and profile demonstration",
            "openirl-agent demo",
            "Synthetic metrics and a redacted localhost profile do not prove media transport.",
        ),
        live_check(
            "live-obs",
            "OBS Studio and OBS WebSocket",
            "scripts/obs/reconcile-smoke.sh or .ps1",
        ),
        live_check(
            "live-mediamtx",
            "MediaMTX ingest and metrics",
            "scripts/ingest/local-ingest-smoke.sh or .ps1",
        ),
        live_check(
            "live-mobile-encoder",
            "Mobile encoder profile import and contribution",
            "scripts/mobile/profile-compat-smoke.sh or .ps1",
        ),
        live_check(
            "live-relay",
            "Relay, SRTLA, and tunnel path",
            "scripts/relay/self-hosted-relay-smoke.sh or .ps1",
        ),
        live_check(
            "live-windows-package",
            "Windows package installation",
            "scripts/windows/build-alpha-portable.ps1",
        ),
    ];
    let summary = summarize(&checks);
    let demo = if mode == RuntimeMode::Demo {
        Some(build_demo_evidence()?)
    } else {
        None
    };

    Ok(ReadinessReport {
        schema_revision,
        report_revision: READINESS_REPORT_REVISION,
        mode,
        config: readiness_config_summary(config),
        checks,
        summary,
        demo,
        limitations: vec![
            "Source checks require separately recorded command results at the same revision."
                .to_string(),
            "Live environment checks require the named external dependency and matching evidence."
                .to_string(),
            "Demo mode never establishes OBS, MediaMTX, encoder, relay, network, or installer compatibility."
                .to_string(),
        ],
    })
}

fn readiness_config_summary(config: &AppConfig) -> ReadinessConfigSummary {
    let api_listener = if config.api.allow_lan {
        ListenerExposure::LanOptIn
    } else if config.api.bind.ip().is_loopback() {
        ListenerExposure::LoopbackOnly
    } else {
        ListenerExposure::NonLoopback
    };
    let artifact_destinations_configured = [
        config.artifacts.fallback_assets_dir.as_str(),
        config.artifacts.obs_templates_dir.as_str(),
        config.artifacts.support_bundles_dir.as_str(),
        config.artifacts.field_reports_dir.as_str(),
        config.artifacts.alpha_package_dir.as_str(),
    ]
    .iter()
    .all(|value| !value.trim().is_empty());
    ReadinessConfigSummary {
        api_listener,
        dashboard_auth_enabled: config.security.dashboard_auth_enabled,
        auth_required_outside_loopback: config.security.require_auth_outside_localhost,
        allowed_origin_count: config.api.cors_allowed_origins.len(),
        obs_adapter: config.obs.adapter,
        relay_enabled: config.relay.enabled,
        relay_supervisor_mode: config.relay.supervisor_mode,
        enabled_relay_process_count: config
            .relay
            .processes
            .iter()
            .filter(|process| process.enabled)
            .count(),
        metrics_enabled: config.metrics.enabled,
        metrics_source: config.metrics.source,
        artifact_destinations_configured,
    }
}

fn build_demo_evidence() -> Result<DemoEvidence, ReadinessError> {
    let mut engine = HealthEngine::new();
    let steps = demo_snapshots()
        .into_iter()
        .enumerate()
        .map(|(index, snapshot)| {
            let metrics = snapshot.to_stream_metrics();
            let decision = engine.evaluate(&metrics)?;
            Ok(DemoStep {
                sequence: index + 1,
                scenario: MetricsScenario::parse(snapshot.source.trim_start_matches("demo-"))
                    .unwrap_or(MetricsScenario::Healthy),
                timestamp_ms: snapshot.timestamp_ms,
                metrics,
                decision,
            })
        })
        .collect::<Result<Vec<_>, HealthError>>()?;
    let profile = generate_profile(&ProfileRequest {
        encoder: EncoderKind::Moblin,
        protocol: Protocol::Srt,
        host: "127.0.0.1".to_string(),
        port: 9000,
        stream_id: "openirl-demo".to_string(),
        passphrase: Some("synthetic-demo-passphrase".to_string()),
        latency_ms: 1_800,
        bitrate_kbps: 4_500,
    })?;

    Ok(DemoEvidence {
        scenario_revision: DEMO_SCENARIO_REVISION,
        deterministic: true,
        outbound_network_requests_made: false,
        external_processes_started: false,
        credentials_required: false,
        steps,
        profile: DemoProfilePreview {
            encoder: profile.encoder,
            protocol: profile.protocol,
            display_url: profile.display_url,
            synthetic: true,
        },
    })
}

fn live_check(id: &str, label: &str, evidence: &str) -> ReadinessCheck {
    check(
        id,
        EvidenceScope::LiveEnvironment,
        ReadinessStatus::NotRun,
        label,
        evidence,
        "No live result is inferred from source code or demo mode.",
    )
}

fn check(
    id: &str,
    scope: EvidenceScope,
    status: ReadinessStatus,
    label: &str,
    evidence: &str,
    note: &str,
) -> ReadinessCheck {
    ReadinessCheck {
        id: id.to_string(),
        scope,
        status,
        label: label.to_string(),
        evidence: evidence.to_string(),
        note: note.to_string(),
    }
}

fn summarize(checks: &[ReadinessCheck]) -> ReadinessSummary {
    ReadinessSummary {
        source: summarize_scope(checks, EvidenceScope::Source),
        local_runtime: summarize_scope(checks, EvidenceScope::LocalRuntime),
        live_environment: summarize_scope(checks, EvidenceScope::LiveEnvironment),
    }
}

fn summarize_scope(checks: &[ReadinessCheck], scope: EvidenceScope) -> ScopeSummary {
    let matching = checks.iter().filter(|check| check.scope == scope);
    ScopeSummary {
        total: matching.clone().count(),
        satisfied: matching
            .clone()
            .filter(|check| check.status.satisfied())
            .count(),
        not_run: matching
            .clone()
            .filter(|check| check.status == ReadinessStatus::NotRun)
            .count(),
        blocked: matching
            .filter(|check| check.status == ReadinessStatus::Blocked)
            .count(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_report_is_deterministic_and_redacted() -> Result<(), Box<dyn std::error::Error>> {
        let config = demo_config("127.0.0.1:7707".parse()?);
        let first = build_readiness_report(&config, 38, RuntimeMode::Demo, true)?;
        let second = build_readiness_report(&config, 38, RuntimeMode::Demo, true)?;
        assert_eq!(first, second);
        let serialized = serde_json::to_string(&first)?;
        assert!(!serialized.contains("synthetic-demo-passphrase"));
        assert!(serialized.contains("<redacted>"));
        assert_eq!(first.summary.live_environment.satisfied, 0);
        assert_eq!(first.summary.live_environment.not_run, 5);
        Ok(())
    }

    #[test]
    fn demo_config_cannot_start_external_media_processes() -> Result<(), Box<dyn std::error::Error>>
    {
        let config = demo_config("127.0.0.1:7707".parse()?);
        assert!(config.api.bind.ip().is_loopback());
        assert!(!config.api.allow_lan);
        assert_eq!(config.obs.adapter, ObsAdapterKind::DryRun);
        assert!(!config.relay.enabled);
        assert!(!config.relay.auto_start);
        assert!(
            config
                .relay
                .processes
                .iter()
                .all(|process| !process.enabled)
        );
        assert_eq!(config.metrics.source, MetricsSourceKind::Demo);
        assert!(!config.metrics.auto_poll);
        Ok(())
    }

    #[test]
    fn standard_report_does_not_infer_source_or_live_results()
    -> Result<(), Box<dyn std::error::Error>> {
        let report =
            build_readiness_report(&AppConfig::default(), 38, RuntimeMode::Standard, false)?;
        assert_eq!(report.summary.source.satisfied, 0);
        assert_eq!(report.summary.live_environment.satisfied, 0);
        assert!(report.demo.is_none());
        Ok(())
    }

    #[test]
    fn readiness_config_omits_private_topology_paths_and_credentials()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut config = AppConfig::default();
        config.obs.host = "private-obs.internal".to_string();
        config.ingest.public_host = "private-ingest.internal".to_string();
        config.relay.mediamtx_api_url =
            "http://operator:synthetic-password@private-router.internal:9997/api".to_string();
        config.metrics.mediamtx_metrics_url =
            "http://private-metrics.internal:9998/metrics?accessToken=synthetic-token".to_string();
        config.relay.processes[0].working_dir = Some(r"C:\\Users\\operator\\OpenIRL".to_string());
        config.artifacts.support_bundles_dir = r"\\private-share\openirl\support".to_string();

        let report = build_readiness_report(&config, 38, RuntimeMode::Standard, false)?;
        let serialized = serde_json::to_string(&report)?;
        for private_value in [
            "private-obs.internal",
            "private-ingest.internal",
            "private-router.internal",
            "private-metrics.internal",
            "synthetic-password",
            "synthetic-token",
            "operator",
            "private-share",
        ] {
            assert!(!serialized.contains(private_value));
        }
        assert_eq!(report.config.api_listener, ListenerExposure::LoopbackOnly);
        assert_eq!(report.config.obs_adapter, config.obs.adapter);
        assert_eq!(report.config.metrics_source, config.metrics.source);
        Ok(())
    }
}
