//! Process-bound relay/media-router supervision for OpenIRL.
//!
//! feature areas deliberately supervises external media tools instead of binding
//! Rust directly to SRT, SRTLA, RTMP, or router internals. This keeps the local
//! agent safe to iterate while preserving a future path to native adapters.

use openirl_core::{DeploymentMode, Protocol};
use openirl_vault::redact_support_text;
use serde::{Deserialize, Serialize};
use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
};
use thiserror::Error;
use time::OffsetDateTime;
use tokio::{
    io::{AsyncBufReadExt, BufReader},
    process::{Child, Command},
    sync::Mutex,
};

const DEFAULT_LOG_LIMIT: usize = 200;

/// Supported process-bound relay/media backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RelayBackend {
    /// MediaMTX media router.
    MediaMtx,
    /// BELABOX-compatible SRTLA receive/relay binary.
    SrtlaReceive,
    /// BELABOX-compatible SRTLA send/forward binary.
    SrtlaSend,
    /// go-irl-compatible relay process.
    GoIrl,
    /// SRT live-transmit helper.
    SrtLiveTransmit,
    /// Custom executable supplied by the operator.
    Custom,
}

impl RelayBackend {
    /// Parses a backend name used in configuration.
    #[must_use]
    pub fn from_router_name(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "mediamtx" | "media-mtx" => Self::MediaMtx,
            "srtla" | "srtla-receive" | "srtla_rec" | "srtla-rec" => Self::SrtlaReceive,
            "srtla-send" | "srtla_send" => Self::SrtlaSend,
            "go-irl" | "goirl" => Self::GoIrl,
            "srt-live-transmit" | "srt_live_transmit" => Self::SrtLiveTransmit,
            _ => Self::Custom,
        }
    }

    /// Returns candidate executable names for PATH discovery.
    #[must_use]
    pub fn candidate_names(self) -> &'static [&'static str] {
        match self {
            Self::MediaMtx => &["mediamtx"],
            Self::SrtlaReceive => &["srtla_rec", "srtla-receive", "srtla"],
            Self::SrtlaSend => &["srtla_send", "srtla-send", "srtla"],
            Self::GoIrl => &["go-irl", "goirl"],
            Self::SrtLiveTransmit => &["srt-live-transmit"],
            Self::Custom => &[],
        }
    }

    /// Returns protocols this backend is expected to support in OpenIRL mode.
    #[must_use]
    pub fn planned_protocols(self) -> &'static [Protocol] {
        match self {
            Self::MediaMtx => &[
                Protocol::Srt,
                Protocol::Rtmp,
                Protocol::Whip,
                Protocol::Whep,
            ],
            Self::SrtlaReceive | Self::SrtlaSend | Self::GoIrl => &[Protocol::Srtla, Protocol::Srt],
            Self::SrtLiveTransmit => &[Protocol::Srt],
            Self::Custom => &[],
        }
    }
}

/// Process environment variable passed to a supervised relay process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayEnvPair {
    /// Environment variable name.
    pub key: String,
    /// Environment variable value.
    pub value: String,
}

/// Relay process configuration used by the supervisor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayProcessConfig {
    /// Stable process name used by APIs.
    pub name: String,
    /// Whether the process may be started.
    pub enabled: bool,
    /// Auto-start the relay when the agent starts.
    pub auto_start: bool,
    /// Deployment mode.
    pub mode: DeploymentMode,
    /// Process backend.
    pub backend: RelayBackend,
    /// Executable name or explicit path.
    pub executable: String,
    /// Process arguments.
    pub args: Vec<String>,
    /// Optional working directory.
    pub working_dir: Option<PathBuf>,
    /// Environment variables.
    pub env: Vec<RelayEnvPair>,
    /// Future watchdog policy marker.
    pub restart_on_exit: bool,
    /// Optional Prometheus-compatible metrics URL.
    pub metrics_url: Option<String>,
    /// Optional media-router control API URL.
    pub api_url: Option<String>,
    /// Maximum retained log lines.
    pub log_tail_limit: usize,
    /// Redact captured child-process and supervisor logs.
    pub redact_logs: bool,
}

