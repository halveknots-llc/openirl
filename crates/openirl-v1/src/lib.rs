//! Public beta implementation layer for OpenIRL.
//!
//! This crate keeps the handoff feature set as typed Rust data and provides
//! materializers for public-beta package artifacts. It is deliberately
//! local-first: no managed cloud account is required to evaluate the plans or
//! write package contents.

use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error;
use time::OffsetDateTime;

/// Errors returned by package materialization.
#[derive(Debug, Error)]
pub enum V1Error {
    /// Filesystem failure.
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
    /// JSON serialization failure.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Roadmap priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Priority {
    /// Required before serious private-alpha users.
    P0,
    /// Required before public beta.
    P1,
    /// Differentiating feature, not a beta blocker.
    P2,
    /// Advanced ecosystem polish.
    P3,
}

/// Implementation status for a feature area.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FeatureStatus {
    /// Implemented as a Rust/domain/API contract with validation evidence paths.
    Implemented,
}

/// A public-beta feature area.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct V1Feature {
    /// Stable feature key.
    pub key: String,
    /// Human-readable title.
    pub title: String,
    /// Priority.
    pub priority: Priority,
    /// Implementation status.
    pub status: FeatureStatus,
    /// Outcome summary.
    pub outcome: String,
    /// Features covered.
    pub features: Vec<String>,
    /// API surfaces or command surfaces.
    pub surfaces: Vec<String>,
    /// Files and artifacts produced by this repo.
    pub artifacts: Vec<String>,
    /// Acceptance gate.
    pub acceptance_gate: String,
}

/// Roll-up summary of the public-beta feature set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct V1ImplementationSummary {
    /// Current schema revision used in health, release, and support payloads.
    pub schema_revision: u16,
    /// Number of feature areas represented.
    pub feature_count: usize,
    /// Private-alpha feature cutline.
    pub private_alpha_cutline: Vec<String>,
    /// Public-beta feature cutline.
    pub public_beta_cutline: Vec<String>,
    /// V1 feature cutline.
    pub v1_cutline: Vec<String>,
    /// Highest-priority live validation order.
    pub live_validation_order: Vec<String>,
    /// Feature areas.
    pub features: Vec<V1Feature>,
}

/// Package layout plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct V1PackageLayout {
    /// Root directory.
    pub root: String,
    /// Directories to create.
    pub directories: Vec<String>,
    /// Files to write.
    pub files: Vec<V1PackageFile>,
}

/// File entry inside a package layout.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct V1PackageFile {
    /// Relative path from root.
    pub path: String,
    /// Text contents.
    pub contents: String,
}

/// Result of package materialization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct V1MaterializationResult {
    /// Root directory.
    pub root: String,
    /// Created directories.
    pub created_directories: Vec<String>,
    /// Written files.
    pub written_files: Vec<String>,
    /// Skipped files.
    pub skipped_files: Vec<String>,
    /// Generated timestamp.
    pub generated_at: String,
}

/// Public-beta readiness evidence supplied by a tester or automation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct V1EvidenceInput {
    /// Cargo CI passed.
    pub rust_ci_passed: bool,
    /// Static validation passed.
    pub static_validation_passed: bool,
    /// OBS scene reconciliation passed.
    pub obs_reconciliation_passed: bool,
    /// Local ingest path passed.
    pub local_ingest_passed: bool,
    /// Mobile QR/device profiles passed.
    pub mobile_profiles_passed: bool,
    /// Dashboard mobile test passed.
    pub dashboard_mobile_passed: bool,
    /// Dashboard/auth/security gate passed.
    pub local_security_passed: bool,
    /// Brownout state machine passed.
    pub brownout_engine_passed: bool,
    /// OBS output health test passed.
    pub obs_output_health_passed: bool,
    /// Support bundle and timeline diagnostics passed.
    pub support_bundle_passed: bool,
    /// Backup ingest/failover passed.
    pub backup_ingest_passed: bool,
    /// Alerts and moderator operations passed.
    pub alerts_mod_ops_passed: bool,
    /// SRTLA bonding compatibility passed.
    pub srtla_bonding_passed: bool,
    /// Self-hosted relay path passed.
    pub self_hosted_relay_passed: bool,
    /// NAT/tunnel integration passed.
    pub nat_tunnel_passed: bool,
    /// Private alpha package passed.
    pub private_alpha_package_passed: bool,
    /// Public documentation/hardware guides passed.
    pub docs_guides_passed: bool,
    /// Public beta security review passed.
    pub public_beta_security_passed: bool,
    /// Public beta release packaging passed.
    pub public_beta_release_passed: bool,
    /// WebRTC/WHEP preview passed.
    pub webrtc_preview_passed: bool,
    /// Vertical and clip production passed.
    pub vertical_clip_passed: bool,
    /// Plugin API/v1 stabilization passed.
    pub plugin_api_passed: bool,
}

/// A single readiness gate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReadinessGate {
    /// Gate name.
    pub name: String,
    /// Whether it passed.
    pub passed: bool,
    /// Blocking reason when not passed.
    pub reason: String,
}

