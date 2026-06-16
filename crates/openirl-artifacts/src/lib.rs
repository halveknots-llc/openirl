//! Disk artifacts for fallback assets, OBS templates, support bundles, and alpha packages.
//!
//! This crate intentionally writes plain files instead of talking directly to OBS
//! or external media tools. The agent can expose these plans to operators, then a
//! future desktop shell can apply them with a safer confirmation flow.

use openirl_core::{SceneBundle, SceneRole};
use openirl_diagnostics::StreamReport;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error;
use time::OffsetDateTime;
use uuid::Uuid;

/// Artifact writer errors.
#[derive(Debug, Error)]
pub enum ArtifactError {
    /// Filesystem error.
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
    /// JSON error.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    /// Invalid input.
    #[error("invalid artifact input: {0}")]
    Invalid(String),
}

/// A file planned or written by the artifact layer.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ArtifactFile {
    /// File path as a portable string.
    pub path: String,
    /// Media or content type.
    pub media_type: String,
    /// File size in bytes.
    pub bytes: usize,
    /// SHA-256 hex digest of the content.
    pub sha256: String,
    /// Human-readable purpose.
    pub purpose: String,
}

/// Materialization result.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ArtifactMaterialization {
    /// Root directory used.
    pub root_dir: String,
    /// Files written.
    pub files: Vec<ArtifactFile>,
    /// Non-blocking warnings.
    pub warnings: Vec<String>,
}

/// One fallback/scene asset specification.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct FallbackAssetSpec {
    /// Scene role.
    pub role: SceneRole,
    /// Relative file name.
    pub file_name: String,
    /// Media type.
    pub media_type: String,
    /// Recommended OBS source/input kind.
    pub obs_input_kind: String,
    /// Rendered content.
    pub content: String,
    /// Human-readable title.
    pub title: String,
}

/// Fallback asset plan.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct FallbackAssetPlan {
    /// Root directory.
    pub root_dir: String,
    /// Assets to write.
    pub assets: Vec<FallbackAssetSpec>,
    /// Notes for operators.
    pub notes: Vec<String>,
}

/// OBS input template.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObsInputTemplate {
    /// Scene role receiving the input.
    pub role: SceneRole,
    /// OBS scene name.
    pub scene_name: String,
    /// OBS input/source name.
    pub input_name: String,
    /// OBS input kind, for example `ffmpeg_source` or `browser_source`.
    pub input_kind: String,
    /// OBS input settings object.
    pub settings: Value,
    /// Whether to enable the input on creation.
    pub enabled: bool,
}

/// OBS scene/source template plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObsSceneTemplatePlan {
    /// Plan creation time.
    pub generated_at: OffsetDateTime,
    /// Scene bundle.
    pub scene_bundle: SceneBundle,
    /// Live ingest URL intended for the main Media Source.
    pub live_input_url: String,
    /// Root directory that contains fallback HTML assets.
    pub asset_root_dir: String,
    /// Inputs to create or update.
    pub inputs: Vec<ObsInputTemplate>,
    /// OBS WebSocket request-shaped preview.
    pub obs_websocket_requests: Vec<Value>,
    /// Warnings.
    pub warnings: Vec<String>,
}

/// Support bundle export request.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct SupportBundleExportRequest {
    /// Root directory for support bundles.
    pub output_dir: String,
    /// Optional field report markdown to include.
    pub field_report_markdown: Option<String>,
}

/// Support bundle export result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SupportBundleExport {
    /// Bundle identifier.
    pub bundle_id: Uuid,
    /// Bundle root directory.
    pub root_dir: String,
    /// Files written.
    pub files: Vec<ArtifactFile>,
    /// Report included in the bundle.
    pub report: Option<StreamReport>,
    /// Creation time.
    pub generated_at: OffsetDateTime,
}