impl RelayProcessConfig {
    /// Returns a config safe for local dry-run use.
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            name: "mediamtx".to_string(),
            enabled: false,
            auto_start: false,
            mode: DeploymentMode::LocalDirect,
            backend: RelayBackend::MediaMtx,
            executable: "mediamtx".to_string(),
            args: Vec::new(),
            working_dir: None,
            env: Vec::new(),
            restart_on_exit: false,
            metrics_url: Some("http://127.0.0.1:9998/metrics".to_string()),
            api_url: Some("http://127.0.0.1:9997".to_string()),
            log_tail_limit: DEFAULT_LOG_LIMIT,
            redact_logs: true,
        }
    }

    fn executable_candidates(&self) -> Vec<PathBuf> {
        if self.executable.trim().is_empty() {
            return self
                .backend
                .candidate_names()
                .iter()
                .flat_map(|name| discover_executable_candidates(name))
                .collect();
        }

        let executable = self.executable.trim();
        let path = Path::new(executable);
        if path.components().count() > 1 || path.is_absolute() {
            vec![PathBuf::from(executable)]
        } else {
            discover_executable_candidates(executable)
        }
    }
}

/// Executable discovery result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayExecutablePlan {
    /// Candidate executable names or paths.
    pub candidates: Vec<String>,
    /// First resolved executable path.
    pub resolved_path: Option<String>,
    /// Whether an executable was found.
    pub found: bool,
}

/// Relay launch plan exposed to the dashboard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayLaunchPlan {
    /// Stable process name.
    pub name: String,
    /// Whether the process may be started.
    pub enabled: bool,
    /// Whether auto-start is enabled.
    pub auto_start: bool,
    /// Deployment mode.
    pub mode: DeploymentMode,
    /// Backend.
    pub backend: RelayBackend,
    /// Planned protocols.
    pub planned_protocols: Vec<Protocol>,
    /// Executable discovery details.
    pub executable: RelayExecutablePlan,
    /// Redacted command line.
    pub redacted_command: Vec<String>,
    /// Optional metrics URL.
    pub metrics_url: Option<String>,
    /// Optional control API URL.
    pub api_url: Option<String>,
}

/// Redacted relay credential and URL planning output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayCredentialPlan {
    /// Stream ID assigned to the main relay path.
    pub stream_id: String,
    /// Environment variable expected to contain the real SRT passphrase.
    pub passphrase_env: String,
    /// SRT encoder URL template with passphrase redacted.
    pub srt_url_redacted: String,
    /// SRTLA encoder URL template with token/passphrase redacted.
    pub srtla_url_redacted: String,
    /// Recommended rotation window for live ingest credentials.
    pub rotation_recommended_after_hours: u16,
}

/// Builds a redacted credential/URL plan for operator setup.
#[must_use]
pub fn build_credential_plan(
    public_host: impl Into<String>,
    srt_port: u16,
    srtla_port: u16,
    stream_id: impl Into<String>,
    passphrase_env: impl Into<String>,
) -> RelayCredentialPlan {
    let public_host = public_host.into();
    let stream_id = stream_id.into();
    let passphrase_env = passphrase_env.into();
    RelayCredentialPlan {
        stream_id: stream_id.clone(),
        passphrase_env,
        srt_url_redacted: format!(
            "srt://{public_host}:{srt_port}?streamid={stream_id}&passphrase=<redacted>&latency=1800"
        ),
        srtla_url_redacted: format!(
            "srtla://{public_host}:{srtla_port}?streamid={stream_id}&token=<redacted>"
        ),
        rotation_recommended_after_hours: 24,
    }
}

/// One captured relay process log line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayLogLine {
    /// UTC timestamp.
    pub at: OffsetDateTime,
    /// Log stream, usually stdout or stderr.
    pub stream: String,
    /// Redacted line.
    pub line: String,
}

/// Runtime status returned by the supervisor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelayRuntimeStatus {
    /// Stable process name.
    pub name: String,
    /// Whether the process may be started.
    pub enabled: bool,
    /// Desired auto-start behavior.
    pub auto_start: bool,
    /// Backend.
    pub backend: RelayBackend,
    /// Deployment mode.
    pub mode: DeploymentMode,
    /// Whether the supervised process is currently running.
    pub running: bool,
    /// OS process ID, if running and available.
    pub pid: Option<u32>,
    /// Start timestamp, if running.
    pub started_at: Option<OffsetDateTime>,
    /// Last observed process exit string.
    pub last_exit: Option<String>,
    /// Executable discovery plan.
    pub executable: RelayExecutablePlan,
    /// Optional metrics URL.
    pub metrics_url: Option<String>,
    /// Optional control API URL.
    pub api_url: Option<String>,
    /// Recent captured logs.
    pub recent_logs: Vec<RelayLogLine>,
}

