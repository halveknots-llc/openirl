//! Real mobile field-stream validation planning and evidence scoring.
//!
//! mobile field alpha moves OpenIRL beyond desktop/OBS readiness and into real encoder
//! field validation. This crate is deliberately pure Rust: it models the
//! operator plan, device checklists, and pass/fail evidence without depending on
//! a live phone, backpack, MediaMTX process, or OBS instance.

use openirl_core::{EncoderKind, HealthState, Protocol, SceneRole};
use openirl_readiness::EvidenceMaturity;
use openirl_vault::redact_support_text;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::Path};
use time::{Date, Month, OffsetDateTime};

/// Field-validation stage.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FieldStage {
    /// Repository/toolchain readiness before mobile testing.
    Toolchain,
    /// Encoder profile generation and QR/device acceptance.
    DeviceProfile,
    /// Physical encoder publishing into OpenIRL or MediaMTX.
    MobileEncoder,
    /// MediaMTX or relay path visibility.
    LocalIngest,
    /// OBS source/scene behavior.
    ObsRouting,
    /// Brownout and recovery behavior.
    BrownoutRecovery,
    /// Diagnostics/support-bundle capture.
    Diagnostics,
    /// Evidence packaging for alpha handoff.
    Evidence,
}

/// Device family used in mobile field alpha field tests.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FieldDevice {
    /// Moblin on iOS.
    Moblin,
    /// IRL Pro on Android.
    IrlPro,
    /// BELABOX hardware/software backpack encoder.
    Belabox,
    /// Larix Broadcaster compatibility pass.
    Larix,
}

impl FieldDevice {
    /// Maps field devices to OpenIRL encoder kinds.
    #[must_use]
    pub fn encoder(self) -> EncoderKind {
        match self {
            Self::Moblin => EncoderKind::Moblin,
            Self::IrlPro => EncoderKind::IrlPro,
            Self::Belabox => EncoderKind::Belabox,
            Self::Larix => EncoderKind::Larix,
        }
    }
}

/// Validation status used by checklists.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FieldStatus {
    /// Not attempted yet.
    NotRun,
    /// Requires a human operator with real hardware.
    NeedsOperator,
    /// Passed.
    Passed,
    /// Blocked by missing hardware/software/network.
    Blocked,
    /// Failed.
    Failed,
}

/// Result recorded at the declared evidence maturity.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CompatibilityResult {
    /// The named environment or dependency has not run.
    NotRun,
    /// The declared evidence maturity passed.
    Passed,
    /// The declared evidence maturity failed.
    Failed,
    /// A named prerequisite prevented the check from running.
    Blocked,
}

/// One versioned compatibility evidence row.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompatibilityRow {
    /// Stable row identifier.
    pub id: String,
    /// Operator workflow family.
    pub workflow: String,
    /// Named dependency, device, or platform.
    pub dependency: String,
    /// Exact dependency version, or `not-recorded` before real integration evidence exists.
    pub dependency_version: String,
    /// Host platform and version, or `not-recorded` before real integration evidence exists.
    pub host_platform: String,
    /// Exact OpenIRL commit used for this evidence.
    pub openirl_revision: String,
    /// Non-secret configuration class.
    pub configuration_class: String,
    /// Reproducible smoke command or script.
    pub smoke_script: String,
    /// Highest established maturity.
    pub maturity: EvidenceMaturity,
    /// Result at that maturity.
    pub result: CompatibilityResult,
    /// Public-safe artifact, test, or report reference.
    pub artifact_reference: String,
    /// Concise interpretation boundary.
    pub notes: String,
}

/// Versioned compatibility matrix.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompatibilityMatrix {
    /// Matrix schema revision.
    pub matrix_revision: u16,
    /// OpenIRL API schema revision.
    pub schema_revision: u16,
    /// Exact source revision shared by every row.
    pub source_revision: String,
    /// Calendar date of the evidence review.
    pub reviewed_on: String,
    /// Compatibility evidence rows.
    pub rows: Vec<CompatibilityRow>,
    /// Public evidence handling rules.
    pub evidence_policy: Vec<String>,
}

/// Semantic validation result for a compatibility matrix.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompatibilityValidationReport {
    /// True when no validation error was found.
    pub ok: bool,
    /// Public-safe validation errors.
    pub errors: Vec<String>,
}

/// One mobile field alpha validation check.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct FieldValidationCheck {
    /// Stable check identifier.
    pub id: String,
    /// Stage.
    pub stage: FieldStage,
    /// Optional device scope.
    pub device: Option<FieldDevice>,
    /// Human-readable label.
    pub label: String,
    /// Operator action or command.
    pub action: String,
    /// Expected evidence/result.
    pub expected: String,
    /// Whether the check blocks private field alpha signoff.
    pub blocking: bool,
    /// Default status.
    pub status: FieldStatus,
}

/// Device-specific checklist shown to operators.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct FieldDeviceChecklist {
    /// Device family.
    pub device: FieldDevice,
    /// OpenIRL encoder kind.
    pub encoder: EncoderKind,
    /// Preferred contribution protocol.
    pub preferred_protocol: Protocol,
    /// Acceptable fallback protocols.
    pub acceptable_protocols: Vec<Protocol>,
    /// Setup steps.
    pub setup_steps: Vec<String>,
    /// Pass criteria.
    pub pass_criteria: Vec<String>,
    /// Failure notes to capture.
    pub failure_notes: Vec<String>,
}

