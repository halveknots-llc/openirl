//! TOML configuration for OpenIRL.

use openirl_core::{DeploymentMode, SceneBundle, SceneNames};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs, net::SocketAddr, path::Path};
use thiserror::Error;

const DEFAULT_HISTORY_LIMIT: usize = 512;

/// Configuration error.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Filesystem error.
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
    /// TOML decode error.
    #[error("TOML decode error: {0}")]
    Decode(#[from] toml::de::Error),
    /// TOML encode error.
    #[error("TOML encode error: {0}")]
    Encode(#[from] toml::ser::Error),
}

/// Full application config.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// Local API config.
    pub api: ApiConfig,
    /// Runtime behavior config.
    pub runtime: RuntimeConfig,
    /// OBS automation config.
    pub obs: ObsConfig,
    /// Ingest listener config.
    pub ingest: IngestConfig,
    /// Relay config.
    pub relay: RelayConfig,
    /// Metric polling and reducer config.
    pub metrics: MetricsConfig,
    /// Security config.
    pub security: SecurityConfig,
    /// Disk artifact output config.
    pub artifacts: ArtifactsConfig,
    /// Scene names.
    pub scenes: SceneConfig,
}

impl AppConfig {
    /// Builds the configured scene bundle.
    #[must_use]
    pub fn scene_bundle(&self) -> SceneBundle {
        self.scenes.to_scene_bundle()
    }

    /// Returns a dashboard-safe config snapshot.
    #[must_use]
    pub fn redacted(&self) -> RedactedAppConfig {
        RedactedAppConfig {
            api: self.api.clone(),
            runtime: self.runtime.clone(),
            obs: RedactedObsConfig {
                adapter: self.obs.adapter,
                host: self.obs.host.clone(),
                port: self.obs.port,
                password_env: self.obs.password_env.clone(),
                password_value: "<redacted-env-value>".to_string(),
                rpc_version: self.obs.rpc_version,
                request_timeout_ms: self.obs.request_timeout_ms,
                create_missing_scenes: self.obs.create_missing_scenes,
            },
            ingest: self.ingest.clone(),
            relay: self.relay.clone(),
            metrics: self.metrics.clone(),
            security: self.security.clone(),
            artifacts: self.artifacts.clone(),
            scenes: self.scenes.clone(),
        }
    }

    /// Builds a readiness-oriented validation report for this config.
    #[must_use]
    pub fn validate(&self) -> ConfigValidationReport {
        validate_config(self)
    }
}

/// Validation severity for config readiness checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ValidationSeverity {
    /// Informational finding.
    Info,
    /// Non-blocking risk or improvement.
    Warning,
    /// Blocking issue for safe production use.
    Error,
}

/// One config validation finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigValidationIssue {
    /// Stable machine-readable issue code.
    pub code: String,
    /// Issue severity.
    pub severity: ValidationSeverity,
    /// Human-readable issue summary.
    pub message: String,
    /// Suggested fix.
    pub remediation: String,
}

/// Aggregate config validation report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfigValidationReport {
    /// True when no error-severity issues were found.
    pub ok: bool,
    /// Number of error-severity findings.
    pub error_count: usize,
    /// Number of warning-severity findings.
    pub warning_count: usize,
    /// Number of info-severity findings.
    pub info_count: usize,
    /// Ordered validation findings.
    pub issues: Vec<ConfigValidationIssue>,
}

impl ConfigValidationReport {
    /// Builds a report from ordered issues.
    #[must_use]
    pub fn from_issues(issues: Vec<ConfigValidationIssue>) -> Self {
        let error_count = issues
            .iter()
            .filter(|issue| issue.severity == ValidationSeverity::Error)
            .count();
        let warning_count = issues
            .iter()
            .filter(|issue| issue.severity == ValidationSeverity::Warning)
            .count();
        let info_count = issues
            .iter()
            .filter(|issue| issue.severity == ValidationSeverity::Info)
            .count();
        Self {
            ok: error_count == 0,
            error_count,
            warning_count,
            info_count,
            issues,
        }
    }
}

impl Default for ConfigValidationReport {
    fn default() -> Self {
        Self::from_issues(Vec::new())
    }
}

/// Dashboard-safe config snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactedAppConfig {
    /// Local API config.
    pub api: ApiConfig,
    /// Runtime behavior config.
    pub runtime: RuntimeConfig,
    /// Redacted OBS config.
    pub obs: RedactedObsConfig,
    /// Ingest listener config.
    pub ingest: IngestConfig,
    /// Relay config. Process env values are config values only; passphrases are referenced by env name.
    pub relay: RelayConfig,
    /// Dashboard-safe metrics config.
    pub metrics: MetricsConfig,
    /// Security config.
    pub security: SecurityConfig,
    /// Disk artifact output config.
    pub artifacts: ArtifactsConfig,
    /// Scene names.
    pub scenes: SceneConfig,
}

