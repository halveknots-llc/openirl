//! Real mobile field-stream validation planning and evidence scoring.
//!
//! mobile field alpha moves OpenIRL beyond desktop/OBS readiness and into real encoder
//! field validation. This crate is deliberately pure Rust: it models the
//! operator plan, device checklists, and pass/fail evidence without depending on
//! a live phone, backpack, MediaMTX process, or OBS instance.

use openirl_core::{EncoderKind, HealthState, Protocol, SceneRole};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

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
            check("field-report", FieldStage::Evidence, None, "Field report", "Write the private alpha field report from captured artifacts.", "Report identifies device, path, failure mode, recovery time, and blockers.", true),
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
}