/// Evidence item that should be saved for a mobile field alpha field run.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct FieldEvidenceItem {
    /// Evidence identifier.
    pub id: String,
    /// Description.
    pub description: String,
    /// Suggested source file/endpoint.
    pub source: String,
    /// Whether this evidence blocks signoff.
    pub required: bool,
}

/// Full mobile field alpha validation plan.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct FieldValidationPlan {
    /// Plan generation timestamp.
    pub generated_at: OffsetDateTime,
    /// Schema revision.
    pub schema_revision: u16,
    /// Plan title.
    pub title: String,
    /// Device checklists.
    pub device_checklists: Vec<FieldDeviceChecklist>,
    /// Validation checks.
    pub checks: Vec<FieldValidationCheck>,
    /// Required and optional evidence.
    pub evidence: Vec<FieldEvidenceItem>,
    /// Operator notes.
    pub notes: Vec<String>,
}

/// Evidence input posted by scripts/operators after a field run.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct FieldEvidenceInput {
    /// Static repository validation passed.
    pub static_validation_passed: bool,
    /// Cargo CI passed.
    pub rust_ci_passed: bool,
    /// desktop alpha Windows/OBS alpha readiness was acceptable.
    pub windows_alpha_ready: bool,
    /// Moblin profile was generated by OpenIRL.
    pub moblin_profile_generated: bool,
    /// Moblin QR/profile was accepted by the device.
    pub moblin_qr_scanned: bool,
    /// Moblin ingest was observed by relay or OBS.
    pub moblin_ingest_seen: bool,
    /// IRL Pro profile was generated by OpenIRL.
    pub irlpro_profile_generated: bool,
    /// IRL Pro QR/profile was accepted by the device.
    pub irlpro_qr_scanned: bool,
    /// IRL Pro ingest was observed by relay or OBS.
    pub irlpro_ingest_seen: bool,
    /// BELABOX profile or relay settings were generated.
    pub belabox_profile_generated: bool,
    /// BELABOX configuration was reviewed on device/UI.
    pub belabox_config_reviewed: bool,
    /// BELABOX ingest was observed by relay or OBS.
    pub belabox_ingest_seen: bool,
    /// MediaMTX SRT path became active during test.
    pub mediamtx_srt_path_active: bool,
    /// MediaMTX or relay metrics were collected.
    pub mediamtx_metrics_seen: bool,
    /// OBS WebSocket connection was active.
    pub obs_connected: bool,
    /// OBS source/scene showed the mobile contribution.
    pub obs_media_source_seen: bool,
    /// Health engine observed healthy state.
    pub healthy_state_seen: bool,
    /// Health engine observed brownout state.
    pub brownout_state_seen: bool,
    /// BRB/fallback scene was observed.
    pub brb_scene_seen: bool,
    /// Recovery back toward live was observed.
    pub recovery_state_seen: bool,
    /// Support bundle was captured.
    pub support_bundle_captured: bool,
    /// Evidence was redacted for secrets/IPs before sharing.
    pub secrets_redacted: bool,
    /// Human-readable field report was written.
    pub field_report_written: bool,
}

/// Evaluated mobile field alpha field evidence.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct FieldEvidenceReport {
    /// Report generation timestamp.
    pub generated_at: OffsetDateTime,
    /// Whether the package is ready for a private field alpha.
    pub ready_for_private_field_alpha: bool,
    /// Integer score out of 100.
    pub score: u8,
    /// Passed blocking checks.
    pub required_passed: u16,
    /// Total blocking checks.
    pub required_total: u16,
    /// Blocking issues.
    pub blockers: Vec<String>,
    /// Non-blocking warnings.
    pub warnings: Vec<String>,
    /// Passed evidence labels.
    pub passed: Vec<String>,
    /// Failed evidence labels.
    pub failed: Vec<String>,
    /// Suggested next actions.
    pub next_actions: Vec<String>,
    /// Concise status summary.
    pub summary: String,
}