/// Process supervision errors.
#[derive(Debug, Error)]
pub enum RelayControlError {
    /// Relay process is disabled.
    #[error("relay process {name} is disabled")]
    Disabled {
        /// Process name.
        name: String,
    },
    /// No executable could be resolved.
    #[error("relay executable was not found for process {name}")]
    ExecutableNotFound {
        /// Process name.
        name: String,
    },
    /// Requested process name is unknown.
    #[error("unknown relay process: {name}")]
    UnknownProcess {
        /// Process name.
        name: String,
    },
    /// Process IO error.
    #[error("relay process IO error: {0}")]
    Io(#[from] std::io::Error),
}

struct RelaySupervisorInner {
    config: RelayProcessConfig,
    child: Option<Child>,
    pid: Option<u32>,
    started_at: Option<OffsetDateTime>,
    last_exit: Option<String>,
}

/// Supervises one relay/media-router child process.
#[derive(Clone)]
pub struct RelaySupervisor {
    inner: Arc<Mutex<RelaySupervisorInner>>,
    logs: Arc<Mutex<Vec<RelayLogLine>>>,
}

impl RelaySupervisor {
    /// Creates a new supervisor.
    #[must_use]
    pub fn new(config: RelayProcessConfig) -> Self {
        Self {
            inner: Arc::new(Mutex::new(RelaySupervisorInner {
                config,
                child: None,
                pid: None,
                started_at: None,
                last_exit: None,
            })),
            logs: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Returns this process name.
    pub async fn name(&self) -> String {
        self.inner.lock().await.config.name.clone()
    }

    /// Returns a launch plan without mutating process state.
    pub async fn plan(&self) -> RelayLaunchPlan {
        let config = self.inner.lock().await.config.clone();
        build_launch_plan(&config)
    }

    /// Returns runtime status and refreshes exited-child state.
    pub async fn status(&self) -> Result<RelayRuntimeStatus, RelayControlError> {
        let mut inner = self.inner.lock().await;
        inner.refresh_child_state()?;
        let config = inner.config.clone();
        let running = inner.child.is_some();
        let pid = inner.pid;
        let started_at = inner.started_at;
        let last_exit = inner.last_exit.clone();
        drop(inner);

        let recent_logs = self.logs.lock().await.clone();
        Ok(build_runtime_status(
            &config,
            running,
            pid,
            started_at,
            last_exit,
            recent_logs,
        ))
    }

    /// Starts the relay process if enabled and not already running.
    pub async fn start(&self) -> Result<RelayRuntimeStatus, RelayControlError> {
        let mut inner = self.inner.lock().await;
        inner.refresh_child_state()?;

        if !inner.config.enabled {
            return Err(RelayControlError::Disabled {
                name: inner.config.name.clone(),
            });
        }

        if inner.child.is_some() {
            drop(inner);
            return self.status().await;
        }

        let executable = resolve_executable(&inner.config).ok_or_else(|| {
            RelayControlError::ExecutableNotFound {
                name: inner.config.name.clone(),
            }
        })?;

        let mut command = Command::new(&executable);
        command.args(&inner.config.args);
        if let Some(working_dir) = &inner.config.working_dir {
            command.current_dir(working_dir);
        }
        for env_pair in &inner.config.env {
            command.env(&env_pair.key, &env_pair.value);
        }
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        command.kill_on_drop(true);

        let mut child = command.spawn()?;
        let pid = child.id();
        let started_at = OffsetDateTime::now_utc();

        if let Some(stdout) = child.stdout.take() {
            spawn_log_reader(
                self.logs.clone(),
                "stdout",
                stdout,
                inner.config.log_tail_limit,
                inner.config.redact_logs,
            );
        }

        if let Some(stderr) = child.stderr.take() {
            spawn_log_reader(
                self.logs.clone(),
                "stderr",
                stderr,
                inner.config.log_tail_limit,
                inner.config.redact_logs,
            );
        }

        push_log_now(
            self.logs.clone(),
            "supervisor",
            format!("started {} as pid {:?}", inner.config.name, pid),
            inner.config.log_tail_limit,
            inner.config.redact_logs,
        )
        .await;

        inner.pid = pid;
        inner.started_at = Some(started_at);
        inner.last_exit = None;
        inner.child = Some(child);
        drop(inner);

        self.status().await
    }

    /// Stops the relay process if it is running.
    pub async fn stop(&self) -> Result<RelayRuntimeStatus, RelayControlError> {
        let (mut child, config) = {
            let mut inner = self.inner.lock().await;
            let child = inner.child.take();
            inner.pid = None;
            inner.started_at = None;
            (child, inner.config.clone())
        };

        if let Some(child_ref) = child.as_mut() {
            child_ref.kill().await?;
            let exit = child_ref.wait().await?;
            let mut inner = self.inner.lock().await;
            inner.last_exit = Some(exit.to_string());
            drop(inner);
            push_log_now(
                self.logs.clone(),
                "supervisor",
                format!("stopped {}: {exit}", config.name),
                config.log_tail_limit,
                config.redact_logs,
            )
            .await;
        }

        self.status().await
    }

    /// Restarts the relay process.
    pub async fn restart(&self) -> Result<RelayRuntimeStatus, RelayControlError> {
        let _status = self.stop().await?;
        self.start().await
    }
}

impl RelaySupervisorInner {
    fn refresh_child_state(&mut self) -> Result<(), RelayControlError> {
        let maybe_exit = match self.child.as_mut() {
            Some(child) => child.try_wait()?,
            None => None,
        };

        if let Some(exit) = maybe_exit {
            self.last_exit = Some(exit.to_string());
            self.child = None;
            self.pid = None;
            self.started_at = None;
        }

        Ok(())
    }
}

fn build_launch_plan(config: &RelayProcessConfig) -> RelayLaunchPlan {
    let executable = build_executable_plan(config);
    let redacted_command = redacted_command(config, executable.resolved_path.clone());
    RelayLaunchPlan {
        name: config.name.clone(),
        enabled: config.enabled,
        auto_start: config.auto_start,
        mode: config.mode,
        backend: config.backend,
        planned_protocols: config.backend.planned_protocols().to_vec(),
        executable,
        redacted_command,
        metrics_url: config.metrics_url.clone(),
        api_url: config.api_url.clone(),
    }
}

fn build_runtime_status(
    config: &RelayProcessConfig,
    running: bool,
    pid: Option<u32>,
    started_at: Option<OffsetDateTime>,
    last_exit: Option<String>,
    recent_logs: Vec<RelayLogLine>,
) -> RelayRuntimeStatus {
    RelayRuntimeStatus {
        name: config.name.clone(),
        enabled: config.enabled,
        auto_start: config.auto_start,
        backend: config.backend,
        mode: config.mode,
        running,
        pid,
        started_at,
        last_exit,
        executable: build_executable_plan(config),
        metrics_url: config.metrics_url.clone(),
        api_url: config.api_url.clone(),
        recent_logs,
    }
}

fn build_executable_plan(config: &RelayProcessConfig) -> RelayExecutablePlan {
    let candidates = config
        .executable_candidates()
        .into_iter()
        .map(|candidate| candidate.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    let resolved_path = resolve_executable(config).map(|path| path.to_string_lossy().to_string());
    RelayExecutablePlan {
        candidates,
        found: resolved_path.is_some(),
        resolved_path,
    }
}

fn redacted_command(config: &RelayProcessConfig, resolved_path: Option<String>) -> Vec<String> {
    let mut command = Vec::new();
    let executable = resolved_path.unwrap_or_else(|| {
        if config.executable.trim().is_empty() {
            config
                .backend
                .candidate_names()
                .first()
                .map_or("<unresolved-relay-executable>".to_string(), |name| {
                    (*name).to_string()
                })
        } else {
            config.executable.clone()
        }
    });
    command.push(executable);
    command.extend(config.args.iter().map(|arg| redact_arg(arg)));
    command
}

fn redact_arg(value: &str) -> String {
    let lowered = value.to_ascii_lowercase();
    if lowered.contains("passphrase")
        || lowered.contains("secret")
        || lowered.contains("token")
        || lowered.contains("streamkey")
        || lowered.contains("stream_key")
    {
        "<redacted>".to_string()
    } else {
        value.to_string()
    }
}

fn resolve_executable(config: &RelayProcessConfig) -> Option<PathBuf> {
    config
        .executable_candidates()
        .into_iter()
        .find(|candidate| executable_exists(candidate))
}

fn discover_executable_candidates(name: &str) -> Vec<PathBuf> {
    let candidate_names = platform_candidate_names(name);
    let path_var = env::var_os("PATH");
    let mut candidates = Vec::new();

    if Path::new(name).components().count() > 1 {
        candidates.extend(candidate_names.iter().map(PathBuf::from));
        return candidates;
    }

    if let Some(path_var) = path_var {
        for path in env::split_paths(&path_var) {
            for candidate_name in &candidate_names {
                candidates.push(path.join(candidate_name));
            }
        }
    }

    candidates
}

fn platform_candidate_names(name: &str) -> Vec<OsString> {
    if cfg!(windows) && Path::new(name).extension().is_none() {
        let pathext = env::var_os("PATHEXT").unwrap_or_else(|| OsString::from(".EXE;.BAT;.CMD"));
        let mut names = Vec::new();
        for ext in pathext.to_string_lossy().split(';') {
            names.push(OsString::from(format!("{name}{ext}")));
        }
        names
    } else {
        vec![OsString::from(name)]
    }
}

fn executable_exists(path: &Path) -> bool {
    path.is_file()
}

fn spawn_log_reader<T>(
    logs: Arc<Mutex<Vec<RelayLogLine>>>,
    stream: &'static str,
    reader: T,
    limit: usize,
    redact_logs: bool,
) where
    T: tokio::io::AsyncRead + Send + Unpin + 'static,
{
    tokio::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    push_log_now(logs.clone(), stream, line, limit, redact_logs).await;
                }
                Ok(None) => break,
                Err(read_error) => {
                    push_log_now(
                        logs.clone(),
                        stream,
                        format!("log reader failed: {read_error}"),
                        limit,
                        redact_logs,
                    )
                    .await;
                    break;
                }
            }
        }
    });
}