/// Dashboard-safe OBS config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactedObsConfig {
    /// Adapter to use.
    pub adapter: ObsAdapterKind,
    /// OBS host.
    pub host: String,
    /// OBS WebSocket port.
    pub port: u16,
    /// Environment variable containing password.
    pub password_env: String,
    /// Redacted password value sample.
    pub password_value: String,
    /// OBS WebSocket RPC version.
    pub rpc_version: u32,
    /// OBS WebSocket request timeout in milliseconds.
    pub request_timeout_ms: u64,
    /// Create missing scenes during startup.
    pub create_missing_scenes: bool,
}

/// API config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ApiConfig {
    /// Bind address.
    pub bind: SocketAddr,
    /// Allow LAN access.
    pub allow_lan: bool,
    /// In-memory samples/events retained for the dashboard.
    pub history_limit: usize,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            bind: SocketAddr::from(([127, 0, 0, 1], 7707)),
            allow_lan: false,
            history_limit: DEFAULT_HISTORY_LIMIT,
        }
    }
}

/// Runtime behavior config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RuntimeConfig {
    /// Enables deterministic mock metric sampling for local demos/tests.
    pub demo_event_loop: bool,
    /// Demo event-loop tick interval in milliseconds.
    pub demo_tick_ms: u64,
    /// Maximum retained in-memory samples/events.
    pub history_limit: usize,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            demo_event_loop: false,
            demo_tick_ms: 2_500,
            history_limit: DEFAULT_HISTORY_LIMIT,
        }
    }
}

/// OBS adapter mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ObsAdapterKind {
    /// In-memory adapter for development and tests.
    #[default]
    DryRun,
    /// OBS WebSocket v5 adapter.
    WebSocket,
}

/// OBS automation config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ObsConfig {
    /// Adapter to use.
    pub adapter: ObsAdapterKind,
    /// OBS host.
    pub host: String,
    /// OBS WebSocket port.
    pub port: u16,
    /// Environment variable containing password.
    pub password_env: String,
    /// OBS WebSocket RPC version.
    pub rpc_version: u32,
    /// OBS WebSocket request timeout in milliseconds.
    pub request_timeout_ms: u64,
    /// Create missing scenes during startup.
    pub create_missing_scenes: bool,
}

impl Default for ObsConfig {
    fn default() -> Self {
        Self {
            adapter: ObsAdapterKind::DryRun,
            host: "127.0.0.1".to_string(),
            port: 4455,
            password_env: "OPENIRL_OBS_PASSWORD".to_string(),
            rpc_version: 1,
            request_timeout_ms: 3_000,
            create_missing_scenes: true,
        }
    }
}

/// Ingest config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct IngestConfig {
    /// Public host or relay host shown in profiles.
    pub public_host: String,
    /// SRT port.
    pub srt_port: u16,
    /// SRTLA port.
    pub srtla_port: u16,
    /// RTMP port.
    pub rtmp_port: u16,
    /// Default contribution latency.
    pub default_latency_ms: u32,
}

impl Default for IngestConfig {
    fn default() -> Self {
        Self {
            public_host: "127.0.0.1".to_string(),
            srt_port: 9000,
            srtla_port: 9001,
            rtmp_port: 1935,
            default_latency_ms: 1800,
        }
    }
}

/// Metrics source selection.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MetricsSourceKind {
    /// Metrics polling is disabled.
    #[default]
    Disabled,
    /// Deterministic local samples for dashboard demos and tests.
    Demo,
    /// MediaMTX Prometheus-compatible metrics endpoint.
    MediaMtxPrometheus,
    /// SRTLA process log/status line ingestion.
    SrtlaLog,
}

/// Metrics ingestion config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MetricsConfig {
    /// Enables metric ingestion endpoints and optional polling loop.
    pub enabled: bool,
    /// Source used by the automatic poll loop.
    pub source: MetricsSourceKind,
    /// Poll MediaMTX/demo metrics automatically when the agent starts.
    pub auto_poll: bool,
    /// Poll interval in milliseconds.
    pub poll_interval_ms: u64,
    /// HTTP request timeout for local metrics endpoints in milliseconds.
    pub request_timeout_ms: u64,
    /// Prometheus metrics endpoint used by MediaMTX-compatible routers.
    pub mediamtx_metrics_url: String,
    /// Switch OBS scenes when polled/ingested metrics change the health state.
    pub auto_switch_scenes: bool,
    /// Preserve deterministic demo samples even when real relay metrics are unavailable.
    pub allow_demo_samples: bool,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            source: MetricsSourceKind::Disabled,
            auto_poll: false,
            poll_interval_ms: 2_500,
            request_timeout_ms: 2_000,
            mediamtx_metrics_url: "http://127.0.0.1:9998/metrics".to_string(),
            auto_switch_scenes: true,
            allow_demo_samples: true,
        }
    }
}