/// Builds the versioned source and field compatibility baseline.
#[must_use]
pub fn build_compatibility_matrix(
    schema_revision: u16,
    source_revision: impl Into<String>,
    reviewed_on: impl Into<String>,
) -> CompatibilityMatrix {
    let source_revision = source_revision.into();
    let reviewed_on = reviewed_on.into();
    macro_rules! row {
        ($id:expr, $workflow:expr, $dependency:expr, $dependency_version:expr,
         $host:expr, $class:expr, $smoke:expr, $maturity:expr, $result:expr,
         $artifact:expr, $notes:expr) => {
            CompatibilityRow {
                id: $id.to_string(),
                workflow: $workflow.to_string(),
                dependency: $dependency.to_string(),
                dependency_version: $dependency_version.to_string(),
                host_platform: $host.to_string(),
                openirl_revision: source_revision.clone(),
                configuration_class: $class.to_string(),
                smoke_script: $smoke.to_string(),
                maturity: $maturity,
                result: $result,
                artifact_reference: $artifact.to_string(),
                notes: $notes.to_string(),
            }
        };
    }

    CompatibilityMatrix {
        matrix_revision: 1,
        schema_revision,
        source_revision: source_revision.clone(),
        reviewed_on,
        rows: vec![
            row!(
                "obs-websocket-v5",
                "OBS scene and output control",
                "OBS Studio",
                "not-recorded",
                "not-recorded",
                "obs-websocket-v5-source-contract",
                "scripts/obs/reconcile-smoke.sh or .ps1",
                EvidenceMaturity::SourceValidated,
                CompatibilityResult::Passed,
                "crates/openirl-obs/src/lib.rs",
                "Typed WebSocket requests and source tests passed; no real OBS version is claimed."
            ),
            row!(
                "mediamtx-srt-ingest",
                "SRT contribution through MediaMTX",
                "MediaMTX",
                "not-recorded",
                "not-recorded",
                "loopback-srt-listener-source-contract",
                "scripts/ingest/local-ingest-smoke.sh or .ps1",
                EvidenceMaturity::SourceValidated,
                CompatibilityResult::Passed,
                "deploy/mediamtx/openirl.mediamtx.yml",
                "Loopback configuration and metrics parsing passed source checks; no media process ran."
            ),
            row!(
                "mediamtx-rtmp-ingest",
                "RTMP contribution through MediaMTX",
                "MediaMTX",
                "not-recorded",
                "not-recorded",
                "loopback-rtmp-listener-plan",
                "scripts/ingest/local-ingest-smoke.sh or .ps1",
                EvidenceMaturity::Modeled,
                CompatibilityResult::NotRun,
                "not-run",
                "The RTMP path is configured but has no source or live evidence row yet."
            ),
            row!(
                "moblin-profile",
                "Moblin contribution profile",
                "Moblin",
                "not-recorded",
                "not-recorded",
                "moblin-srt-srtla-profile-contract",
                "scripts/mobile/profile-compat-smoke.sh or .ps1",
                EvidenceMaturity::SourceValidated,
                CompatibilityResult::Passed,
                "presets/encoders/moblin-srt.json",
                "Profile generation and redaction passed; no iOS import or stream is claimed."
            ),
            row!(
                "irl-pro-profile",
                "IRL Pro contribution profile",
                "IRL Pro",
                "not-recorded",
                "not-recorded",
                "irl-pro-srt-srtla-profile-contract",
                "scripts/mobile/profile-compat-smoke.sh or .ps1",
                EvidenceMaturity::SourceValidated,
                CompatibilityResult::Passed,
                "presets/encoders/irl-pro-srt.json",
                "Profile generation and redaction passed; no Android import or stream is claimed."
            ),
            row!(
                "larix-profile",
                "Larix contribution profile",
                "Larix Broadcaster",
                "not-recorded",
                "not-recorded",
                "larix-srt-profile-contract",
                "scripts/mobile/profile-compat-smoke.sh or .ps1",
                EvidenceMaturity::SourceValidated,
                CompatibilityResult::Passed,
                "presets/encoders/larix-srt.json",
                "Profile generation passed; no device import or contribution is claimed."
            ),
            row!(
                "belabox-profile",
                "BELABOX-oriented bonded contribution profile",
                "BELABOX",
                "not-recorded",
                "not-recorded",
                "belabox-srtla2-profile-contract",
                "scripts/mobile/profile-compat-smoke.sh or .ps1",
                EvidenceMaturity::SourceValidated,
                CompatibilityResult::Passed,
                "presets/encoders/belabox-srtla2.json",
                "Profile generation passed; no BELABOX hardware or service was contacted."
            ),
            row!(
                "srtla-process-path",
                "Process-bound SRTLA relay path",
                "SRTLA-compatible tools",
                "not-recorded",
                "not-recorded",
                "process-supervision-source-contract",
                "scripts/relay/srtla2-compat-smoke.sh",
                EvidenceMaturity::SourceValidated,
                CompatibilityResult::Passed,
                "crates/openirl-relay-control/src/lib.rs",
                "Bounded process planning and redaction passed; no SRTLA binary ran."
            ),
            row!(
                "brownout-health",
                "Brownout detection and fallback recommendation",
                "OpenIRL health engine",
                "0.1.0-alpha.0",
                "deterministic-local-runtime",
                "fixed-health-scenario-v1",
                "python3 scripts/smoke/demo-mode-smoke.py",
                EvidenceMaturity::LocalRuntimeValidated,
                CompatibilityResult::Passed,
                "fixtures/metrics/brownout-v2-scenarios.json",
                "Deterministic local metrics exercised health decisions without contribution media."
            ),
            row!(
                "backup-ingest",
                "Primary-to-backup ingest selection",
                "OBS and MediaMTX",
                "not-recorded",
                "not-recorded",
                "backup-ingest-source-contract",
                "scripts/ingest/backup-failover-smoke.sh",
                EvidenceMaturity::SourceValidated,
                CompatibilityResult::Passed,
                "presets/obs/backup-ingest-policy.json",
                "Policy and scene-selection source checks passed; no dual ingest environment ran."
            ),
            row!(
                "recovery-hysteresis",
                "Brownout recovery and scene stabilization",
                "OpenIRL health engine",
                "0.1.0-alpha.0",
                "deterministic-local-runtime",
                "fixed-health-scenario-v1",
                "cargo test --package openirl-health",
                EvidenceMaturity::LocalRuntimeValidated,
                CompatibilityResult::Passed,
                "crates/openirl-health/src/lib.rs",
                "Recovery hysteresis passed deterministic tests; no real network recovery is claimed."
            ),
            row!(
                "windows-portable-alpha",
                "Windows portable package and first launch",
                "Windows",
                "not-recorded",
                "not-recorded",
                "windows-portable-release",
                "scripts/windows/build-alpha-portable.ps1",
                EvidenceMaturity::Modeled,
                CompatibilityResult::NotRun,
                "not-run",
                "A real Windows runner must build and verify the package before this row advances."
            ),
        ],
        evidence_policy: vec![
            "Record exact host, dependency, and OpenIRL versions for integration or field evidence."
                .to_string(),
            "Use only sanitized fixtures and reviewed artifact references in the public matrix."
                .to_string(),
            "Never attach stream credentials, private network details, raw support bundles, or location-sensitive media."
                .to_string(),
            "Source and deterministic local results never imply integration, field, or release maturity."
                .to_string(),
        ],
    }
}