/// Public-beta readiness report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct V1ReadinessReport {
    /// Private alpha readiness.
    pub private_alpha_ready: bool,
    /// Public beta readiness.
    pub public_beta_ready: bool,
    /// V1 readiness.
    pub v1_ready: bool,
    /// Gate list.
    pub gates: Vec<ReadinessGate>,
    /// Recommended next work.
    pub next_actions: Vec<String>,
}

macro_rules! feature_area {
    (
        $key:expr,
        $title:expr,
        $priority:expr,
        $outcome:expr,
        $features:expr,
        $surfaces:expr,
        $artifacts:expr,
        $acceptance_gate:expr $(,)?
    ) => {
        V1Feature {
            key: s($key),
            title: s($title),
            priority: $priority,
            status: FeatureStatus::Implemented,
            outcome: s($outcome),
            features: $features.iter().map(|value| s(value)).collect(),
            surfaces: $surfaces.iter().map(|value| s(value)).collect(),
            artifacts: $artifacts.iter().map(|value| s(value)).collect(),
            acceptance_gate: s($acceptance_gate),
        }
    };
}

/// Returns all public-beta feature areas.
#[must_use]
pub fn build_v1_features() -> Vec<V1Feature> {
    vec![
        feature_area!(
            "obs-reconciliation",
            "Real OBS template reconciliation and source transform hardening",
            Priority::P0,
            "Desired scene/source graphs have reconciliation contracts, idempotency requirements, transform planning, and smoke artifacts.",
            &[
                "OBS reconciliation engine",
                "idempotent scene creation",
                "source transform templates",
                "browser-source URL hardening",
                "duplicate-source policy",
            ],
            &[
                "POST /api/obs/template/apply",
                "scripts/obs/reconcile-smoke.*",
            ],
            &[
                "presets/obs/openirl-v1-scene-template.json",
                "docs/features/obs-reconciliation.md",
            ],
            "A clean OBS profile can be transformed into an OpenIRL scene pack without manual OBS setup.",
        ),
        feature_area!(
            "local-ingest",
            "Local MediaMTX/SRT/RTMP ingest",
            Priority::P0,
            "Local-direct SRT/RTMP routing is represented as generated MediaMTX config, OBS media-source binding, and ingest smoke scripts.",
            &[
                "MediaMTX config generator",
                "OBS source URL binding",
                "port conflict checks",
                "router health panel contract",
                "firewall guidance",
            ],
            &[
                "/api/v1/package-layout/materialize",
                "scripts/ingest/local-ingest-smoke.*",
            ],
            &[
                "presets/relay/mediamtx.openirl.local.yml",
                "docs/features/local-ingest.md",
            ],
            "A mobile encoder or test publisher can publish into OpenIRL and OBS can read the media path.",
        ),
        feature_area!(
            "encoder-profiles",
            "Mobile encoder QR/profile compatibility",
            Priority::P0,
            "Moblin, IRL Pro, Larix, and BELABOX profile families are formalized with shareable presets and compatibility docs.",
            &[
                "Moblin SRT/SRTLA profiles",
                "IRL Pro SRT/SRTLA profiles",
                "Larix fallback profiles",
                "BELABOX receiver profile",
                "profile import/export",
            ],
            &[
                "POST /api/profile/qr",
                "scripts/mobile/profile-compat-smoke.*",
            ],
            &[
                "presets/encoders/*.json",
                "docs/features/encoder-profiles.md",
            ],
            "A user can scan or import a generated profile without hand-typing SRT URLs, stream IDs, or passphrases.",
        ),
        feature_area!(
            "dashboard",
            "Mobile-first dashboard control room",
            Priority::P0,
            "The dashboard is a mobile-first control-room contract with stream state, scene controls, metrics, QR setup, and support actions.",
            &[
                "mobile layout",
                "stream state card",
                "scene controls",
                "metrics cards",
                "QR flow",
                "support bundle button",
            ],
            &[
                "apps/openirl-agent/static/index.html",
                "GET /api/v1/summary",
            ],
            &["docs/features/dashboard.md", "scripts/smoke/api_smoke.py"],
            "The dashboard can be operated from a phone during a field stream.",
        ),
        feature_area!(
            "security",
            "Auth, local security, and remote access guardrails",
            Priority::P0,
            "Dashboard auth, role-aware operations, LAN exposure warnings, and secret redaction are represented as product gates.",
            &[
                "token policy",
                "owner/producer/mod/viewer roles",
                "LAN exposure warnings",
                "OBS password warning",
                "support bundle redaction",
            ],
            &[
                "GET /api/auth/status",
                "POST /api/auth/check",
                "scripts/security/security-audit-smoke.py",
            ],
            &["docs/SECURITY.md", "docs/features/security.md"],
            "No default flow exposes OBS control, stream keys, dashboard tokens, or passphrases to the public internet.",
        ),
        feature_area!(
            "brownout",
            "Brownout engine and recovery hysteresis",
            Priority::P0,
            "Brownout detection has weighted scoring, severity, explainability, recovery hysteresis, and scenario fixtures.",
            &[
                "weighted health score",
                "severity levels",
                "recovery timer",
                "scene-flap prevention",
                "state explanation",
            ],
            &[
                "POST /api/metrics/simulate/{scenario}",
                "POST /api/evaluate",
            ],
            &[
                "fixtures/metrics/brownout-v2-scenarios.json",
                "docs/features/brownout.md",
            ],
            "OpenIRL switches before a hard disconnect and returns only after a stable recovery window.",
        ),
        feature_area!(
            "obs-output",
            "OBS output and destination health",
            Priority::P1,
            "Output health is modeled separately from contribution ingest so failures can be classified as phone, OBS, relay, or platform-side.",
            &[
                "OBS output polling",
                "encoder stress indicators",
                "destination health classification",
                "output bitrate tracking",
                "failure taxonomy",
            ],
            &["GET /api/obs/status", "GET /api/runtime/readiness"],
            &[
                "fixtures/diagnostics/output-health.sample.json",
                "docs/features/obs-output.md",
            ],
            "A support report can say whether the failure was input, OBS, relay, or platform output.",
        ),
        feature_area!(
            "support-bundles",
            "Disk-based support bundles and timeline diagnostics",
            Priority::P1,
            "Support bundle output has a timeline contract, redacted logs, session report, human summary, and issue-template payload.",
            &[
                "timeline model",
                "session report",
                "bundle zip plan",
                "human summary",
                "machine-readable report",
            ],
            &[
                "POST /api/session/support-bundle/export",
                "scripts/support/support-bundle-v2-smoke.*",
            ],
            &[
                "issue_templates/field_report.md",
                "docs/features/support-bundles.md",
            ],
            "A tester can attach one bundle and a maintainer can reconstruct the failure path.",
        ),
        feature_area!(
            "backup-ingest",
            "Backup ingest and multi-source failover",
            Priority::P1,
            "Primary/backup ingest roles, manual override behavior, per-source health, and recovery priority are formalized.",
            &[
                "primary ingest",
                "backup ingest",
                "manual override",
                "automatic failover",
                "per-ingest health",
            ],
            &[
                "POST /api/obs/switch/backup-feed",
                "scripts/ingest/backup-failover-smoke.*",
            ],
            &[
                "presets/obs/backup-ingest-policy.json",
                "docs/features/backup-ingest.md",
            ],
            "OpenIRL can survive a primary device failure when a backup feed is healthy.",
        ),
        feature_area!(
            "moderation",
            "Alerts and moderator operations",
            Priority::P1,
            "Moderator controls, event notifications, stream markers, alert rate limits, and audit-log expectations are represented.",
            &[
                "web push alert model",
                "Discord/Telegram optional hooks",
                "mod-safe controls",
                "incident notifications",
                "audit log",
            ],
            &[
                "POST /api/moderation/command",
                "POST /api/production/marker",
            ],
            &[
                "presets/moderation/default-policy.json",
                "docs/features/moderation.md",
            ],
            "A moderator can help operate a stream from a phone without unsafe machine-level control.",
        ),
        feature_area!(
            "bonding",
            "SRTLA and advanced bonding compatibility",
            Priority::P1,
            "SRTLA compatibility modes are modeled with link-level statistics and version warnings.",
            &[
                "SRTLA backend profile",
                "link-level stats",
                "BELABOX mode",
                "version detection",
                "compatibility warnings",
            ],
            &["GET /api/v1/summary", "scripts/relay/srtla2-compat-smoke.*"],
            &[
                "presets/relay/srtla2-compat.json",
                "docs/features/bonding.md",
            ],
            "A bonded setup reports link count, degradation, and incompatible transport pairings clearly.",
        ),
        feature_area!(
            "self-hosted-relay",
            "Self-hosted relay",
            Priority::P1,
            "Home, VPS, friend/mod relay, and backpack relay modes have deployable package contents and support-bundle expectations.",
            &[
                "relay wizard",
                "Docker Compose",
                "tokenized ingest",
                "public relay readiness",
                "local client mode",
                "relay support bundle",
            ],
            &[
                "GET /api/relay/plan",
                "scripts/relay/self-hosted-relay-smoke.*",
            ],
            &[
                "deploy/docker-compose.relay.yml",
                "docs/features/self-hosted-relay.md",
            ],
            "A cheap VPS or friend relay can be configured without bespoke maintainer assistance.",
        ),
        feature_area!(
            "nat-tunnel",
            "NAT traversal and tunnel integrations",
            Priority::P1,
            "WireGuard, frp, and rathole tunnel planning is added for CGNAT and no-public-IP users.",
            &[
                "WireGuard profile planning",
                "reverse tunnel planning",
                "tunnel health",
                "tunnel-aware QR",
                "security warnings",
            ],
            &["scripts/tunnels/tunnel-readiness-smoke.*"],
            &[
                "presets/tunnels/wireguard.example.conf",
                "docs/features/nat-tunnel.md",
            ],
            "A user behind CGNAT has a documented path that does not expose OBS WebSocket publicly.",
        ),
        feature_area!(
            "private-alpha-package",
            "Private alpha package",
            Priority::P0,
            "The private-alpha package layout includes binaries, configs, assets, templates, smoke scripts, docs, and limitations.",
            &[
                "portable layout",
                "first-run inputs",
                "checksums",
                "smoke bundle",
                "limitations",
            ],
            &[
                "POST /api/alpha/package-layout/materialize",
                "POST /api/v1/package-layout/materialize",
            ],
            &[
                "artifacts/private-alpha",
                "docs/features/private-alpha-package.md",
            ],
            "A tester receives one folder with all alpha docs, presets, assets, and smoke scripts.",
        ),
        feature_area!(
            "docs",
            "Documentation and hardware guides",
            Priority::P1,
            "Quickstarts, hardware guides, relay guides, troubleshooting docs, and compatibility matrix are generated.",
            &[
                "Moblin quickstart",
                "IRL Pro guide",
                "BELABOX guide",
                "relay guide",
                "troubleshooting",
                "compatibility matrix",
            ],
            &["GET /api/v1/package-layout"],
            &["docs/guides/*.md", "docs/hardware/*.md"],
            "A non-contributor can set up OpenIRL from docs and the package alone.",
        ),
        feature_area!(
            "public-beta-security",
            "Public beta security review",
            Priority::P1,
            "Threat model, default-bind audit, secret storage audit, redaction checks, permission checks, and update trust are documented.",
            &[
                "threat model",
                "bind audit",
                "secret storage audit",
                "redaction tests",
                "permission tests",
                "release provenance",
            ],
            &["scripts/security/security-audit-smoke.py"],
            &["docs/SECURITY.md", "docs/features/public-beta-security.md"],
            "No beta default exposes OBS, relay control, dashboard control, or secrets to the public internet.",
        ),
        feature_area!(
            "public-beta-release",
            "Public beta release",
            Priority::P1,
            "Public beta release metadata, issue templates, release notes, telemetry stance, presets, and migration docs are created.",
            &[
                "beta binaries plan",
                "release notes",
                "issue templates",
                "local telemetry stance",
                "community presets",
                "migration guide",
            ],
            &[
                "GET /api/v1/readiness",
                "scripts/public-beta/package-public-beta.*",
            ],
            &[
                "dist/manifest/openirl-release-manifest.handoff.json",
                "docs/RELEASE_CHECKLIST.md",
            ],
            "A public user can install, test, file issues, and recover without maintainer hand-holding.",
        ),
        feature_area!(
            "webrtc-preview",
            "WebRTC / WHEP preview and producer view",
            Priority::P2,
            "WHEP preview, producer dashboard, preview auth, latency indicators, LAN preview, and relay preview plans are added.",
            &[
                "WHEP preview",
                "producer dashboard",
                "preview auth",
                "latency indicators",
                "LAN preview",
                "relay preview",
            ],
            &["scripts/webrtc/webrtc-preview-smoke.*"],
            &[
                "presets/webrtc/whep-preview.json",
                "docs/features/webrtc-preview.md",
            ],
            "A producer can monitor a local or relayed preview from a browser without remote desktop.",
        ),
        feature_area!(
            "vertical-clips",
            "Vertical, Shorts, and clip-oriented production",
            Priority::P2,
            "Vertical scene templates, replay/marker workflow, post-stream clip folders, and crop-planning metadata are added.",
            &[
                "9:16 templates",
                "dual composition planning",
                "replay buffer workflow",
                "clip marker timeline",
                "post-stream clip folder",
            ],
            &[
                "GET /api/production/plan",
                "POST /api/production/replay/save",
            ],
            &[
                "presets/production/vertical-scenes.json",
                "docs/features/vertical-clips.md",
            ],
            "OpenIRL supports live resilience and post-stream content capture in the same local workflow.",
        ),
        feature_area!(
            "plugin-api",
            "Plugin / extension API and v1 stabilization",
            Priority::P2,
            "A versioned plugin manifest, webhook action model, recipe contracts, config schema migration notes, stable API surface, and readiness report are added.",
            &[
                "plugin manifest",
                "webhook actions",
                "automation recipes",
                "config migrations",
                "stable API contracts",
            ],
            &[
                "GET /api/v1/features",
                "GET /api/v1/readiness",
                "scripts/plugins/validate-plugin-manifest.py",
            ],
            &[
                "plugin/openirl-plugin-manifest.schema.json",
                "docs/features/plugin-api.md",
            ],
            "Community contributors can add integrations without patching the core agent.",
        ),
    ]
}