/// Alpha source package layout plan.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct AlphaSourcePackageLayout {
    /// Package root directory.
    pub root_dir: String,
    /// Required directories.
    pub directories: Vec<String>,
    /// Sample files to materialize.
    pub files: Vec<AlphaPackageFile>,
    /// Operator instructions.
    pub instructions: Vec<String>,
}

/// Alpha source package sample file.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct AlphaPackageFile {
    /// Relative file path.
    pub relative_path: String,
    /// File content.
    pub content: String,
    /// Purpose.
    pub purpose: String,
}

/// Builds the default fallback asset plan.
#[must_use]
pub fn default_fallback_asset_plan(
    bundle: &SceneBundle,
    root_dir: impl Into<String>,
) -> FallbackAssetPlan {
    let root_dir = root_dir.into();
    let roles = [
        (
            SceneRole::StartingSoon,
            "starting-soon.html",
            "Starting Soon",
            "Preparing the IRL route",
        ),
        (
            SceneRole::LowSignal,
            "low-signal.html",
            "Signal Unstable",
            "Holding live while connection recovers",
        ),
        (
            SceneRole::Brb,
            "brb.html",
            "Be Right Back",
            "Mobile signal dropped; fallback is active",
        ),
        (
            SceneRole::BackupFeed,
            "backup-feed.html",
            "Backup Feed",
            "Waiting for backup camera or backpack encoder",
        ),
        (
            SceneRole::Privacy,
            "privacy.html",
            "Privacy Mode",
            "Live view hidden by operator",
        ),
        (
            SceneRole::Ending,
            "ending.html",
            "Ending Stream",
            "Thanks for watching",
        ),
    ];

    let assets = roles
        .into_iter()
        .map(|(role, file_name, title, subtitle)| FallbackAssetSpec {
            role,
            file_name: file_name.to_string(),
            media_type: "text/html; charset=utf-8".to_string(),
            obs_input_kind: "browser_source".to_string(),
            content: fallback_html(title, subtitle, scene_name(bundle, role)),
            title: title.to_string(),
        })
        .collect();

    FallbackAssetPlan {
        root_dir,
        assets,
        notes: vec![
            "HTML cards are local browser-source assets so the alpha package does not depend on bundled font/video files.".to_string(),
            "Replace these samples with streamer-branded assets before a public release.".to_string(),
        ],
    }
}

/// Materializes fallback assets to disk.
pub fn materialize_fallback_assets(
    plan: &FallbackAssetPlan,
) -> Result<ArtifactMaterialization, ArtifactError> {
    let root = PathBuf::from(&plan.root_dir);
    fs::create_dir_all(&root)?;
    let mut files = Vec::new();
    for asset in &plan.assets {
        let path = root.join(&asset.file_name);
        files.push(write_text_artifact(
            &path,
            &asset.content,
            &asset.media_type,
            format!("fallback asset for {}", asset.role),
        )?);
    }
    let manifest = json!({
        "generated_at": OffsetDateTime::now_utc(),
        "assets": plan.assets.clone(),
        "notes": plan.notes.clone(),
    });
    files.push(write_json_artifact(
        root.join("fallback-assets.manifest.json"),
        &manifest,
        "fallback asset manifest",
    )?);
    Ok(ArtifactMaterialization {
        root_dir: path_to_string(&root),
        files,
        warnings: plan.notes.clone(),
    })
}