/// Relay supervisor mode.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RelaySupervisorMode {
    /// Plan and log lifecycle actions without starting external media tools.
    #[default]
    DryRun,
    /// Start and supervise process-bound external media tools.
    Process,
}

/// Relay config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RelayConfig {
    /// Whether relay mode is enabled.
    pub enabled: bool,
    /// Deployment mode.
    pub mode: DeploymentMode,
    /// Media router selection.
    pub media_router: String,
    /// Supervisor behavior for external tools.
    pub supervisor_mode: RelaySupervisorMode,
    /// Start relay processes when the local agent starts.
    pub auto_start: bool,
    /// Restart backoff in milliseconds for future watchdog loops.
    pub restart_backoff_ms: u64,
    /// Process status/lifecycle timeout in milliseconds.
    pub process_timeout_ms: u64,
    /// Generated MediaMTX config path.
    pub mediamtx_config_path: String,
    /// MediaMTX Control API URL.
    pub mediamtx_api_url: String,
    /// MediaMTX Prometheus metrics URL.
    pub mediamtx_metrics_url: String,
    /// Environment variable containing the SRT/SRTLA passphrase.
    pub passphrase_env: String,
    /// Supervised relay/media-router process definitions.
    pub processes: Vec<RelayProcessConfig>,
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: DeploymentMode::LocalDirect,
            media_router: "mediamtx".to_string(),
            supervisor_mode: RelaySupervisorMode::DryRun,
            auto_start: false,
            restart_backoff_ms: 1_500,
            process_timeout_ms: 2_000,
            mediamtx_config_path: "deploy/mediamtx/openirl.mediamtx.yml".to_string(),
            mediamtx_api_url: "http://127.0.0.1:9997".to_string(),
            mediamtx_metrics_url: "http://127.0.0.1:9998/metrics".to_string(),
            passphrase_env: "OPENIRL_SRT_PASSPHRASE".to_string(),
            processes: vec![RelayProcessConfig::mediamtx_default()],
        }
    }
}

/// Supervised relay/media-router process config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RelayProcessConfig {
    /// Stable process name.
    pub name: String,
    /// Process kind.
    pub kind: RelayProcessKind,
    /// Whether this process may be started.
    pub enabled: bool,
    /// Executable name or absolute path.
    pub executable: String,
    /// Environment variable that can override the executable.
    pub executable_env: String,
    /// Command-line arguments.
    pub args: Vec<String>,
    /// Version probe arguments.
    pub version_args: Vec<String>,
    /// Optional working directory.
    pub working_dir: Option<String>,
    /// Environment variables passed to the process.
    pub env: Vec<RelayEnvVar>,
    /// Whether future watchdogs should restart this process if it exits.
    pub restart_on_exit: bool,
}

impl RelayProcessConfig {
    /// Default MediaMTX process shell.
    #[must_use]
    pub fn mediamtx_default() -> Self {
        Self {
            name: "mediamtx".to_string(),
            kind: RelayProcessKind::MediaMtx,
            enabled: false,
            executable: "mediamtx".to_string(),
            executable_env: "OPENIRL_MEDIAMTX_PATH".to_string(),
            args: vec!["deploy/mediamtx/openirl.mediamtx.yml".to_string()],
            version_args: vec!["--version".to_string()],
            working_dir: None,
            env: Vec::new(),
            restart_on_exit: true,
        }
    }

    /// Default SRTLA receiver process shell.
    #[must_use]
    pub fn srtla_receive_default() -> Self {
        Self {
            name: "srtla-rec".to_string(),
            kind: RelayProcessKind::SrtlaReceive,
            enabled: false,
            executable: "srtla_rec".to_string(),
            executable_env: "OPENIRL_SRTLA_REC_PATH".to_string(),
            args: vec![
                "9001".to_string(),
                "127.0.0.1".to_string(),
                "9000".to_string(),
            ],
            version_args: vec!["--help".to_string()],
            working_dir: None,
            env: Vec::new(),
            restart_on_exit: true,
        }
    }
}