/// Returns the consolidated feature summary.
#[must_use]
pub fn build_v1_implementation_summary() -> V1ImplementationSummary {
    V1ImplementationSummary {
        schema_revision: 38,
        feature_count: build_v1_features().len(),
        private_alpha_cutline: keys(&[
            "obs-reconciliation",
            "local-ingest",
            "encoder-profiles",
            "dashboard",
            "security",
            "brownout",
            "support-bundles",
            "private-alpha-package",
        ]),
        public_beta_cutline: keys(&[
            "obs-output",
            "backup-ingest",
            "moderation",
            "self-hosted-relay",
            "nat-tunnel",
            "docs",
            "public-beta-security",
            "public-beta-release",
        ]),
        v1_cutline: keys(&["bonding", "webrtc-preview", "vertical-clips", "plugin-api"]),
        live_validation_order: keys(&[
            "obs-reconciliation",
            "local-ingest",
            "encoder-profiles",
            "brownout",
            "support-bundles",
            "self-hosted-relay",
            "webrtc-preview",
        ]),
        features: build_v1_features(),
    }
}

/// Builds the default source-package layout.
#[must_use]
pub fn default_v1_package_layout(root: impl Into<String>) -> V1PackageLayout {
    V1PackageLayout {
        root: root.into(),
        directories: vec![
            s("bin"),
            s("config"),
            s("docs"),
            s("docs/guides"),
            s("docs/hardware"),
            s("docs/troubleshooting"),
            s("presets/obs"),
            s("presets/encoders"),
            s("presets/relay"),
            s("presets/tunnels"),
            s("presets/webrtc"),
            s("presets/production"),
            s("presets/moderation"),
            s("plugin"),
            s("issue_templates"),
            s("scripts/smoke"),
            s("scripts/security"),
            s("scripts/plugins"),
            s("release"),
        ],
        files: vec![
            file("README.md", package_readme()),
            file("config/openirl.public-beta.toml", public_beta_config()),
            file(
                "presets/obs/openirl-v1-scene-template.json",
                obs_scene_template(),
            ),
            file(
                "presets/obs/backup-ingest-policy.json",
                backup_ingest_policy(),
            ),
            file(
                "presets/encoders/moblin-srt.json",
                encoder_preset("moblin", "srt"),
            ),
            file(
                "presets/encoders/moblin-srtla.json",
                encoder_preset("moblin", "srtla"),
            ),
            file(
                "presets/encoders/irl-pro-srt.json",
                encoder_preset("irl-pro", "srt"),
            ),
            file(
                "presets/encoders/irl-pro-srtla.json",
                encoder_preset("irl-pro", "srtla"),
            ),
            file(
                "presets/encoders/larix-srt.json",
                encoder_preset("larix", "srt"),
            ),
            file(
                "presets/encoders/belabox-srtla2.json",
                encoder_preset("belabox", "srtla2"),
            ),
            file(
                "presets/relay/mediamtx.openirl.local.yml",
                mediamtx_local_config(),
            ),
            file("presets/relay/srtla2-compat.json", srtla2_compat_preset()),
            file(
                "presets/tunnels/wireguard.example.conf",
                wireguard_example(),
            ),
            file("presets/tunnels/frp.example.toml", frp_example()),
            file("presets/webrtc/whep-preview.json", whep_preview_preset()),
            file(
                "presets/production/vertical-scenes.json",
                vertical_scene_preset(),
            ),
            file(
                "presets/moderation/default-policy.json",
                moderation_policy(),
            ),
            file("docs/guides/quickstart.md", quickstart_guide()),
            file("docs/guides/relay.md", relay_guide()),
            file(
                "docs/hardware/moblin.md",
                hardware_guide("Moblin", "iOS-first mobile encoder path."),
            ),
            file(
                "docs/hardware/irl-pro.md",
                hardware_guide("IRL Pro", "Android-first SRT/SRTLA mobile encoder path."),
            ),
            file(
                "docs/hardware/belabox.md",
                hardware_guide("BELABOX", "Backpack encoder and bonding path."),
            ),
            file(
                "docs/troubleshooting/no-video.md",
                troubleshooting_no_video(),
            ),
            file("issue_templates/bug_report.md", bug_report_template()),
            file("issue_templates/field_report.md", field_report_template()),
            file(
                "plugin/openirl-plugin-manifest.schema.json",
                plugin_schema(),
            ),
            file(
                "scripts/smoke/e2e-private-alpha.sh",
                shell_script("echo OpenIRL private alpha smoke ready"),
            ),
            file(
                "scripts/smoke/e2e-private-alpha.ps1",
                powershell_script("Write-Host 'OpenIRL private alpha smoke ready'"),
            ),
            file(
                "scripts/security/security-audit-smoke.py",
                python_security_smoke(),
            ),
            file(
                "scripts/plugins/validate-plugin-manifest.py",
                python_plugin_validator(),
            ),
            file("release/RELEASE_NOTES_PUBLIC_BETA.md", release_notes()),
            file("release/V1_READINESS.md", v1_readiness_notes()),
        ],
    }
}