/// Builds an OBS scene/source template plan.
#[must_use]
pub fn build_obs_scene_template_plan(
    bundle: &SceneBundle,
    asset_root_dir: impl Into<String>,
    live_input_url: impl Into<String>,
) -> ObsSceneTemplatePlan {
    let asset_root_dir = asset_root_dir.into();
    let live_input_url = live_input_url.into();
    let mut inputs = Vec::new();

    if let Some(scene_name) = bundle.scene_name(SceneRole::Live) {
        inputs.push(ObsInputTemplate {
            role: SceneRole::Live,
            scene_name: scene_name.to_string(),
            input_name: "OpenIRL Main Ingest".to_string(),
            input_kind: "ffmpeg_source".to_string(),
            settings: json!({
                "is_local_file": false,
                "input": live_input_url,
                "reconnect_delay_sec": 1,
                "close_when_inactive": false,
            }),
            enabled: true,
        });
    }

    for asset in default_fallback_asset_plan(bundle, &asset_root_dir).assets {
        if let Some(scene_name) = bundle.scene_name(asset.role) {
            let asset_path = PathBuf::from(&asset_root_dir).join(&asset.file_name);
            inputs.push(ObsInputTemplate {
                role: asset.role,
                scene_name: scene_name.to_string(),
                input_name: format!("OpenIRL {} Card", asset.title),
                input_kind: asset.obs_input_kind,
                settings: json!({
                    "url": file_url(&path_to_string(&asset_path)),
                    "width": 1920,
                    "height": 1080,
                    "reroute_audio": false,
                    "shutdown": false,
                }),
                enabled: true,
            });
        }
    }

    let obs_websocket_requests = build_obs_requests(bundle, &inputs);
    ObsSceneTemplatePlan {
        generated_at: OffsetDateTime::now_utc(),
        scene_bundle: bundle.clone(),
        live_input_url,
        asset_root_dir,
        inputs,
        obs_websocket_requests,
        warnings: vec![
            "feature areas materializes request-shaped templates; applying source transforms is deferred to live OBS smoke validation.".to_string(),
            "Existing OBS inputs with the same names should be reviewed before applying this template on a production profile.".to_string(),
        ],
    }
}

/// Materializes the OBS template plan to JSON.
pub fn materialize_obs_scene_template(
    output_path: impl AsRef<Path>,
    plan: &ObsSceneTemplatePlan,
) -> Result<ArtifactFile, ArtifactError> {
    write_json_artifact(
        output_path.as_ref(),
        &json!(plan),
        "OBS scene/source template",
    )
}

/// Exports a disk-based support bundle.
pub fn export_support_bundle(
    request: &SupportBundleExportRequest,
    payload: &Value,
    report: Option<StreamReport>,
) -> Result<SupportBundleExport, ArtifactError> {
    if request.output_dir.trim().is_empty() {
        return Err(ArtifactError::Invalid(
            "support bundle output_dir is empty".to_string(),
        ));
    }

    let bundle_id = Uuid::new_v4();
    let root = PathBuf::from(&request.output_dir).join(bundle_id.to_string());
    fs::create_dir_all(&root)?;
    let mut files = Vec::new();
    files.push(write_json_artifact(
        root.join("support-bundle.json"),
        payload,
        "complete redacted support bundle payload",
    )?);
    if let Some(report) = &report {
        files.push(write_json_artifact(
            root.join("session-report.json"),
            &json!(report),
            "session health report",
        )?);
    }
    if let Some(field_report) = request.field_report_markdown.as_deref() {
        files.push(write_text_artifact(
            root.join("field-report.md"),
            field_report,
            "text/markdown; charset=utf-8",
            "operator field report",
        )?);
    }
    files.push(write_text_artifact(
        root.join("README.md"),
        support_bundle_readme(bundle_id),
        "text/markdown; charset=utf-8",
        "support bundle readme",
    )?);
    Ok(SupportBundleExport {
        bundle_id,
        root_dir: path_to_string(&root),
        files,
        report,
        generated_at: OffsetDateTime::now_utc(),
    })
}