/// Validates matrix structure, evidence maturity, and public-safe references.
#[must_use]
pub fn validate_compatibility_matrix(
    matrix: &CompatibilityMatrix,
) -> CompatibilityValidationReport {
    let mut errors = Vec::new();
    if matrix.matrix_revision == 0 {
        errors.push("matrix_revision must be greater than zero".to_string());
    }
    if !is_git_revision(&matrix.source_revision) {
        errors.push("source_revision must be a full 40-character Git commit".to_string());
    }
    if !looks_like_date(&matrix.reviewed_on) {
        errors.push("reviewed_on must use YYYY-MM-DD format".to_string());
    }
    if matrix.rows.is_empty() {
        errors.push("matrix must contain at least one compatibility row".to_string());
    }
    if matrix.evidence_policy.is_empty() {
        errors.push("evidence_policy must contain public handling rules".to_string());
    }
    for policy in &matrix.evidence_policy {
        if policy.trim().is_empty() {
            errors.push("evidence_policy contains an empty rule".to_string());
        }
        if redact_support_text(policy, true) != policy.as_str() {
            errors.push("evidence_policy contains sensitive public evidence".to_string());
        }
    }

    let mut ids = BTreeSet::new();
    for row in &matrix.rows {
        if !ids.insert(row.id.as_str()) {
            errors.push(format!("duplicate compatibility row id: {}", row.id));
        }
        for (field, value) in [
            ("id", row.id.as_str()),
            ("workflow", row.workflow.as_str()),
            ("dependency", row.dependency.as_str()),
            ("dependency_version", row.dependency_version.as_str()),
            ("host_platform", row.host_platform.as_str()),
            ("configuration_class", row.configuration_class.as_str()),
            ("smoke_script", row.smoke_script.as_str()),
            ("artifact_reference", row.artifact_reference.as_str()),
            ("notes", row.notes.as_str()),
        ] {
            if value.trim().is_empty() {
                errors.push(format!("row {} has an empty {field}", row.id));
            }
        }
        if row.openirl_revision != matrix.source_revision {
            errors.push(format!(
                "row {} does not match the matrix source revision",
                row.id
            ));
        }
        let high_maturity = matches!(
            row.maturity,
            EvidenceMaturity::IntegrationValidated
                | EvidenceMaturity::FieldValidated
                | EvidenceMaturity::Released
        );
        if high_maturity
            && (row.dependency_version == "not-recorded"
                || row.host_platform == "not-recorded"
                || row.artifact_reference == "not-run")
        {
            errors.push(format!(
                "row {} lacks concrete version, host, or artifact evidence",
                row.id
            ));
        }
        if row.result == CompatibilityResult::NotRun && row.maturity != EvidenceMaturity::Modeled {
            errors.push(format!(
                "row {} cannot claim evidence maturity for a check that did not run",
                row.id
            ));
        }
        if row.result == CompatibilityResult::Passed && row.maturity == EvidenceMaturity::Modeled {
            errors.push(format!("row {} cannot pass at modeled maturity", row.id));
        }
        if Path::new(&row.artifact_reference).is_absolute()
            || row.artifact_reference.split('/').any(|part| part == "..")
        {
            errors.push(format!(
                "row {} artifact_reference must be public and repository-relative",
                row.id
            ));
        }
        let public_text = format!(
            "{} {} {} {} {} {} {} {}",
            row.workflow,
            row.dependency,
            row.dependency_version,
            row.host_platform,
            row.configuration_class,
            row.smoke_script,
            row.artifact_reference,
            row.notes
        );
        if redact_support_text(&public_text, true) != public_text {
            errors.push(format!("row {} contains sensitive public evidence", row.id));
        }
    }

    CompatibilityValidationReport {
        ok: errors.is_empty(),
        errors,
    }
}

