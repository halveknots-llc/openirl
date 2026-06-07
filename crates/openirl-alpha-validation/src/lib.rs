//! Windows and OBS alpha validation planning.
//!
//! desktop alpha turns the previous release-plan prototype into an operator-grade
//! checklist for a real Windows desktop with OBS Studio, the OpenIRL agent, and
//! the local dashboard running together.

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// Alpha validation stage.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AlphaStage {
    /// Rust workspace validation.
    RustToolchain,
    /// Static repository validation.
    StaticRepository,
    /// Windows portable artifact validation.
    WindowsPortable,
    /// Agent process validation.
    AgentRuntime,
    /// OBS WebSocket validation.
    ObsWebSocket,
    /// OBS scene/action validation.
    ObsAutomation,
    /// Encoder profile validation.
    EncoderProfiles,
    /// Relay and metrics validation.
    RelayMetrics,
    /// Packaging validation.
    Packaging,
    /// Evidence capture validation.
    Evidence,
}

/// Alpha validation status.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AlphaStatus {
    /// The check is expected but not attempted by the agent.
    NotRun,
    /// The check is blocked.
    Blocked,
    /// The check requires an operator on Windows or OBS.
    NeedsOperator,
    /// The check passed.
    Passed,
    /// The check failed.
    Failed,
}

/// One validation check.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct AlphaValidationCheck {
    /// Stable check identifier.
    pub id: String,
    /// Stage.
    pub stage: AlphaStage,
    /// Human-readable label.
    pub label: String,
    /// Command or operator action.
    pub action: String,
    /// Expected result.
    pub expected: String,
    /// Whether the check blocks a public alpha handoff.
    pub blocking: bool,
    /// Current default status.
    pub status: AlphaStatus,
}

/// Evidence item that should be captured during alpha validation.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct AlphaEvidenceItem {
    /// Evidence identifier.
    pub id: String,
    /// Description.
    pub description: String,
    /// Suggested file or endpoint.
    pub source: String,
    /// Whether the evidence is required for alpha signoff.
    pub required: bool,
}

/// Full desktop alpha validation plan.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct AlphaValidationPlan {
    /// Plan generation time.
    pub generated_at: OffsetDateTime,
    /// Schema revision.
    pub schema_revision: u16,
    /// Plan title.
    pub title: String,
    /// Validation checks.
    pub checks: Vec<AlphaValidationCheck>,
    /// Evidence requirements.
    pub evidence: Vec<AlphaEvidenceItem>,
    /// Operator notes.
    pub notes: Vec<String>,
}

/// Operator checklist step.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct OperatorChecklistStep {
    /// Step number.
    pub number: u8,
    /// Title.
    pub title: String,
    /// Instructions.
    pub instructions: Vec<String>,
    /// Pass criteria.
    pub pass_criteria: Vec<String>,
}

/// Evidence input reported by scripts or a human operator.
#[derive(Debug, Clone, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct AlphaEvidenceInput {
    /// Static validation succeeded.
    pub static_validation_passed: bool,
    /// Cargo fmt/clippy/test succeeded.
    pub rust_ci_passed: bool,
    /// Default config validation has no blocking errors.
    pub config_ok: bool,
    /// Agent health endpoint returned ok.
    pub agent_health_ok: bool,
    /// Dashboard loaded locally.
    pub dashboard_loaded: bool,
    /// OBS WebSocket connected and authenticated.
    pub obs_connected: bool,
    /// OBS scene switching was verified against a real scene collection.
    pub obs_scene_switch_verified: bool,
    /// Start/stop streaming was verified against OBS.
    pub obs_streaming_start_stop_verified: bool,
    /// Replay buffer save was verified.
    pub replay_save_verified: bool,
    /// Recording start/stop was verified.
    pub recording_controls_verified: bool,
    /// Moblin or IRL Pro QR/profile was tested on a real device.
    pub profile_qr_tested: bool,
    /// MediaMTX or relay metrics polling produced a health sample.
    pub metrics_poll_verified: bool,
    /// Windows portable artifact was built.
    pub windows_portable_built: bool,
    /// MSI packaging template was tested or intentionally deferred.
    pub windows_msi_reviewed: bool,
}