impl Default for RelayProcessConfig {
    fn default() -> Self {
        Self::mediamtx_default()
    }
}

/// Relay process kind.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RelayProcessKind {
    /// MediaMTX media router.
    #[default]
    MediaMtx,
    /// SRTLA receive/relay process.
    SrtlaReceive,
    /// SRTLA sender/forwarder process.
    SrtlaSend,
    /// SRT live-transmit helper.
    SrtLiveTransmit,
    /// Custom operator-managed process.
    Custom,
}

impl RelayProcessKind {
    /// Stable label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::MediaMtx => "mediamtx",
            Self::SrtlaReceive => "srtla-receive",
            Self::SrtlaSend => "srtla-send",
            Self::SrtLiveTransmit => "srt-live-transmit",
            Self::Custom => "custom",
        }
    }
}

/// Environment variable for a supervised relay process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayEnvVar {
    /// Environment variable key.
    pub key: String,
    /// Environment variable value.
    pub value: String,
}

/// Security flags.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SecurityConfig {
    /// Redact logs by default.
    pub redact_logs: bool,
    /// Require auth outside localhost.
    pub require_auth_outside_localhost: bool,
    /// Redact IPs in support bundles.
    pub support_bundle_redact_ips: bool,
    /// Environment variable containing a dashboard/operator auth token.
    pub dashboard_token_env: String,
    /// Enable token checks for dashboard/operator APIs.
    pub dashboard_auth_enabled: bool,
    /// Allow localhost browser use without a token while keeping LAN auth required.
    pub allow_loopback_without_token: bool,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            redact_logs: true,
            require_auth_outside_localhost: true,
            support_bundle_redact_ips: true,
            dashboard_token_env: "OPENIRL_DASHBOARD_TOKEN".to_string(),
            dashboard_auth_enabled: false,
            allow_loopback_without_token: true,
        }
    }
}

/// Disk artifact output config.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ArtifactsConfig {
    /// Fallback/browser-source asset output directory.
    pub fallback_assets_dir: String,
    /// OBS scene/source template output directory.
    pub obs_templates_dir: String,
    /// Disk support bundle output directory.
    pub support_bundles_dir: String,
    /// Field report output directory.
    pub field_reports_dir: String,
    /// Private alpha package layout directory.
    pub private_alpha_dir: String,
    /// Whether generated assets/templates may overwrite existing files.
    pub overwrite_existing: bool,
}

impl Default for ArtifactsConfig {
    fn default() -> Self {
        Self {
            fallback_assets_dir: "artifacts/assets/fallback".to_string(),
            obs_templates_dir: "artifacts/obs-templates".to_string(),
            support_bundles_dir: "artifacts/support-bundles".to_string(),
            field_reports_dir: "artifacts/field-reports".to_string(),
            private_alpha_dir: "artifacts/private-alpha".to_string(),
            overwrite_existing: false,
        }
    }
}

/// Scene names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SceneConfig {
    /// Live scene.
    pub live: String,
    /// Low signal scene.
    pub low_signal: String,
    /// BRB scene.
    pub brb: String,
    /// Backup scene.
    pub backup: String,
    /// Privacy scene.
    pub privacy: String,
    /// Starting scene.
    pub starting: String,
    /// Ending scene.
    pub ending: String,
}

impl SceneConfig {
    /// Converts configured scene names into a semantic scene bundle.
    #[must_use]
    pub fn to_scene_bundle(&self) -> SceneBundle {
        SceneBundle::from_names(
            "OpenIRL Configured IRL Bundle",
            SceneNames {
                live: self.live.clone(),
                low_signal: self.low_signal.clone(),
                brb: self.brb.clone(),
                backup_feed: self.backup.clone(),
                privacy: self.privacy.clone(),
                starting_soon: self.starting.clone(),
                ending: self.ending.clone(),
            },
        )
    }
}

impl Default for SceneConfig {
    fn default() -> Self {
        Self {
            live: "OpenIRL Live".to_string(),
            low_signal: "OpenIRL Low Signal".to_string(),
            brb: "OpenIRL BRB".to_string(),
            backup: "OpenIRL Backup Feed".to_string(),
            privacy: "OpenIRL Privacy".to_string(),
            starting: "OpenIRL Starting Soon".to_string(),
            ending: "OpenIRL Ending".to_string(),
        }
    }
}