fn is_git_revision(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn looks_like_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes
            .iter()
            .enumerate()
            .any(|(index, byte)| !matches!(index, 4 | 7) && !byte.is_ascii_digit())
    {
        return false;
    }
    let year = i32::from(bytes[0] - b'0') * 1_000
        + i32::from(bytes[1] - b'0') * 100
        + i32::from(bytes[2] - b'0') * 10
        + i32::from(bytes[3] - b'0');
    let month = (bytes[5] - b'0') * 10 + (bytes[6] - b'0');
    let day = (bytes[8] - b'0') * 10 + (bytes[9] - b'0');
    let Ok(month) = Month::try_from(month) else {
        return false;
    };
    Date::from_calendar_date(year, month, day).is_ok()
}

/// Builds the mobile field alpha field-validation plan.
#[must_use]
pub fn build_field_validation_plan(schema_revision: u16) -> FieldValidationPlan {
    FieldValidationPlan {
        generated_at: OffsetDateTime::now_utc(),
        schema_revision,
        title: "Real Mobile Field-Stream Validation".to_string(),
        device_checklists: build_device_checklists(),
        checks: vec![
            check("static-validation", FieldStage::Toolchain, None, "Static validation", "Run python3 scripts/static_validate.py before device testing.", "Static validation passes with mobile field alpha markers.", true),
            check("cargo-ci", FieldStage::Toolchain, None, "Rust CI", "Run cargo xtask ci on a Rust workstation.", "fmt, clippy, and tests pass.", true),
            check("moblin-profile", FieldStage::DeviceProfile, Some(FieldDevice::Moblin), "Moblin profile", "Generate Moblin SRTLA/SRT QR from OpenIRL.", "The iOS device accepts the generated contribution URL.", true),
            check("irlpro-profile", FieldStage::DeviceProfile, Some(FieldDevice::IrlPro), "IRL Pro profile", "Generate IRL Pro SRTLA/SRT QR from OpenIRL.", "The Android device accepts the generated contribution URL.", true),
            check("belabox-profile", FieldStage::DeviceProfile, Some(FieldDevice::Belabox), "BELABOX profile", "Generate BELABOX relay settings and review passphrase/stream ID.", "The BELABOX UI/config contains the expected endpoint.", false),
            check("mediamtx-path", FieldStage::LocalIngest, None, "MediaMTX SRT path", "Publish a mobile encoder into the local MediaMTX/OpenIRL path.", "A path becomes active and metrics are visible.", true),
            check("obs-source", FieldStage::ObsRouting, None, "OBS media source", "Verify OBS receives the contribution source and can switch scenes.", "Live and fallback scene roles are observable.", true),
            check("brownout", FieldStage::BrownoutRecovery, None, "Brownout transition", "Simulate or create uplink degradation with the real device connected.", "Health engine records brownout and BRB/fallback behavior.", true),
            check("recovery", FieldStage::BrownoutRecovery, None, "Recovery transition", "Restore stable network after brownout.", "Health engine records recovery and return-to-live readiness.", true),
            check("support-bundle", FieldStage::Diagnostics, None, "Support bundle", "Capture /api/session/support-bundle after the field run.", "Bundle includes metrics, OBS, relay, field, and redacted config context.", true),
            check("field-report", FieldStage::Evidence, None, "Field report", "Write the alpha field report from captured artifacts.", "Report identifies device, path, failure mode, recovery time, and blockers.", true),
        ],
        evidence: vec![
            evidence("field-evidence-json", "Operator-submitted field evidence", "artifacts/field/field-evidence.json", true),
            evidence("field-report-md", "Human-readable field report", "artifacts/field/mobile-field-report.md", true),
            evidence("support-bundle-json", "OpenIRL support bundle after field run", "artifacts/field/support-bundle.json", true),
            evidence("metrics-before-json", "Metrics before brownout", "artifacts/field/metrics-before.json", true),
            evidence("metrics-brownout-json", "Metrics during brownout", "artifacts/field/metrics-brownout.json", true),
            evidence("metrics-recovery-json", "Metrics after recovery", "artifacts/field/metrics-recovery.json", true),
            evidence("moblin-profile-svg", "Moblin QR/profile evidence", "artifacts/field/moblin-profile.svg", true),
            evidence("irlpro-profile-svg", "IRL Pro QR/profile evidence", "artifacts/field/irlpro-profile.svg", true),
            evidence("belabox-config-note", "BELABOX config screenshot/note", "artifacts/field/belabox-config.md", false),
            evidence("obs-field-log", "OBS smoke/scene log", "artifacts/field/obs-field-smoke.json", true),
        ],
        notes: vec![
            "Run the first live-device pass against a private test channel or disconnected OBS output profile.".to_string(),
            "Prefer SRTLA for Moblin, IRL Pro, and BELABOX when bonding or multiple links are in scope; fall back to SRT for simpler local validation.".to_string(),
            "Capture evidence before and after brownout so recovery timing is not based on memory.".to_string(),
            "Do not publish field artifacts until stream keys, SRT passphrases, LAN IPs, and public IPs have been redacted.".to_string(),
        ],
    }
}