/// Evidence report.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct AlphaEvidenceReport {
    /// Report generation time.
    pub generated_at: OffsetDateTime,
    /// Overall readiness.
    pub ready_for_private_alpha: bool,
    /// Completed checks.
    pub completed: Vec<String>,
    /// Blocking missing checks.
    pub blockers: Vec<String>,
    /// Non-blocking warnings.
    pub warnings: Vec<String>,
    /// Next operator actions.
    pub next_actions: Vec<String>,
}

/// Builds the desktop alpha validation plan.
#[must_use]
pub fn build_alpha_validation_plan(schema_revision: u16) -> AlphaValidationPlan {
    AlphaValidationPlan {
        generated_at: OffsetDateTime::now_utc(),
        schema_revision,
        title: "desktop alpha real Windows + OBS alpha validation".to_string(),
        checks: vec![
            check("static-validate", AlphaStage::StaticRepository, "Run static validation", "python3 scripts/static_validate.py", "static validation passed", true, AlphaStatus::NotRun),
            check("cargo-ci", AlphaStage::RustToolchain, "Run Rust workspace CI", "cargo xtask ci", "cargo fmt, clippy, and tests pass", true, AlphaStatus::NeedsOperator),
            check("portable-build", AlphaStage::WindowsPortable, "Build portable Windows alpha", "scripts\\windows\\build-alpha-portable.ps1", "openirl-windows-portable-alpha.zip is generated", true, AlphaStatus::NeedsOperator),
            check("agent-health", AlphaStage::AgentRuntime, "Start agent and verify health", "openirl-agent.exe serve --config config\\openirl.example.toml", "GET /health returns schema revision", true, AlphaStatus::NeedsOperator),
            check("dashboard-load", AlphaStage::AgentRuntime, "Open dashboard", "Open http://127.0.0.1:7707/", "dashboard renders readiness, OBS, metrics, and alpha cards", true, AlphaStatus::NeedsOperator),
            check("obs-connect", AlphaStage::ObsWebSocket, "Connect to real OBS WebSocket", "scripts\\smoke\\obs-websocket-smoke.ps1 -Action Status", "OBS authenticates and returns status", true, AlphaStatus::NeedsOperator),
            check("obs-scenes", AlphaStage::ObsAutomation, "Switch required OBS scenes", "scripts\\smoke\\obs-websocket-smoke.ps1 -Action Scenes", "Live, BRB, Low Signal, Backup Feed, and Privacy scene switches succeed", true, AlphaStatus::NeedsOperator),
            check("obs-stream-controls", AlphaStage::ObsAutomation, "Verify stream controls", "scripts\\smoke\\obs-websocket-smoke.ps1 -Action StreamControls -DryRun:$false", "StartStream and StopStream return OBS success in a safe test profile", false, AlphaStatus::NeedsOperator),
            check("replay-recording", AlphaStage::ObsAutomation, "Verify replay and recording controls", "scripts\\smoke\\obs-websocket-smoke.ps1 -Action Production", "SaveReplayBuffer, StartRecord, and StopRecord are verified or blocked with clear OBS reason", false, AlphaStatus::NeedsOperator),
            check("profile-qr", AlphaStage::EncoderProfiles, "Test real encoder QR profile", "openirl-agent.exe onboarding --encoder moblin --protocol srt --mode local-direct", "Moblin or IRL Pro accepts generated profile", true, AlphaStatus::NeedsOperator),
            check("metrics-poll", AlphaStage::RelayMetrics, "Poll relay/router metrics", "POST /api/metrics/poll or /api/metrics/simulate/healthy", "metrics sample reaches health engine", true, AlphaStatus::NeedsOperator),
            check("evidence-bundle", AlphaStage::Evidence, "Capture support bundle and alpha evidence", "scripts\\smoke\\alpha-evidence.ps1", "evidence JSON and support bundle are saved under artifacts\\alpha", true, AlphaStatus::NeedsOperator),
        ],
        evidence: vec![
            evidence("health-json", "Agent /health response", "artifacts/alpha/health.json", true),
            evidence("readiness-json", "Runtime readiness response", "artifacts/alpha/readiness.json", true),
            evidence("obs-smoke-json", "OBS WebSocket smoke results", "artifacts/alpha/obs-smoke.json", true),
            evidence("support-bundle-json", "OpenIRL support bundle", "artifacts/alpha/support-bundle.json", true),
            evidence("profile-qr-svg", "Generated encoder QR SVG", "artifacts/alpha/profile.svg", true),
            evidence("metrics-json", "Metrics and health sample", "artifacts/alpha/metrics.json", true),
            evidence("portable-sha256", "Portable Windows artifact checksum", "artifacts/alpha/openirl-windows-portable-alpha.zip.sha256", true),
            evidence("msi-log", "WiX/MSI build log if attempted", "artifacts/alpha/msi-build.log", false),
        ],
        notes: vec![
            "Run destructive OBS stream-start tests only against a private test channel/profile.".to_string(),
            "Keep OBS WebSocket bound to localhost unless the operator intentionally uses a VPN or trusted LAN.".to_string(),
            "Do not publish alpha binaries until the support bundle redaction path has been reviewed.".to_string(),
        ],
    }
}