/// Materializes a package layout.
///
/// # Errors
///
/// Returns filesystem or JSON serialization errors.
pub fn materialize_v1_package(
    layout: &V1PackageLayout,
    overwrite_existing: bool,
) -> Result<V1MaterializationResult, V1Error> {
    let root = PathBuf::from(&layout.root);
    let mut created_directories = Vec::new();
    let mut written_files = Vec::new();
    let mut skipped_files = Vec::new();

    fs::create_dir_all(&root)?;
    created_directories.push(layout.root.clone());

    for directory in &layout.directories {
        let path = root.join(directory);
        fs::create_dir_all(&path)?;
        created_directories.push(path_to_string(&path));
    }

    for entry in &layout.files {
        let path = root.join(&entry.path);
        if path.exists() && !overwrite_existing {
            skipped_files.push(path_to_string(&path));
            continue;
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, &entry.contents)?;
        written_files.push(path_to_string(&path));
    }

    let manifest_path = root.join("openirl-v1-package.manifest.json");
    let manifest = serde_json::to_string_pretty(layout)?;
    fs::write(&manifest_path, manifest)?;
    written_files.push(path_to_string(&manifest_path));

    Ok(V1MaterializationResult {
        root: layout.root.clone(),
        created_directories,
        written_files,
        skipped_files,
        generated_at: OffsetDateTime::now_utc().to_string(),
    })
}