async fn push_log_now(
    logs: Arc<Mutex<Vec<RelayLogLine>>>,
    stream: &str,
    line: String,
    limit: usize,
    redact_logs: bool,
) {
    let line = if redact_logs {
        redact_support_text(&line, true)
    } else {
        line
    };
    let mut logs = logs.lock().await;
    logs.push(RelayLogLine {
        at: OffsetDateTime::now_utc(),
        stream: stream.to_string(),
        line,
    });
    let limit = limit.max(1);
    if logs.len() > limit {
        let excess = logs.len().saturating_sub(limit);
        logs.drain(0..excess);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_parses_known_names() {
        assert_eq!(
            RelayBackend::from_router_name("mediamtx"),
            RelayBackend::MediaMtx
        );
        assert_eq!(
            RelayBackend::from_router_name("go-irl"),
            RelayBackend::GoIrl
        );
        assert_eq!(
            RelayBackend::from_router_name("unknown"),
            RelayBackend::Custom
        );
    }

    #[test]
    fn redaction_masks_secret_arguments() {
        assert_eq!(redact_arg("--passphrase=abc"), "<redacted>");
        assert_eq!(redact_arg("--token=abc"), "<redacted>");
        assert_eq!(redact_arg("--port=9000"), "--port=9000");
    }

    #[test]
    fn credential_plan_redacts_urls() {
        let plan =
            build_credential_plan("example.test", 9000, 9001, "main", "OPENIRL_SRT_PASSPHRASE");
        assert!(plan.srt_url_redacted.contains("passphrase=<redacted>"));
        assert!(plan.srtla_url_redacted.contains("token=<redacted>"));
    }

    #[tokio::test]
    async fn captured_logs_use_shared_redaction() {
        let logs = Arc::new(Mutex::new(Vec::new()));
        push_log_now(
            logs.clone(),
            "stdout",
            "Authorization: Bearer field-token relay=10.23.45.67".to_string(),
            10,
            true,
        )
        .await;
        let captured = logs.lock().await[0].line.clone();
        assert!(captured.contains("Bearer <redacted>"));
        assert!(captured.contains("relay=<redacted-ip>"));
        assert!(!captured.contains("field-token"));
        assert!(!captured.contains("10.23.45.67"));
    }
}