/// Builds the Windows operator checklist.
#[must_use]
pub fn build_operator_checklist() -> Vec<OperatorChecklistStep> {
    vec![
        step(
            1,
            "Prepare Windows host",
            &[
                "Install OBS Studio 28 or newer.",
                "Install the Rust toolchain.",
                "Clone or extract the OpenIRL repo.",
            ],
            &[
                "obs64.exe starts.",
                "cargo --version succeeds.",
                "repo files are writable.",
            ],
        ),
        step(
            2,
            "Configure OBS WebSocket",
            &[
                "Open OBS Settings > WebSocket Server Settings.",
                "Enable the WebSocket server on 127.0.0.1:4455.",
                "Set a strong password and export it as OPENIRL_OBS_PASSWORD.",
            ],
            &[
                "OBS accepts WebSocket authentication.",
                "OBS is not exposed directly to the public internet.",
            ],
        ),
        step(
            3,
            "Run local validation",
            &[
                "Run python3 scripts/static_validate.py.",
                "Run cargo xtask ci.",
            ],
            &["Both commands pass without denied patterns or test failures."],
        ),
        step(
            4,
            "Start OpenIRL agent",
            &[
                "Run openirl-agent.exe serve --config config\\openirl.example.toml --obs-adapter web-socket.",
                "Open http://127.0.0.1:7707/.",
            ],
            &[
                "/health reports schema revision.",
                "Dashboard loads locally.",
            ],
        ),
        step(
            5,
            "Run OBS smoke scripts",
            &[
                "Run scripts\\smoke\\obs-websocket-smoke.ps1 -Action Status.",
                "Run scripts\\smoke\\obs-websocket-smoke.ps1 -Action Scenes.",
                "Run production controls only on a safe test profile.",
            ],
            &[
                "OBS status is returned.",
                "Scene switching succeeds.",
                "Any OBS-side block is recorded as evidence.",
            ],
        ),
        step(
            6,
            "Validate encoder onboarding",
            &[
                "Generate Moblin or IRL Pro QR/profile.",
                "Scan with a real device or document why hardware is unavailable.",
            ],
            &[
                "Device accepts the generated contribution URL.",
                "Profile evidence is saved.",
            ],
        ),
        step(
            7,
            "Capture alpha evidence",
            &[
                "Run scripts\\smoke\\alpha-evidence.ps1.",
                "Review artifacts\\alpha before sharing.",
            ],
            &[
                "Evidence JSON files exist.",
                "Secrets and public IPs are redacted where expected.",
            ],
        ),
    ]
}