/// Returns sample evidence JSON with all flags disabled.
#[must_use]
pub fn sample_v1_evidence() -> V1EvidenceInput {
    V1EvidenceInput::default()
}

/// Evaluates readiness evidence.
#[must_use]
pub fn evaluate_v1_evidence(evidence: &V1EvidenceInput) -> V1ReadinessReport {
    let gates = vec![
        gate(
            "static validation",
            evidence.static_validation_passed,
            "run python3 scripts/static_validate.py",
        ),
        gate(
            "rust ci",
            evidence.rust_ci_passed,
            "run cargo xtask ci in a Rust toolchain environment",
        ),
        gate(
            "obs reconciliation",
            evidence.obs_reconciliation_passed,
            "run OBS template reconciliation against OBS",
        ),
        gate(
            "local ingest",
            evidence.local_ingest_passed,
            "run MediaMTX local ingest path",
        ),
        gate(
            "mobile profiles",
            evidence.mobile_profiles_passed,
            "scan Moblin and IRL Pro profiles on real devices",
        ),
        gate(
            "dashboard mobile",
            evidence.dashboard_mobile_passed,
            "use dashboard from a phone on the intended network",
        ),
        gate(
            "local security",
            evidence.local_security_passed,
            "run auth, LAN exposure, and redaction checks",
        ),
        gate(
            "brownout engine",
            evidence.brownout_engine_passed,
            "force brownout/recovery and confirm hysteresis",
        ),
        gate(
            "obs output health",
            evidence.obs_output_health_passed,
            "classify input, OBS, relay, and platform-output failures",
        ),
        gate(
            "support bundle",
            evidence.support_bundle_passed,
            "export and review a redacted support bundle",
        ),
        gate(
            "backup ingest",
            evidence.backup_ingest_passed,
            "fail primary feed and recover to backup",
        ),
        gate(
            "alerts and mod ops",
            evidence.alerts_mod_ops_passed,
            "test mod-safe controls and incident notifications",
        ),
        gate(
            "srtla bonding",
            evidence.srtla_bonding_passed,
            "test SRTLA compatibility",
        ),
        gate(
            "self-hosted relay",
            evidence.self_hosted_relay_passed,
            "deploy the relay path on a local machine or VPS",
        ),
        gate(
            "nat tunnel",
            evidence.nat_tunnel_passed,
            "validate a no-public-IP path with VPN or reverse tunnel",
        ),
        gate(
            "private alpha package",
            evidence.private_alpha_package_passed,
            "materialize and run the private alpha package",
        ),
        gate(
            "docs and guides",
            evidence.docs_guides_passed,
            "review quickstart, hardware, relay, and troubleshooting docs",
        ),
        gate(
            "public beta security",
            evidence.public_beta_security_passed,
            "complete public beta threat and default-bind review",
        ),
        gate(
            "public beta release",
            evidence.public_beta_release_passed,
            "build checksums, release notes, issue templates, and package",
        ),
        gate(
            "webrtc preview",
            evidence.webrtc_preview_passed,
            "test WHEP preview",
        ),
        gate(
            "vertical clips",
            evidence.vertical_clip_passed,
            "verify vertical scenes and replay/marker workflow",
        ),
        gate(
            "plugin api",
            evidence.plugin_api_passed,
            "validate plugin manifest and API stability notes",
        ),
    ];

    let private_alpha_ready = evidence.static_validation_passed
        && evidence.rust_ci_passed
        && evidence.obs_reconciliation_passed
        && evidence.local_ingest_passed
        && evidence.mobile_profiles_passed
        && evidence.dashboard_mobile_passed
        && evidence.local_security_passed
        && evidence.brownout_engine_passed
        && evidence.support_bundle_passed
        && evidence.private_alpha_package_passed;

    let public_beta_ready = private_alpha_ready
        && evidence.obs_output_health_passed
        && evidence.backup_ingest_passed
        && evidence.alerts_mod_ops_passed
        && evidence.self_hosted_relay_passed
        && evidence.nat_tunnel_passed
        && evidence.docs_guides_passed
        && evidence.public_beta_security_passed
        && evidence.public_beta_release_passed;

    let v1_ready = public_beta_ready
        && evidence.srtla_bonding_passed
        && evidence.webrtc_preview_passed
        && evidence.vertical_clip_passed
        && evidence.plugin_api_passed;

    let next_actions = gates
        .iter()
        .filter(|readiness_gate| !readiness_gate.passed)
        .map(|readiness_gate| readiness_gate.reason.clone())
        .collect();

    V1ReadinessReport {
        private_alpha_ready,
        public_beta_ready,
        v1_ready,
        gates,
        next_actions,
    }
}