/// Builds device-specific field checklists.
#[must_use]
pub fn build_device_checklists() -> Vec<FieldDeviceChecklist> {
    vec![
        FieldDeviceChecklist {
            device: FieldDevice::Moblin,
            encoder: FieldDevice::Moblin.encoder(),
            preferred_protocol: Protocol::Srtla,
            acceptable_protocols: vec![Protocol::Srtla, Protocol::Srt, Protocol::Rtmp, Protocol::Rist, Protocol::Whip],
            setup_steps: vec![
                "Generate a Moblin profile or QR from OpenIRL.".to_string(),
                "Scan/import the profile on the iOS device.".to_string(),
                "Publish to the local direct or MediaMTX relay endpoint.".to_string(),
                "Record whether Moblin accepts latency, stream ID, and passphrase as generated.".to_string(),
            ],
            pass_criteria: vec![
                "OpenIRL sees a metrics sample or active ingest path.".to_string(),
                format!("Healthy state maps to {} scene.", SceneRole::Live),
                "Brownout simulation triggers fallback behavior without crashing the agent.".to_string(),
            ],
            failure_notes: vec![
                "Record exact profile URL shape accepted/rejected by Moblin.".to_string(),
                "Record whether the failure was QR import, network, relay, OBS, or health scoring.".to_string(),
            ],
        },
        FieldDeviceChecklist {
            device: FieldDevice::IrlPro,
            encoder: FieldDevice::IrlPro.encoder(),
            preferred_protocol: Protocol::Srtla,
            acceptable_protocols: vec![Protocol::Srtla, Protocol::Srt, Protocol::Rtmp],
            setup_steps: vec![
                "Generate an IRL Pro profile from OpenIRL.".to_string(),
                "Import or manually enter the generated endpoint on Android.".to_string(),
                "Publish SRTLA first, then SRT if bonding is unavailable.".to_string(),
                "Capture bitrate and link-count behavior during movement or link toggles.".to_string(),
            ],
            pass_criteria: vec![
                "IRL Pro ingest appears in OpenIRL/MediaMTX metrics.".to_string(),
                "At least one degraded or brownout sample can be recorded without losing the control plane.".to_string(),
                format!("Recovery returns toward {} after stable samples.", HealthState::Healthy),
            ],
            failure_notes: vec![
                "Record Android network state, active links, and whether SRTLA server settings matched OpenIRL.".to_string(),
                "Capture the first failed health decision reason if scene automation misfires.".to_string(),
            ],
        },
        FieldDeviceChecklist {
            device: FieldDevice::Belabox,
            encoder: FieldDevice::Belabox.encoder(),
            preferred_protocol: Protocol::Srtla,
            acceptable_protocols: vec![Protocol::Srtla, Protocol::Srtla2, Protocol::Srt],
            setup_steps: vec![
                "Generate BELABOX endpoint settings from OpenIRL.".to_string(),
                "Apply endpoint, stream ID, latency, and passphrase in the BELABOX UI/config.".to_string(),
                "Publish through the local relay or friend/VPS relay path.".to_string(),
                "Toggle or remove one network link to verify metrics and brownout behavior.".to_string(),
            ],
            pass_criteria: vec![
                "BELABOX contribution survives at least one link-change event or produces a clear diagnostic when it cannot.".to_string(),
                "OpenIRL records link/bitrate degradation and fallback timing.".to_string(),
            ],
            failure_notes: vec![
                "Record BELABOX software version, hardware, modem count, and chosen codec.".to_string(),
                "Record whether SRTLA2 was attempted or deferred.".to_string(),
            ],
        },
        FieldDeviceChecklist {
            device: FieldDevice::Larix,
            encoder: FieldDevice::Larix.encoder(),
            preferred_protocol: Protocol::Srt,
            acceptable_protocols: vec![Protocol::Srt, Protocol::Rtmp, Protocol::Rist, Protocol::Whip],
            setup_steps: vec![
                "Generate a Larix-compatible profile from OpenIRL.".to_string(),
                "Publish a simple SRT stream into the local route.".to_string(),
                "Record feature gaps if premium-only Larix behavior is required.".to_string(),
            ],
            pass_criteria: vec![
                "Basic SRT contribution reaches OBS or MediaMTX.".to_string(),
                "Fallback logic still works with non-SRTLA contribution.".to_string(),
            ],
            failure_notes: vec!["Mark Larix as compatibility-only if advanced IRL features require a paid app tier.".to_string()],
        },
    ]
}

/// Returns an all-false sample evidence payload suitable for editing.
#[must_use]
pub fn sample_field_evidence() -> FieldEvidenceInput {
    FieldEvidenceInput {
        static_validation_passed: false,
        rust_ci_passed: false,
        windows_alpha_ready: false,
        moblin_profile_generated: false,
        moblin_qr_scanned: false,
        moblin_ingest_seen: false,
        irlpro_profile_generated: false,
        irlpro_qr_scanned: false,
        irlpro_ingest_seen: false,
        belabox_profile_generated: false,
        belabox_config_reviewed: false,
        belabox_ingest_seen: false,
        mediamtx_srt_path_active: false,
        mediamtx_metrics_seen: false,
        obs_connected: false,
        obs_media_source_seen: false,
        healthy_state_seen: false,
        brownout_state_seen: false,
        brb_scene_seen: false,
        recovery_state_seen: false,
        support_bundle_captured: false,
        secrets_redacted: false,
        field_report_written: false,
    }
}