/// Exports a standalone field report markdown artifact.
pub fn export_field_report_markdown(
    output_dir: impl AsRef<Path>,
    markdown: impl AsRef<str>,
) -> Result<ArtifactMaterialization, ArtifactError> {
    let root = output_dir.as_ref();
    fs::create_dir_all(root)?;
    let report_id = Uuid::new_v4();
    let report_path = root.join(format!("field-report-{report_id}.md"));
    let mut files = Vec::new();
    files.push(write_text_artifact(
        &report_path,
        markdown.as_ref(),
        "text/markdown; charset=utf-8",
        "standalone field report",
    )?);
    let manifest = json!({
        "generated_at": OffsetDateTime::now_utc(),
        "report_id": report_id,
        "report_path": path_to_string(&report_path),
    });
    files.push(write_json_artifact(
        root.join(format!("field-report-{report_id}.manifest.json")),
        &manifest,
        "field report manifest",
    )?);
    Ok(ArtifactMaterialization {
        root_dir: path_to_string(root),
        files,
        warnings: vec![
            "Review field reports for location-adjacent information before sharing.".to_string(),
        ],
    })
}

/// Builds an alpha source package layout plan.
#[must_use]
pub fn alpha_source_package_layout(root_dir: impl Into<String>) -> AlphaSourcePackageLayout {
    let root_dir = root_dir.into();
    let directories = vec![
        "bin".to_string(),
        "config".to_string(),
        "assets/fallback".to_string(),
        "obs-templates".to_string(),
        "support-bundles".to_string(),
        "field-reports".to_string(),
        "logs".to_string(),
    ];
    let files = vec![
        AlphaPackageFile {
            relative_path: "README.alpha.md".to_string(),
            purpose: "alpha source package operator entrypoint".to_string(),
            content: "# OpenIRL Alpha Source Package\n\nRun `openirl-agent serve --config config/openirl.example.toml`, open the local dashboard, materialize fallback assets, then run the OBS/mobile field smoke tests.\n".to_string(),
        },
        AlphaPackageFile {
            relative_path: "field-reports/README.md".to_string(),
            purpose: "field report folder guide".to_string(),
            content: "# Field Reports\n\nStore redacted Moblin, IRL Pro, BELABOX, MediaMTX, OBS, brownout, BRB, and recovery notes here.\n".to_string(),
        },
        AlphaPackageFile {
            relative_path: "support-bundles/README.md".to_string(),
            purpose: "support bundle folder guide".to_string(),
            content: "# Support Bundles\n\nOpenIRL disk exports should be redacted before sharing. Do not paste stream keys or private location notes.\n".to_string(),
        },
    ];
    AlphaSourcePackageLayout {
        root_dir,
        directories,
        files,
        instructions: vec![
            "Copy release binaries into bin/ after cargo release builds pass.".to_string(),
            "Copy config/openirl.example.toml into config/ and keep secrets in environment variables.".to_string(),
            "Generate fallback assets before starting a real OBS smoke test.".to_string(),
        ],
    }
}

/// Materializes an alpha source package layout.
pub fn materialize_alpha_source_layout(
    layout: &AlphaSourcePackageLayout,
) -> Result<ArtifactMaterialization, ArtifactError> {
    let root = PathBuf::from(&layout.root_dir);
    fs::create_dir_all(&root)?;
    for directory in &layout.directories {
        fs::create_dir_all(root.join(directory))?;
    }

    let mut files = Vec::new();
    for file in &layout.files {
        let path = root.join(&file.relative_path);
        files.push(write_text_artifact(
            &path,
            &file.content,
            "text/markdown; charset=utf-8",
            file.purpose.clone(),
        )?);
    }
    let manifest = json!({
        "generated_at": OffsetDateTime::now_utc(),
        "layout": layout.clone(),
    });
    files.push(write_json_artifact(
        root.join("openirl-alpha-layout.manifest.json"),
        &manifest,
        "alpha source package layout manifest",
    )?);
    Ok(ArtifactMaterialization {
        root_dir: path_to_string(&root),
        files,
        warnings: layout.instructions.clone(),
    })
}

fn build_obs_requests(bundle: &SceneBundle, inputs: &[ObsInputTemplate]) -> Vec<Value> {
    let mut requests = Vec::new();
    for scene in &bundle.scenes {
        requests.push(json!({
            "requestType": "CreateScene",
            "requestData": { "sceneName": scene.name.clone() },
        }));
    }
    for input in inputs {
        requests.push(json!({
            "requestType": "CreateInput",
            "requestData": {
                "sceneName": input.scene_name.clone(),
                "inputName": input.input_name.clone(),
                "inputKind": input.input_kind.clone(),
                "inputSettings": input.settings.clone(),
                "sceneItemEnabled": input.enabled,
            },
        }));
    }
    requests
}