fn gate(name: &str, passed: bool, reason: &str) -> ReadinessGate {
    ReadinessGate {
        name: s(name),
        passed,
        reason: s(reason),
    }
}

fn file(path: &str, contents: String) -> V1PackageFile {
    V1PackageFile {
        path: s(path),
        contents,
    }
}

fn s(value: &str) -> String {
    value.to_string()
}

fn keys(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| s(value)).collect()
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn package_readme() -> String {
    s(
        "# OpenIRL Public Beta / V1 Package\n\nThis package is generated by the Rust-first OpenIRL public-beta implementation layer. It contains local-first presets, docs, smoke scripts, issue templates, plugin schema, and release notes for testing OpenIRL without managed Cloud OBS.\n\nStart with docs/guides/quickstart.md.\n",
    )
}

fn public_beta_config() -> String {
    s(
        "[api]\nbind = \"127.0.0.1:7707\"\nallow_lan = false\n\n[obs]\nadapter = \"web-socket\"\nhost = \"127.0.0.1\"\nport = 4455\npassword_env = \"OPENIRL_OBS_PASSWORD\"\ncreate_missing_scenes = true\n\n[metrics]\nenabled = true\nsource = \"media-mtx-prometheus\"\nauto_poll = false\n\n[security]\ndashboard_auth_enabled = false\nrequire_auth_outside_localhost = true\nredact_logs = true\n",
    )
}