/// Validates config for safe local-agent startup and LAN exposure risk.
#[must_use]
pub fn validate_config(config: &AppConfig) -> ConfigValidationReport {
    let mut issues = Vec::new();

    let bind_ip = config.api.bind.ip();
    if !bind_ip.is_loopback() {
        if config.security.require_auth_outside_localhost {
            issues.push(issue(
                "api.public-bind.auth-required",
                ValidationSeverity::Warning,
                "API bind address is reachable beyond localhost; authentication must be enforced.",
                "Keep require_auth_outside_localhost enabled and set a strong dashboard token before LAN use.",
            ));
            if !config.security.dashboard_auth_enabled {
                issues.push(issue(
                    "api.public-bind.auth-disabled",
                    ValidationSeverity::Error,
                    "API bind address is reachable beyond localhost while dashboard auth is not enabled.",
                    "Set security.dashboard_auth_enabled=true and configure a strong dashboard token before LAN use.",
                ));
            }
        } else {
            issues.push(issue(
                "api.public-bind.no-auth",
                ValidationSeverity::Error,
                "API bind address is reachable beyond localhost while outside-localhost auth is disabled.",
                "Bind to 127.0.0.1 or enable require_auth_outside_localhost before exposing the dashboard.",
            ));
        }

        if !config.api.allow_lan {
            issues.push(issue(
                "api.public-bind.allow-lan-false",
                ValidationSeverity::Warning,
                "API bind address is public/LAN while allow_lan is false.",
                "Either bind to localhost or set allow_lan=true intentionally with dashboard auth.",
            ));
        }
    }

    if (config.api.allow_lan || config.security.dashboard_auth_enabled)
        && config.security.dashboard_token_env.trim().is_empty()
    {
        issues.push(issue(
            "security.dashboard-token-env-empty",
            ValidationSeverity::Error,
            "Dashboard auth/LAN access is enabled without a dashboard token environment variable name.",
            "Set security.dashboard_token_env to a non-empty environment variable name.",
        ));
    }

    if config.security.dashboard_auth_enabled && config.security.allow_loopback_without_token {
        issues.push(issue(
            "security.loopback-token-bypass",
            ValidationSeverity::Info,
            "Dashboard auth is enabled but loopback browser access may omit the token.",
            "Set allow_loopback_without_token=false when testing full token enforcement locally.",
        ));
    }

    validate_obs(config, &mut issues);
    validate_ingest(config, &mut issues);
    validate_relay(config, &mut issues);
    validate_metrics(config, &mut issues);
    validate_artifacts(config, &mut issues);

    if !config.security.redact_logs {
        issues.push(issue(
            "security.log-redaction-disabled",
            ValidationSeverity::Warning,
            "Log redaction is disabled.",
            "Keep log redaction enabled by default because IRL configs can contain stream keys, passphrases, and location-adjacent network data.",
        ));
    }

    ConfigValidationReport::from_issues(issues)
}

fn validate_obs(config: &AppConfig, issues: &mut Vec<ConfigValidationIssue>) {
    if matches!(config.obs.adapter, ObsAdapterKind::WebSocket) {
        if config.obs.password_env.trim().is_empty() {
            issues.push(issue(
                "obs.password-env-empty",
                ValidationSeverity::Warning,
                "OBS WebSocket mode has no password environment variable name configured.",
                "Set obs.password_env and keep OBS WebSocket authentication enabled.",
            ));
        }

        if config.obs.request_timeout_ms < 500 {
            issues.push(issue(
                "obs.timeout-too-low",
                ValidationSeverity::Warning,
                "OBS WebSocket request timeout is very low.",
                "Use at least 500ms locally and a higher value when OBS is busy or tunneled.",
            ));
        }

        if !is_local_obs_host(&config.obs.host) {
            issues.push(issue(
                "obs.remote-host",
                ValidationSeverity::Warning,
                "OBS WebSocket host is not clearly local.",
                "Prefer 127.0.0.1, localhost, or a private VPN/tunnel endpoint. Never expose OBS WebSocket directly to the internet.",
            ));
        }
    }
}

fn validate_ingest(config: &AppConfig, issues: &mut Vec<ConfigValidationIssue>) {
    if config.ingest.srt_port == config.ingest.srtla_port
        || config.ingest.srt_port == config.ingest.rtmp_port
        || config.ingest.srtla_port == config.ingest.rtmp_port
    {
        issues.push(issue(
            "ingest.port-collision",
            ValidationSeverity::Error,
            "Two or more ingest protocols are configured to use the same port.",
            "Use distinct ports for SRT, SRTLA, and RTMP listeners.",
        ));
    }

    if config.ingest.default_latency_ms < 500 {
        issues.push(issue(
            "ingest.latency-too-low",
            ValidationSeverity::Warning,
            "Default contribution latency is very low for unstable mobile IRL networks.",
            "Use a larger SRT/SRTLA latency buffer for cellular IRL contribution, then tune after field tests.",
        ));
    }

    if config.ingest.default_latency_ms > 8_000 {
        issues.push(issue(
            "ingest.latency-too-high",
            ValidationSeverity::Warning,
            "Default contribution latency is very high and may hurt chat interaction.",
            "Lower latency after confirming the route is stable enough for the target bitrate.",
        ));
    }
}