/// Evaluates field evidence.
#[must_use]
pub fn evaluate_field_evidence(input: &FieldEvidenceInput) -> FieldEvidenceReport {
    let mut required = RequirementCounter::default();
    let mut warnings = Vec::new();
    let mut passed = Vec::new();
    let mut failed = Vec::new();

    required.check(
        input.static_validation_passed,
        "static validation passed",
        &mut passed,
        &mut failed,
    );
    required.check(
        input.rust_ci_passed,
        "Rust CI passed",
        &mut passed,
        &mut failed,
    );
    required.check(
        input.windows_alpha_ready,
        "Windows/OBS alpha baseline ready",
        &mut passed,
        &mut failed,
    );
    required.check(
        input.moblin_profile_generated,
        "Moblin profile generated",
        &mut passed,
        &mut failed,
    );
    required.check(
        input.moblin_qr_scanned,
        "Moblin QR/profile accepted",
        &mut passed,
        &mut failed,
    );
    required.check(
        input.moblin_ingest_seen,
        "Moblin ingest observed",
        &mut passed,
        &mut failed,
    );
    required.check(
        input.irlpro_profile_generated,
        "IRL Pro profile generated",
        &mut passed,
        &mut failed,
    );
    required.check(
        input.irlpro_qr_scanned,
        "IRL Pro QR/profile accepted",
        &mut passed,
        &mut failed,
    );
    required.check(
        input.irlpro_ingest_seen,
        "IRL Pro ingest observed",
        &mut passed,
        &mut failed,
    );
    required.check(
        input.mediamtx_srt_path_active,
        "MediaMTX SRT path active",
        &mut passed,
        &mut failed,
    );
    required.check(
        input.mediamtx_metrics_seen,
        "MediaMTX/relay metrics seen",
        &mut passed,
        &mut failed,
    );
    required.check(
        input.obs_connected,
        "OBS connected",
        &mut passed,
        &mut failed,
    );
    required.check(
        input.obs_media_source_seen,
        "OBS media source observed",
        &mut passed,
        &mut failed,
    );
    required.check(
        input.healthy_state_seen,
        "healthy health state observed",
        &mut passed,
        &mut failed,
    );
    required.check(
        input.brownout_state_seen,
        "brownout health state observed",
        &mut passed,
        &mut failed,
    );
    required.check(
        input.brb_scene_seen,
        "BRB/fallback scene observed",
        &mut passed,
        &mut failed,
    );
    required.check(
        input.recovery_state_seen,
        "recovery state observed",
        &mut passed,
        &mut failed,
    );
    required.check(
        input.support_bundle_captured,
        "support bundle captured",
        &mut passed,
        &mut failed,
    );
    required.check(
        input.secrets_redacted,
        "field artifacts redacted",
        &mut passed,
        &mut failed,
    );
    required.check(
        input.field_report_written,
        "field report written",
        &mut passed,
        &mut failed,
    );

    if input.belabox_profile_generated && input.belabox_config_reviewed && input.belabox_ingest_seen
    {
        passed.push("BELABOX compatibility evidence captured".to_string());
    } else {
        warnings.push("BELABOX remains optional for mobile field alpha signoff but should be validated before broader backpack alpha.".to_string());
    }

    if input.brownout_state_seen && !input.brb_scene_seen {
        warnings.push("Brownout was seen but BRB/fallback scene was not confirmed; inspect scene automation hysteresis.".to_string());
    }

    if input.mediamtx_metrics_seen && !input.mediamtx_srt_path_active {
        warnings.push("Metrics were present but no active SRT path was confirmed; verify path labels and source selection.".to_string());
    }

    let required_total = required.total;
    let required_passed = required.passed;
    let ready_for_private_field_alpha = required_total > 0 && required_passed == required_total;
    let score = score(required_passed, required_total);
    let blockers = if ready_for_private_field_alpha {
        Vec::new()
    } else {
        failed.clone()
    };
    let next_actions = next_actions(input, &blockers, &warnings);
    let summary = if ready_for_private_field_alpha {
        "mobile field alpha field evidence is complete for private mobile alpha.".to_string()
    } else {
        format!(
            "mobile field alpha field evidence is incomplete: {required_passed}/{required_total} required checks passed."
        )
    };

    FieldEvidenceReport {
        generated_at: OffsetDateTime::now_utc(),
        ready_for_private_field_alpha,
        score,
        required_passed,
        required_total,
        blockers,
        warnings,
        passed,
        failed,
        next_actions,
        summary,
    }
}

#[derive(Default)]
struct RequirementCounter {
    total: u16,
    passed: u16,
}

impl RequirementCounter {
    fn check(
        &mut self,
        value: bool,
        label: &str,
        passed: &mut Vec<String>,
        failed: &mut Vec<String>,
    ) {
        self.total = self.total.saturating_add(1);
        if value {
            self.passed = self.passed.saturating_add(1);
            passed.push(label.to_string());
        } else {
            failed.push(label.to_string());
        }
    }
}

fn score(passed: u16, total: u16) -> u8 {
    if total == 0 {
        return 0;
    }
    let value = u32::from(passed).saturating_mul(100) / u32::from(total);
    if value > 100 { 100 } else { value as u8 }
}