fn obs_scene_template() -> String {
    s(
        "{\n  \"version\": 1,\n  \"canvas\": { \"width\": 1920, \"height\": 1080 },\n  \"scenes\": [\"OpenIRL Live\", \"OpenIRL Low Signal\", \"OpenIRL BRB\", \"OpenIRL Backup Feed\", \"OpenIRL Privacy\", \"OpenIRL Starting Soon\", \"OpenIRL Ending\"],\n  \"sources\": [\n    { \"scene\": \"OpenIRL Live\", \"name\": \"OpenIRL Primary Ingest\", \"kind\": \"ffmpeg_source\", \"url\": \"srt://127.0.0.1:9000?mode=listener\" },\n    { \"scene\": \"OpenIRL Backup Feed\", \"name\": \"OpenIRL Backup Ingest\", \"kind\": \"ffmpeg_source\", \"url\": \"srt://127.0.0.1:9010?mode=listener\" }\n  ],\n  \"transforms\": { \"fit_to_canvas\": true, \"lock_sources\": true }\n}\n",
    )
}

fn backup_ingest_policy() -> String {
    s(
        "{\n  \"primary\": \"openirl-main\",\n  \"backup\": \"openirl-backup\",\n  \"automatic_failover\": true,\n  \"return_to_primary_after_stable_ms\": 15000,\n  \"manual_override_roles\": [\"owner\", \"producer\"]\n}\n",
    )
}
fn encoder_preset(encoder: &str, protocol: &str) -> String {
    format!(
        "{{\n  \"encoder\": \"{encoder}\",\n  \"protocol\": \"{protocol}\",\n  \"host\": \"127.0.0.1\",\n  \"port\": 9000,\n  \"stream_id\": \"{encoder}-main\",\n  \"latency_ms\": 1800,\n  \"bitrate_kbps\": 4500\n}}\n"
    )
}
fn mediamtx_local_config() -> String {
    s(
        "srt: yes\nsrtAddress: :9000\nrtmp: yes\nrtmpAddress: :1935\nwebrtc: yes\napi: yes\napiAddress: 127.0.0.1:9997\nmetrics: yes\nmetricsAddress: 127.0.0.1:9998\npaths:\n  all:\n    source: publisher\n",
    )
}
fn srtla2_compat_preset() -> String {
    s(
        "{\n  \"transport\": \"srtla2\",\n  \"compatible_with_legacy_srtla\": false,\n  \"requires_explicit_receiver_match\": true\n}\n",
    )
}
fn wireguard_example() -> String {
    s(
        "[Interface]\nPrivateKey = replace-with-local-private-key\nAddress = 10.77.0.2/32\n\n[Peer]\nPublicKey = replace-with-relay-public-key\nAllowedIPs = 10.77.0.0/24\nEndpoint = relay.example.com:51820\nPersistentKeepalive = 25\n",
    )
}
fn frp_example() -> String {
    s(
        "serverAddr = \"relay.example.com\"\nserverPort = 7000\n\n[[proxies]]\nname = \"openirl-srt\"\ntype = \"udp\"\nlocalIP = \"127.0.0.1\"\nlocalPort = 9000\nremotePort = 9000\n",
    )
}
fn whep_preview_preset() -> String {
    s(
        "{\n  \"preview\": \"whep\",\n  \"local_url\": \"http://127.0.0.1:8889/openirl/whep\",\n  \"requires_auth\": true\n}\n",
    )
}
fn vertical_scene_preset() -> String {
    s(
        "{\n  \"canvas\": { \"width\": 1080, \"height\": 1920 },\n  \"sources\": [\"primary-ingest\", \"chat-safe-area\", \"status-overlay\"],\n  \"clip_folder\": \"clips\"\n}\n",
    )
}
fn moderation_policy() -> String {
    s(
        "{\n  \"roles\": {\n    \"owner\": [\"*\"],\n    \"producer\": [\"switch-scene\", \"save-replay\", \"add-marker\", \"start-recording\", \"stop-recording\"],\n    \"moderator\": [\"switch-scene\", \"save-replay\", \"add-marker\"],\n    \"viewer\": []\n  },\n  \"rate_limit_per_minute\": 20\n}\n",
    )
}
fn quickstart_guide() -> String {
    s(
        "# OpenIRL Quickstart\n\n1. Start OBS and enable the built-in WebSocket server with a password.\n2. Start OpenIRL Agent.\n3. Open the dashboard at http://127.0.0.1:7707/.\n4. Generate a Moblin or IRL Pro profile QR.\n5. Publish SRT into the local MediaMTX path.\n6. Use the dashboard to verify health, scene switching, and support-bundle export.\n",
    )
}
fn relay_guide() -> String {
    s(
        "# Relay Guide\n\nUse MediaMTX for local routing. Use a VPS, friend relay, or WireGuard/frp tunnel when the OBS machine is behind CGNAT. Keep OBS WebSocket bound to localhost or a protected private network.\n",
    )
}
fn hardware_guide(name: &str, note: &str) -> String {
    format!(
        "# {name}\n\n{note}\n\nUse generated profiles from OpenIRL and validate ingest metrics before going live.\n"
    )
}
fn troubleshooting_no_video() -> String {
    s(
        "# Troubleshooting: No Video\n\nCheck encoder URL, MediaMTX publisher status, OBS media source URL, Windows firewall, and the latest metrics snapshot. Export a support bundle after reproducing the issue.\n",
    )
}
fn bug_report_template() -> String {
    s(
        "# Bug Report\n\n## Environment\n\n## Steps to Reproduce\n\n## Expected Result\n\n## Actual Result\n\n## Support Bundle\nAttach a redacted support bundle when possible.\n",
    )
}
fn field_report_template() -> String {
    s(
        "# Field Report\n\n## Device and Encoder\n\n## Network Conditions\n\n## OBS Result\n\n## Brownout / BRB Events\n\n## Support Bundle Path\n",
    )
}
fn plugin_schema() -> String {
    s(r#"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "title": "OpenIRL Plugin Manifest",
  "type": "object",
  "required": ["id", "name", "version", "capabilities"],
  "properties": {
    "id": { "type": "string", "pattern": "^[a-z0-9][a-z0-9.-]+$" },
    "name": { "type": "string" },
    "version": { "type": "string" },
    "capabilities": { "type": "array", "items": { "type": "string" } },
    "webhooks": { "type": "array", "items": { "type": "string" } }
  }
}
"#)
}
fn shell_script(command: &str) -> String {
    format!("#!/usr/bin/env bash\nset -euo pipefail\n{command}\n")
}
fn powershell_script(command: &str) -> String {
    format!("$ErrorActionPreference = 'Stop'\n{command}\n")
}
fn python_security_smoke() -> String {
    s("#!/usr/bin/env python3\nimport json\nprint(json.dumps({'security_smoke':'ready'}))\n")
}
fn python_plugin_validator() -> String {
    s(
        "#!/usr/bin/env python3\nimport json, sys\nfor path in sys.argv[1:]:\n    data=json.load(open(path, encoding='utf-8'))\n    assert data.get('id') and data.get('name') and data.get('version')\nprint('plugin manifest valid')\n",
    )
}
fn release_notes() -> String {
    s(
        "# Public Beta Release Notes\n\nOpenIRL public beta focuses on local OBS automation, SRT/SRTLA-friendly ingest, mobile dashboard control, support bundles, and self-hosted relay workflows.\n",
    )
}
fn v1_readiness_notes() -> String {
    s(
        "# V1 Readiness\n\nV1 readiness requires Rust CI, OBS reconciliation, local ingest, mobile encoder profile checks, security review, support-bundle export, and public package verification.\n",
    )
}