fn validate_relay(config: &AppConfig, issues: &mut Vec<ConfigValidationIssue>) {
    if config.relay.enabled && is_loopback_host(&config.ingest.public_host) {
        issues.push(issue(
            "relay.public-host-loopback",
            ValidationSeverity::Warning,
            "Relay mode is enabled but ingest.public_host is loopback.",
            "Set ingest.public_host to the relay hostname/IP that Moblin, IRL Pro, BELABOX, or Larix will reach.",
        ));
    }

    if config.relay.enabled && config.relay.media_router.trim().is_empty() {
        issues.push(issue(
            "relay.media-router-empty",
            ValidationSeverity::Error,
            "Relay mode is enabled without a media router label.",
            "Set relay.media_router to mediamtx or another explicit process-bound router.",
        ));
    }

    if config.relay.enabled && config.relay.media_router != "mediamtx" {
        issues.push(issue(
            "relay.media-router-unsupported",
            ValidationSeverity::Warning,
            "Relay mode is enabled with a media router other than mediamtx.",
            "Use media_router=\"mediamtx\" until additional process adapters are implemented.",
        ));
    }

    if config.relay.enabled && config.relay.process_timeout_ms < 500 {
        issues.push(issue(
            "relay.process-timeout-too-low",
            ValidationSeverity::Warning,
            "Relay process timeout is very low.",
            "Use at least 500ms for process lifecycle checks and higher values on slow hosts.",
        ));
    }

    if config.relay.restart_backoff_ms < 250 {
        issues.push(issue(
            "relay.restart-backoff-too-low",
            ValidationSeverity::Warning,
            "Relay restart backoff is very low.",
            "Use at least 250ms to avoid tight restart loops during bad configs.",
        ));
    }

    if !is_loopback_url(&config.relay.mediamtx_api_url) {
        issues.push(issue(
            "relay.mediamtx-api-not-local",
            ValidationSeverity::Warning,
            "MediaMTX Control API URL is not clearly loopback-local.",
            "Keep the Control API on 127.0.0.1 and proxy it only through authenticated OpenIRL endpoints.",
        ));
    }

    validate_relay_processes(config, issues);
}

fn validate_relay_processes(config: &AppConfig, issues: &mut Vec<ConfigValidationIssue>) {
    let mut names = BTreeSet::new();
    let mut enabled_count = 0usize;
    let mut has_media_mtx = false;

    for process in &config.relay.processes {
        let trimmed_name = process.name.trim();
        if trimmed_name.is_empty() {
            issues.push(issue(
                "relay.process-name-empty",
                ValidationSeverity::Error,
                "A relay process has an empty name.",
                "Set every relay.processes entry to a stable non-empty name.",
            ));
        } else if !names.insert(trimmed_name.to_ascii_lowercase()) {
            issues.push(issue(
                "relay.process-name-duplicate",
                ValidationSeverity::Error,
                "Two relay processes use the same name.",
                "Use unique relay process names so start/stop/status APIs are unambiguous.",
            ));
        }

        if process.enabled {
            enabled_count = enabled_count.saturating_add(1);
        }

        if process.kind == RelayProcessKind::MediaMtx {
            has_media_mtx = true;
        }

        if process.enabled && process.executable.trim().is_empty() {
            issues.push(issue(
                "relay.process-executable-empty",
                ValidationSeverity::Error,
                "An enabled relay process has no executable configured.",
                "Set relay.processes[].executable to a PATH executable or absolute path.",
            ));
        }

        if process.enabled && process.executable_env.trim().is_empty() {
            issues.push(issue(
                "relay.process-executable-env-empty",
                ValidationSeverity::Warning,
                "An enabled relay process has no executable override environment variable.",
                "Set relay.processes[].executable_env so deployments can override binary paths without editing config.",
            ));
        }

        for env_var in &process.env {
            if env_var.key.trim().is_empty() {
                issues.push(issue(
                    "relay.process-env-key-empty",
                    ValidationSeverity::Error,
                    "A relay process environment variable has an empty key.",
                    "Remove the empty environment entry or set a valid variable name.",
                ));
            }
        }
    }

    if config.relay.enabled
        && config.relay.supervisor_mode == RelaySupervisorMode::Process
        && enabled_count == 0
    {
        issues.push(issue(
            "relay.no-enabled-processes",
            ValidationSeverity::Error,
            "Relay process mode is enabled but no relay processes are enabled.",
            "Enable at least one MediaMTX/SRTLA process or switch supervisor_mode to dry-run for planning only.",
        ));
    }

    if config.relay.enabled
        && config.relay.supervisor_mode == RelaySupervisorMode::DryRun
        && enabled_count == 0
    {
        issues.push(issue(
            "relay.dry-run-no-enabled-processes",
            ValidationSeverity::Info,
            "Relay is enabled in dry-run mode without enabled child processes.",
            "Enable a process when ready for real MediaMTX/SRTLA supervision.",
        ));
    }

    if config.relay.auto_start && enabled_count == 0 {
        issues.push(issue(
            "relay.autostart-without-processes",
            ValidationSeverity::Warning,
            "Relay auto_start is enabled without any enabled processes.",
            "Enable a relay process or turn auto_start off.",
        ));
    }

    if config.relay.media_router.eq_ignore_ascii_case("mediamtx") && !has_media_mtx {
        issues.push(issue(
            "relay.mediamtx-router-without-process",
            ValidationSeverity::Warning,
            "media_router is mediamtx but no MediaMTX process is configured.",
            "Add a relay.processes entry with kind=\"media-mtx\" or change relay.media_router.",
        ));
    }
}