fn write_json_artifact(
    path: impl AsRef<Path>,
    value: &Value,
    purpose: impl Into<String>,
) -> Result<ArtifactFile, ArtifactError> {
    let text = serde_json::to_string_pretty(value)?;
    write_text_artifact(path, &text, "application/json", purpose)
}

fn write_text_artifact(
    path: impl AsRef<Path>,
    content: impl AsRef<str>,
    media_type: impl Into<String>,
    purpose: impl Into<String>,
) -> Result<ArtifactFile, ArtifactError> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let text = content.as_ref();
    fs::write(path, text.as_bytes())?;
    Ok(ArtifactFile {
        path: path_to_string(path),
        media_type: media_type.into(),
        bytes: text.len(),
        sha256: sha256_hex(text.as_bytes()),
        purpose: purpose.into(),
    })
}

fn fallback_html(title: &str, subtitle: &str, scene_name: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>{}</title>
    <style>
      html, body {{ margin: 0; width: 100%; height: 100%; background: #05070b; color: #f4f7fb; font-family: Inter, system-ui, sans-serif; }}
      main {{ width: 100vw; height: 100vh; display: grid; place-items: center; text-align: center; }}
      section {{ max-width: 1100px; padding: 64px; border: 2px solid rgba(255,255,255,.16); border-radius: 32px; background: linear-gradient(145deg, rgba(255,255,255,.08), rgba(255,255,255,.02)); }}
      h1 {{ margin: 0 0 24px; font-size: 92px; letter-spacing: -0.06em; }}
      p {{ margin: 16px 0; font-size: 34px; opacity: .86; }}
      small {{ display: block; margin-top: 48px; font-size: 22px; opacity: .52; }}
    </style>
  </head>
  <body>
    <main>
      <section>
        <h1>{}</h1>
        <p>{}</p>
        <small>OpenIRL scene: {}</small>
      </section>
    </main>
  </body>
</html>
"#,
        escape_html(title),
        escape_html(title),
        escape_html(subtitle),
        escape_html(scene_name),
    )
}

fn support_bundle_readme(bundle_id: Uuid) -> String {
    format!(
        "# OpenIRL Support Bundle\n\nBundle ID: `{bundle_id}`\n\nThis folder is intended for redacted diagnostics only. Review `support-bundle.json`, remove location-adjacent notes, stream keys, tokens, and private relay addresses before sharing.\n"
    )
}

fn scene_name(bundle: &SceneBundle, role: SceneRole) -> &str {
    bundle.scene_name(role).unwrap_or("OpenIRL Scene")
}

fn file_url(path: &str) -> String {
    if path.starts_with("file://") || path.starts_with("http://") || path.starts_with("https://") {
        return path.to_string();
    }
    format!("file:///{}", path.replace('\\', "/"))
}

fn path_to_string(path: impl AsRef<Path>) -> String {
    path.as_ref().to_string_lossy().replace('\\', "/")
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn escape_html(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for character in input.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_plan_has_brb_asset() {
        let bundle = SceneBundle::default_irl();
        let plan = default_fallback_asset_plan(&bundle, "artifacts/assets/fallback");
        assert!(plan.assets.iter().any(|asset| asset.role == SceneRole::Brb));
    }

    #[test]
    fn obs_template_has_live_source() {
        let bundle = SceneBundle::default_irl();
        let plan = build_obs_scene_template_plan(
            &bundle,
            "artifacts/assets/fallback",
            "srt://127.0.0.1:9000?mode=listener",
        );
        assert!(
            plan.inputs
                .iter()
                .any(|input| input.role == SceneRole::Live)
        );
    }
}