fn next_actions(
    input: &FieldEvidenceInput,
    blockers: &[String],
    warnings: &[String],
) -> Vec<String> {
    let mut actions = Vec::new();
    if !input.moblin_ingest_seen {
        actions
            .push("Run Moblin SRTLA/SRT ingest test and capture relay/OBS evidence.".to_string());
    }
    if !input.irlpro_ingest_seen {
        actions
            .push("Run IRL Pro SRTLA/SRT ingest test and capture relay/OBS evidence.".to_string());
    }
    if !input.brownout_state_seen || !input.recovery_state_seen {
        actions.push(
            "Capture a controlled brownout and stable recovery sequence with timestamps."
                .to_string(),
        );
    }
    if !input.support_bundle_captured || !input.secrets_redacted {
        actions.push(
            "Capture and redact the support bundle before sharing field artifacts.".to_string(),
        );
    }
    if blockers.is_empty() && warnings.is_empty() {
        actions.push(
            "Promote to private field alpha with one additional streamer/operator.".to_string(),
        );
    }
    actions
}

fn check(
    id: &str,
    stage: FieldStage,
    device: Option<FieldDevice>,
    label: &str,
    action: &str,
    expected: &str,
    blocking: bool,
) -> FieldValidationCheck {
    FieldValidationCheck {
        id: id.to_string(),
        stage,
        device,
        label: label.to_string(),
        action: action.to_string(),
        expected: expected.to_string(),
        blocking,
        status: FieldStatus::NeedsOperator,
    }
}

fn evidence(id: &str, description: &str, source: &str, required: bool) -> FieldEvidenceItem {
    FieldEvidenceItem {
        id: id.to_string(),
        description: description.to_string(),
        source: source.to_string(),
        required,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_evidence_is_not_ready() {
        let report = evaluate_field_evidence(&sample_field_evidence());
        assert!(!report.ready_for_private_field_alpha);
        assert_eq!(report.required_passed, 0);
    }

    #[test]
    fn plan_contains_moblin_and_irl_pro() {
        let plan = build_field_validation_plan(17);
        assert!(
            plan.device_checklists
                .iter()
                .any(|item| item.device == FieldDevice::Moblin)
        );
        assert!(
            plan.device_checklists
                .iter()
                .any(|item| item.device == FieldDevice::IrlPro)
        );
    }

    #[test]
    fn compatibility_matrix_keeps_live_claims_unproven() {
        let revision = "a".repeat(40);
        let matrix = build_compatibility_matrix(38, revision, "2026-08-01");
        let validation = validate_compatibility_matrix(&matrix);
        assert!(validation.ok, "{:?}", validation.errors);
        assert!(matrix.rows.iter().any(|row| {
            row.maturity == EvidenceMaturity::LocalRuntimeValidated
                && row.result == CompatibilityResult::Passed
        }));
        assert!(!matrix.rows.iter().any(|row| {
            matches!(
                row.maturity,
                EvidenceMaturity::IntegrationValidated
                    | EvidenceMaturity::FieldValidated
                    | EvidenceMaturity::Released
            ) && row.result == CompatibilityResult::Passed
        }));
    }

    #[test]
    fn compatibility_matrix_rejects_unsafe_or_overstated_evidence() {
        let mut matrix = build_compatibility_matrix(38, "a".repeat(40), "2026-99-99");
        matrix.rows[0].maturity = EvidenceMaturity::FieldValidated;
        matrix.rows[0].notes = "dashboard_token=synthetic-secret".to_string();
        let validation = validate_compatibility_matrix(&matrix);
        assert!(!validation.ok);
        assert!(validation.errors.len() >= 3);
    }

    #[test]
    fn checked_in_compatibility_matrix_is_valid() -> Result<(), Box<dyn std::error::Error>> {
        let matrix: CompatibilityMatrix =
            serde_json::from_str(include_str!("../../../compatibility/matrix-v1.json"))?;
        let validation = validate_compatibility_matrix(&matrix);
        assert!(validation.ok, "{:?}", validation.errors);
        assert_eq!(
            matrix,
            build_compatibility_matrix(
                38,
                "73d54bea13b5d02bb5d5b91c54cc74e49cc2a66d",
                "2026-08-01"
            )
        );
        Ok(())
    }

    #[test]
    fn contributor_live_smoke_fixture_cannot_claim_a_run() -> Result<(), Box<dyn std::error::Error>>
    {
        let row: CompatibilityRow = serde_json::from_str(include_str!(
            "../../../fixtures/contributing/live-smoke-evidence.sample.json"
        ))?;
        let matrix = CompatibilityMatrix {
            matrix_revision: 1,
            schema_revision: 38,
            source_revision: row.openirl_revision.clone(),
            reviewed_on: "2026-08-01".to_string(),
            rows: vec![row],
            evidence_policy: vec!["Synthetic schema fixture only; no live claim.".to_string()],
        };
        let validation = validate_compatibility_matrix(&matrix);
        assert!(validation.ok, "{:?}", validation.errors);
        assert_eq!(matrix.rows[0].maturity, EvidenceMaturity::Modeled);
        assert_eq!(matrix.rows[0].result, CompatibilityResult::NotRun);
        Ok(())
    }
}