fn validate_metrics(config: &AppConfig, issues: &mut Vec<ConfigValidationIssue>) {
    if !config.metrics.enabled && config.metrics.auto_poll {
        issues.push(issue(
            "metrics.autopoll-disabled",
            ValidationSeverity::Warning,
            "Metrics auto_poll is enabled while metrics are disabled.",
            "Enable metrics.enabled or turn metrics.auto_poll off.",
        ));
    }

    if config.metrics.poll_interval_ms < 500 {
        issues.push(issue(
            "metrics.poll-interval-too-low",
            ValidationSeverity::Warning,
            "Metrics poll interval is very low for a local desktop agent.",
            "Use at least 500ms, and prefer 1000-2500ms for normal IRL dashboard updates.",
        ));
    }

    if config.metrics.request_timeout_ms < 250 {
        issues.push(issue(
            "metrics.timeout-too-low",
            ValidationSeverity::Warning,
            "Metrics request timeout is very low.",
            "Use at least 250ms for localhost and higher values for VPS/friend relay polling.",
        ));
    }

    if matches!(config.metrics.source, MetricsSourceKind::MediaMtxPrometheus)
        && config.metrics.mediamtx_metrics_url.trim().is_empty()
    {
        issues.push(issue(
            "metrics.mediamtx-url-empty",
            ValidationSeverity::Error,
            "MediaMTX Prometheus metrics source is selected without a metrics URL.",
            "Set metrics.mediamtx_metrics_url or switch metrics.source to disabled/demo.",
        ));
    }

    if matches!(config.metrics.source, MetricsSourceKind::MediaMtxPrometheus)
        && !is_loopback_url(&config.metrics.mediamtx_metrics_url)
        && config.security.require_auth_outside_localhost
    {
        issues.push(issue(
            "metrics.mediamtx-url-not-local",
            ValidationSeverity::Warning,
            "MediaMTX metrics URL is not clearly loopback-local.",
            "Prefer polling MediaMTX metrics on 127.0.0.1 through the local agent or protect remote metrics behind a VPN/authenticated tunnel.",
        ));
    }
}

fn validate_artifacts(config: &AppConfig, issues: &mut Vec<ConfigValidationIssue>) {
    let entries = [
        (
            "artifacts.fallback-assets-dir-empty",
            &config.artifacts.fallback_assets_dir,
            "fallback_assets_dir",
        ),
        (
            "artifacts.obs-templates-dir-empty",
            &config.artifacts.obs_templates_dir,
            "obs_templates_dir",
        ),
        (
            "artifacts.support-bundles-dir-empty",
            &config.artifacts.support_bundles_dir,
            "support_bundles_dir",
        ),
        (
            "artifacts.field-reports-dir-empty",
            &config.artifacts.field_reports_dir,
            "field_reports_dir",
        ),
        (
            "artifacts.private-alpha-dir-empty",
            &config.artifacts.private_alpha_dir,
            "private_alpha_dir",
        ),
    ];

    for (code, value, label) in entries {
        if value.trim().is_empty() {
            issues.push(issue(
                code,
                ValidationSeverity::Error,
                &format!("Artifact path {label} is empty."),
                "Set all artifacts.* directories to local paths under the OpenIRL workspace or user data directory.",
            ));
        }
    }

    if config.artifacts.support_bundles_dir == config.artifacts.fallback_assets_dir {
        issues.push(issue(
            "artifacts.bundle-dir-overlaps-assets",
            ValidationSeverity::Warning,
            "Support bundles and fallback assets share the same directory.",
            "Keep support bundles separate from OBS assets so redacted diagnostics are easier to review before sharing.",
        ));
    }
}