/// Evaluates operator evidence.
#[must_use]
pub fn evaluate_alpha_evidence(input: &AlphaEvidenceInput) -> AlphaEvidenceReport {
    let mut completed = Vec::new();
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();
    let mut next_actions = Vec::new();

    push_bool(
        input.static_validation_passed,
        "static validation",
        true,
        &mut completed,
        &mut blockers,
        &mut next_actions,
        "Run python3 scripts/static_validate.py",
    );
    push_bool(
        input.rust_ci_passed,
        "Rust CI",
        true,
        &mut completed,
        &mut blockers,
        &mut next_actions,
        "Run cargo xtask ci on a Rust toolchain machine",
    );
    push_bool(
        input.config_ok,
        "config validation",
        true,
        &mut completed,
        &mut blockers,
        &mut next_actions,
        "Fix /api/config/validation blocking issues",
    );
    push_bool(
        input.agent_health_ok,
        "agent health",
        true,
        &mut completed,
        &mut blockers,
        &mut next_actions,
        "Start agent and save /health response",
    );
    push_bool(
        input.dashboard_loaded,
        "dashboard load",
        true,
        &mut completed,
        &mut blockers,
        &mut next_actions,
        "Open http://127.0.0.1:7707/ on Windows host",
    );
    push_bool(
        input.obs_connected,
        "OBS WebSocket connection",
        true,
        &mut completed,
        &mut blockers,
        &mut next_actions,
        "Enable OBS WebSocket and rerun smoke script",
    );
    push_bool(
        input.obs_scene_switch_verified,
        "OBS scene switching",
        true,
        &mut completed,
        &mut blockers,
        &mut next_actions,
        "Run scene smoke test against real OBS",
    );
    push_bool(
        input.profile_qr_tested,
        "real encoder QR/profile",
        true,
        &mut completed,
        &mut blockers,
        &mut next_actions,
        "Test Moblin or IRL Pro QR on device",
    );
    push_bool(
        input.metrics_poll_verified,
        "metrics poll",
        true,
        &mut completed,
        &mut blockers,
        &mut next_actions,
        "Poll MediaMTX metrics or record simulation fallback",
    );
    push_bool(
        input.windows_portable_built,
        "Windows portable artifact",
        true,
        &mut completed,
        &mut blockers,
        &mut next_actions,
        "Run scripts/windows/build-alpha-portable.ps1",
    );

    push_bool(
        input.obs_streaming_start_stop_verified,
        "OBS stream start/stop",
        false,
        &mut completed,
        &mut warnings,
        &mut next_actions,
        "Verify StartStream/StopStream on private test channel",
    );
    push_bool(
        input.replay_save_verified,
        "replay buffer save",
        false,
        &mut completed,
        &mut warnings,
        &mut next_actions,
        "Enable replay buffer and verify save action",
    );
    push_bool(
        input.recording_controls_verified,
        "recording controls",
        false,
        &mut completed,
        &mut warnings,
        &mut next_actions,
        "Verify recording start/stop on local disk",
    );
    push_bool(
        input.windows_msi_reviewed,
        "Windows MSI template review",
        false,
        &mut completed,
        &mut warnings,
        &mut next_actions,
        "Review WiX template or explicitly defer MSI for portable alpha",
    );

    AlphaEvidenceReport {
        generated_at: OffsetDateTime::now_utc(),
        ready_for_private_alpha: blockers.is_empty(),
        completed,
        blockers,
        warnings,
        next_actions,
    }
}

fn check(
    id: &str,
    stage: AlphaStage,
    label: &str,
    action: &str,
    expected: &str,
    blocking: bool,
    status: AlphaStatus,
) -> AlphaValidationCheck {
    AlphaValidationCheck {
        id: id.to_string(),
        stage,
        label: label.to_string(),
        action: action.to_string(),
        expected: expected.to_string(),
        blocking,
        status,
    }
}

fn evidence(id: &str, description: &str, source: &str, required: bool) -> AlphaEvidenceItem {
    AlphaEvidenceItem {
        id: id.to_string(),
        description: description.to_string(),
        source: source.to_string(),
        required,
    }
}

fn step(
    number: u8,
    title: &str,
    instructions: &[&str],
    pass_criteria: &[&str],
) -> OperatorChecklistStep {
    OperatorChecklistStep {
        number,
        title: title.to_string(),
        instructions: instructions
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        pass_criteria: pass_criteria
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
    }
}

fn push_bool(
    passed: bool,
    label: &str,
    blocking: bool,
    completed: &mut Vec<String>,
    missing: &mut Vec<String>,
    next_actions: &mut Vec<String>,
    remediation: &str,
) {
    if passed {
        completed.push(label.to_string());
    } else {
        let prefix = if blocking {
            "missing required"
        } else {
            "not yet verified"
        };
        missing.push(format!("{prefix}: {label}"));
        next_actions.push(remediation.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_targets_desktop_alpha() {
        let plan = build_alpha_validation_plan(38);
        assert_eq!(plan.schema_revision, 38);
        assert!(plan.checks.iter().any(|check| check.id == "obs-connect"));
    }

    #[test]
    fn evidence_blocks_without_obs() {
        let input = AlphaEvidenceInput {
            static_validation_passed: true,
            rust_ci_passed: true,
            config_ok: true,
            agent_health_ok: true,
            dashboard_loaded: true,
            ..AlphaEvidenceInput::default()
        };
        let report = evaluate_alpha_evidence(&input);
        assert!(!report.ready_for_private_alpha);
        assert!(
            report
                .blockers
                .iter()
                .any(|item| item.contains("OBS WebSocket"))
        );
    }
}
