//! Release manifest and gate modeling for OpenIRL.

use openirl_config::ConfigValidationReport;
use serde::{Deserialize, Serialize};

/// Release artifact kind.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArtifactKind {
    /// Source archive.
    SourceZip,
    /// Windows portable package.
    WindowsPortable,
    /// Windows MSI package.
    WindowsMsi,
    /// Release manifest.
    Manifest,
    /// Checksum file.
    Checksum,
}

/// Release artifact metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseArtifact {
    /// Artifact kind.
    pub kind: ArtifactKind,
    /// Artifact path or name.
    pub path: String,
    /// Whether the artifact is required for this release tier.
    pub required: bool,
}

/// Release gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseGate {
    /// Stable gate ID.
    pub id: String,
    /// Human-readable label.
    pub label: String,
    /// Gate result.
    pub passing: bool,
    /// Evidence command or file.
    pub evidence: String,
}

/// Smoke test command entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SmokeTestEntry {
    /// Test name.
    pub name: String,
    /// Command.
    pub command: String,
    /// Whether this test requires external runtime dependencies.
    pub requires_external_runtime: bool,
}

/// Compatibility row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompatibilityEntry {
    /// Component.
    pub component: String,
    /// Expected status.
    pub status: String,
    /// Notes.
    pub notes: String,
}

/// Release manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseManifest {
    /// OpenIRL version.
    pub version: String,
    /// Schema revision.
    pub schema_revision: u16,
    /// Release tier.
    pub tier: String,
    /// Artifacts.
    pub artifacts: Vec<ReleaseArtifact>,
    /// Gates.
    pub gates: Vec<ReleaseGate>,
    /// Smoke tests.
    pub smoke_tests: Vec<SmokeTestEntry>,
    /// Compatibility.
    pub compatibility: Vec<CompatibilityEntry>,
}

/// Builds the default release manifest.
#[must_use]
pub fn build_release_manifest(
    version: impl Into<String>,
    schema_revision: u16,
    validation: &ConfigValidationReport,
) -> ReleaseManifest {
    ReleaseManifest {
        version: version.into(),
        schema_revision,
        tier: "public-alpha-source".to_string(),
        artifacts: vec![
            artifact(ArtifactKind::SourceZip, "openirl-source-alpha.zip", true),
            artifact(
                ArtifactKind::Manifest,
                "dist/manifest/openirl-release-manifest.json",
                true,
            ),
            artifact(
                ArtifactKind::Checksum,
                "openirl-source-alpha.zip.sha256",
                true,
            ),
            artifact(
                ArtifactKind::WindowsPortable,
                "target/release/openirl-agent.exe",
                false,
            ),
            artifact(
                ArtifactKind::WindowsMsi,
                "target/wix/openirl-agent.msi",
                false,
            ),
        ],
        gates: release_gates(validation, schema_revision),
        smoke_tests: smoke_tests(),
        compatibility: compatibility_matrix(),
    }
}

/// Builds release gates.
#[must_use]
pub fn release_gates(
    validation: &ConfigValidationReport,
    schema_revision: u16,
) -> Vec<ReleaseGate> {
    vec![
        gate(
            "schema-revision",
            "Schema revision is current",
            schema_revision >= 38,
            "GET /health reports schema_revision >= 38",
        ),
        gate(
            "config-validation",
            "Default config validates",
            validation.ok,
            "openirl-agent check-config --config config/openirl.example.toml",
        ),
        gate(
            "static-validation",
            "Static repository validation result supplied",
            false,
            "python3 scripts/static_validate.py",
        ),
        gate(
            "obs-validation",
            "OBS automation smoke test result supplied",
            false,
            "scripts/obs/reconcile-smoke.sh or .ps1",
        ),
        gate(
            "field-evidence",
            "Mobile field evidence result supplied",
            false,
            "scripts/field/mobile-field-evidence.sh or .ps1",
        ),
        gate(
            "artifact-materialization",
            "Fallback assets and OBS templates materialization result supplied",
            false,
            "openirl-agent artifacts materialize-fallback and artifacts obs-template --materialize",
        ),
        gate(
            "support-bundle",
            "Support-bundle export result supplied",
            false,
            "POST /api/session/support-bundle/export",
        ),
        gate(
            "security-review",
            "Security review result supplied",
            false,
            "python3 scripts/security/security-audit-smoke.py",
        ),
    ]
}

fn artifact(kind: ArtifactKind, path: &str, required: bool) -> ReleaseArtifact {
    ReleaseArtifact {
        kind,
        path: path.to_string(),
        required,
    }
}
fn gate(id: &str, label: &str, passing: bool, evidence: &str) -> ReleaseGate {
    ReleaseGate {
        id: id.to_string(),
        label: label.to_string(),
        passing,
        evidence: evidence.to_string(),
    }
}
fn smoke_tests() -> Vec<SmokeTestEntry> {
    vec![
        smoke(
            "static validation",
            "python3 scripts/static_validate.py",
            false,
        ),
        smoke("agent API", "python3 scripts/smoke/api_smoke.py", true),
        smoke(
            "OBS WebSocket",
            "pwsh scripts/smoke/obs-websocket-smoke.ps1",
            true,
        ),
        smoke(
            "local ingest",
            "bash scripts/ingest/local-ingest-smoke.sh",
            true,
        ),
        smoke(
            "support bundle",
            "bash scripts/support/support-bundle-v2-smoke.sh",
            true,
        ),
    ]
}
fn smoke(name: &str, command: &str, requires_external_runtime: bool) -> SmokeTestEntry {
    SmokeTestEntry {
        name: name.to_string(),
        command: command.to_string(),
        requires_external_runtime,
    }
}
fn compatibility_matrix() -> Vec<CompatibilityEntry> {
    vec![
        compat(
            "OBS Studio",
            "requires-live-validation",
            "OBS 28+ WebSocket path is modeled; run the OBS smoke against a real profile",
        ),
        compat(
            "MediaMTX",
            "requires-live-validation",
            "local router and metrics paths are modeled; run ingest smoke against a real process",
        ),
        compat(
            "Moblin",
            "requires-live-validation",
            "SRT/SRTLA profile generation is present; validate import on a real iOS device",
        ),
        compat(
            "IRL Pro",
            "requires-live-validation",
            "SRT/SRTLA profile generation is present; validate import on a real Android device",
        ),
        compat(
            "BELABOX",
            "requires-live-validation",
            "SRTLA and backpack presets are present; validate against real BELABOX tooling",
        ),
    ]
}
fn compat(component: &str, status: &str, notes: &str) -> CompatibilityEntry {
    CompatibilityEntry {
        component: component.to_string(),
        status: status.to_string(),
        notes: notes.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openirl_config::AppConfig;

    #[test]
    fn release_manifest_has_required_artifacts() {
        let validation = AppConfig::default().validate();
        let manifest = build_release_manifest("0.1.0", 38, &validation);
        assert!(manifest.artifacts.iter().any(|item| item.required));
        assert!(
            manifest
                .gates
                .iter()
                .any(|gate| gate.id == "schema-revision" && gate.passing)
        );
    }
}