fn issue(
    code: &str,
    severity: ValidationSeverity,
    message: &str,
    remediation: &str,
) -> ConfigValidationIssue {
    ConfigValidationIssue {
        code: code.to_string(),
        severity,
        message: message.to_string(),
        remediation: remediation.to_string(),
    }
}

fn is_local_obs_host(host: &str) -> bool {
    is_loopback_host(host) || host.ends_with(".local")
}

fn is_loopback_url(url: &str) -> bool {
    let normalized = url.trim().to_ascii_lowercase();
    normalized.starts_with("http://127.")
        || normalized.starts_with("http://localhost")
        || normalized.starts_with("https://127.")
        || normalized.starts_with("https://localhost")
}

fn is_loopback_host(host: &str) -> bool {
    let normalized = host.trim().to_ascii_lowercase();
    normalized == "localhost"
        || normalized == "127.0.0.1"
        || normalized == "::1"
        || normalized.starts_with("127.")
}

/// Loads config from TOML.
///
/// # Errors
///
/// Returns filesystem or TOML parse errors.
pub fn load_config(path: impl AsRef<Path>) -> Result<AppConfig, ConfigError> {
    let raw = fs::read_to_string(path)?;
    Ok(toml::from_str(&raw)?)
}

/// Saves config to TOML.
///
/// # Errors
///
/// Returns filesystem or TOML encode errors.
pub fn save_config(path: impl AsRef<Path>, config: &AppConfig) -> Result<(), ConfigError> {
    let raw = toml::to_string_pretty(config)?;
    fs::write(path, raw)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use openirl_core::SceneRole;

    #[test]
    fn config_round_trips_to_toml() -> Result<(), Box<dyn std::error::Error>> {
        let config = AppConfig::default();
        let raw = toml::to_string(&config)?;
        let decoded: AppConfig = toml::from_str(&raw)?;
        assert_eq!(decoded, config);
        Ok(())
    }

    #[test]
    fn missing_new_fields_use_defaults() -> Result<(), Box<dyn std::error::Error>> {
        let decoded: AppConfig = toml::from_str(
            r#"
            [api]
            bind = "127.0.0.1:7707"
            allow_lan = false

            [obs]
            host = "127.0.0.1"
            port = 4455
            password_env = "OPENIRL_OBS_PASSWORD"
            "#,
        )?;
        assert_eq!(decoded.obs.adapter, ObsAdapterKind::DryRun);
        assert_eq!(decoded.api.history_limit, DEFAULT_HISTORY_LIMIT);
        assert_eq!(decoded.relay.processes.len(), 1);
        Ok(())
    }

    #[test]
    fn scene_config_builds_default_bundle() {
        let bundle = AppConfig::default().scene_bundle();
        assert_eq!(bundle.scenes.len(), 7);
        assert_eq!(bundle.scene_name(SceneRole::Brb), Some("OpenIRL BRB"));
    }

    #[test]
    fn redacted_config_never_exposes_password_value() {
        let redacted = AppConfig::default().redacted();
        assert_eq!(redacted.obs.password_value, "<redacted-env-value>");
    }

    #[test]
    fn default_config_validation_is_ok() {
        let report = AppConfig::default().validate();
        assert!(report.ok);
        assert_eq!(report.error_count, 0);
    }

    #[test]
    fn unsafe_public_bind_is_blocking() -> Result<(), Box<dyn std::error::Error>> {
        let mut config = AppConfig::default();
        config.api.bind = "0.0.0.0:7707".parse()?;
        config.security.require_auth_outside_localhost = false;
        let report = validate_config(&config);
        assert!(!report.ok);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "api.public-bind.no-auth")
        );
        Ok(())
    }

    #[test]
    fn relay_process_mode_without_processes_is_blocking() {
        let mut config = AppConfig::default();
        config.relay.enabled = true;
        config.relay.supervisor_mode = RelaySupervisorMode::Process;
        config.relay.processes.clear();
        let report = validate_config(&config);
        assert!(!report.ok);
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.code == "relay.no-enabled-processes")
        );
    }
}
