//! OBS scene/source templates and fallback asset generation for OpenIRL.
//!
//! This crate owns the portable plan for turning a semantic IRL scene bundle into
//! real OBS sources plus a local fallback-asset folder. The OBS adapter executes
//! the source templates; this crate keeps those templates deterministic and easy
//! to inspect in support bundles.

use openirl_core::{SceneBundle, SceneRole};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use thiserror::Error;

/// Errors returned by scene-template and fallback-asset generation.
#[derive(Debug, Error)]
pub enum SceneTemplateError {
    /// Filesystem error.
    #[error("filesystem error: {0}")]
    Io(#[from] std::io::Error),
    /// Serialization error.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    /// Invalid input.
    #[error("invalid scene-template input: {0}")]
    Invalid(String),
}

/// Kind of fallback asset written to disk.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FallbackAssetKind {
    /// SVG fallback visual.
    Svg,
    /// Browser-source HTML helper.
    Html,
    /// Markdown/documentation.
    Markdown,
    /// JSON manifest.
    Json,
}

/// One local fallback asset.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct FallbackAsset {
    /// Related scene role, when the asset maps to a specific scene.
    pub role: Option<SceneRole>,
    /// Asset kind.
    pub kind: FallbackAssetKind,
    /// Relative path below the configured asset root.
    pub relative_path: String,
    /// MIME/content type.
    pub content_type: String,
    /// Human-readable purpose.
    pub purpose: String,
    /// File contents.
    pub contents: String,
}

/// Asset-generation request.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct FallbackAssetPlan {
    /// Asset root directory.
    pub root_dir: String,
    /// Whether existing files may be overwritten.
    pub overwrite_existing: bool,
    /// Assets to materialize.
    pub assets: Vec<FallbackAsset>,
}

/// One written asset.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct WrittenAsset {
    /// Relative path from asset root.
    pub relative_path: String,
    /// Absolute or process-relative path written on disk.
    pub path: String,
    /// Whether the file was written during this run.
    pub written: bool,
    /// Byte count of the generated content.
    pub bytes: usize,
}

/// Asset write report.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct AssetWriteReport {
    /// Asset root directory.
    pub root_dir: String,
    /// Written/skipped files.
    pub files: Vec<WrittenAsset>,
    /// Count of skipped files.
    pub skipped_count: usize,
    /// Manifest path.
    pub manifest_path: String,
}

/// OBS input/source kind used by the templates.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ObsSourceKind {
    /// OBS media source, usually `ffmpeg_source`.
    MediaSource,
    /// OBS browser source.
    BrowserSource,
    /// OBS image source.
    ImageSource,
    /// OBS text source.
    TextSource,
}

/// One OBS source template.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObsSourceTemplate {
    /// Target scene role.
    pub scene_role: SceneRole,
    /// Target OBS scene name.
    pub scene_name: String,
    /// Desired source/input name.
    pub input_name: String,
    /// Logical source kind.
    pub source_kind: ObsSourceKind,
    /// OBS input kind string.
    pub input_kind: String,
    /// OBS input settings payload.
    pub input_settings: Value,
    /// Whether the scene item should be enabled at creation.
    pub scene_item_enabled: bool,
    /// Human-readable purpose.
    pub purpose: String,
}

/// End-to-end scene materialization plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneMaterializationPlan {
    /// Scene bundle.
    pub scenes: SceneBundle,
    /// Fallback asset plan.
    pub assets: FallbackAssetPlan,
    /// OBS source templates.
    pub sources: Vec<ObsSourceTemplate>,
    /// Non-blocking notes.
    pub notes: Vec<String>,
}

/// Input for materialization planning.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct SceneTemplateRequest {
    /// Asset root directory.
    pub asset_root_dir: String,
    /// SRT listener host shown to OBS locally.
    pub local_srt_host: String,
    /// SRT listener port.
    pub srt_port: u16,
    /// Backup SRT listener port.
    pub backup_srt_port: u16,
    /// RTMP listener port.
    pub rtmp_port: u16,
    /// Contribution latency in milliseconds.
    pub latency_ms: u32,
    /// Whether to overwrite existing fallback assets.
    pub overwrite_existing_assets: bool,
}

impl Default for SceneTemplateRequest {
    fn default() -> Self {
        Self {
            asset_root_dir: "artifacts/assets".to_string(),
            local_srt_host: "127.0.0.1".to_string(),
            srt_port: 9000,
            backup_srt_port: 9002,
            rtmp_port: 1935,
            latency_ms: 1800,
            overwrite_existing_assets: false,
        }
    }
}

/// Builds the default scene materialization plan.
#[must_use]
pub fn build_scene_materialization_plan(
    scenes: &SceneBundle,
    request: &SceneTemplateRequest,
) -> SceneMaterializationPlan {
    let assets =
        default_fallback_asset_plan(&request.asset_root_dir, request.overwrite_existing_assets);
    let sources = default_obs_source_templates(scenes, request);
    SceneMaterializationPlan {
        scenes: scenes.clone(),
        assets,
        sources,
        notes: vec![
            "Media sources are created through OBS WebSocket CreateInput and updated with SetInputSettings when they already exist.".to_string(),
            "Fallback visuals are local SVG/HTML files so the stream can keep running without a hosted asset service.".to_string(),
            "Live and backup inputs are listener-style SRT media sources by default; adapt the profile generator and MediaMTX route during field tests.".to_string(),
        ],
    }
}

/// Builds default fallback assets.
#[must_use]
pub fn default_fallback_asset_plan(
    root_dir: impl Into<String>,
    overwrite_existing: bool,
) -> FallbackAssetPlan {
    FallbackAssetPlan {
        root_dir: root_dir.into(),
        overwrite_existing,
        assets: vec![
            svg_asset(
                Some(SceneRole::Live),
                "live-sample.svg",
                "LIVE SOURCE",
                "Waiting for primary ingest",
                "Use this only as a dry-run sample.",
            ),
            svg_asset(
                Some(SceneRole::LowSignal),
                "low-signal.svg",
                "SIGNAL DEGRADED",
                "Holding the stream while the mobile link recovers",
                "Low-motion fallback for brownouts.",
            ),
            svg_asset(
                Some(SceneRole::Brb),
                "brb.svg",
                "BRB",
                "The IRL feed will return automatically",
                "Main fallback scene.",
            ),
            svg_asset(
                Some(SceneRole::BackupFeed),
                "backup-feed.svg",
                "BACKUP FEED",
                "Switching to backup ingest",
                "Backup-feed sample.",
            ),
            svg_asset(
                Some(SceneRole::Privacy),
                "privacy.svg",
                "PRIVACY MODE",
                "Camera and location are hidden",
                "Panic/privacy fallback.",
            ),
            svg_asset(
                Some(SceneRole::StartingSoon),
                "starting-soon.svg",
                "STARTING SOON",
                "OpenIRL is preparing the IRL link",
                "Pre-stream fallback.",
            ),
            svg_asset(
                Some(SceneRole::Ending),
                "ending.svg",
                "STREAM ENDING",
                "Thanks for watching",
                "Ending fallback.",
            ),
            html_asset("browser/fallback.html"),
            markdown_asset("README.md"),
        ],
    }
}

/// Writes fallback assets to disk and returns a report.
pub fn write_fallback_assets(
    plan: &FallbackAssetPlan,
) -> Result<AssetWriteReport, SceneTemplateError> {
    let root = PathBuf::from(plan.root_dir.trim());
    if root.as_os_str().is_empty() {
        return Err(SceneTemplateError::Invalid(
            "asset root cannot be empty".to_string(),
        ));
    }

    fs::create_dir_all(&root)?;
    let mut files = Vec::new();
    let mut skipped_count = 0usize;

    for asset in &plan.assets {
        let destination = safe_join(&root, &asset.relative_path)?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        let exists = destination.exists();
        let should_write = plan.overwrite_existing || !exists;
        if should_write {
            let mut file = fs::File::create(&destination)?;
            file.write_all(asset.contents.as_bytes())?;
        } else {
            skipped_count = skipped_count.saturating_add(1);
        }
        files.push(WrittenAsset {
            relative_path: asset.relative_path.clone(),
            path: destination.display().to_string(),
            written: should_write,
            bytes: asset.contents.len(),
        });
    }

    let manifest_path = root.join("manifest.json");
    let manifest = asset_manifest(plan);
    if plan.overwrite_existing || !manifest_path.exists() {
        fs::write(&manifest_path, serde_json::to_string_pretty(&manifest)?)?;
    } else {
        skipped_count = skipped_count.saturating_add(1);
    }

    Ok(AssetWriteReport {
        root_dir: root.display().to_string(),
        files,
        skipped_count,
        manifest_path: manifest_path.display().to_string(),
    })
}

/// Builds default OBS source templates for a scene bundle.
#[must_use]
pub fn default_obs_source_templates(
    scenes: &SceneBundle,
    request: &SceneTemplateRequest,
) -> Vec<ObsSourceTemplate> {
    let mut templates = Vec::new();
    if let Some(scene_name) = scenes.scene_name(SceneRole::Live) {
        templates.push(media_source(
            SceneRole::Live,
            scene_name,
            "OpenIRL Primary SRT Ingest",
            &srt_listener_url(
                &request.local_srt_host,
                request.srt_port,
                request.latency_ms,
            ),
            "Primary phone/BELABOX SRT contribution feed.",
        ));
    }
    if let Some(scene_name) = scenes.scene_name(SceneRole::BackupFeed) {
        templates.push(media_source(
            SceneRole::BackupFeed,
            scene_name,
            "OpenIRL Backup SRT Ingest",
            &srt_listener_url(
                &request.local_srt_host,
                request.backup_srt_port,
                request.latency_ms,
            ),
            "Backup phone or backpack SRT contribution feed.",
        ));
    }

    for role in [
        SceneRole::LowSignal,
        SceneRole::Brb,
        SceneRole::Privacy,
        SceneRole::StartingSoon,
        SceneRole::Ending,
    ] {
        if let Some(scene_name) = scenes.scene_name(role) {
            templates.push(browser_source(
                role,
                scene_name,
                format!("OpenIRL {role} Asset"),
                asset_file_uri(&request.asset_root_dir, role_asset_file(role)),
                format!("Local fallback visual for {role} state."),
            ));
        }
    }

    if let Some(scene_name) = scenes.scene_name(SceneRole::Live) {
        templates.push(browser_source(
            SceneRole::Live,
            scene_name,
            "OpenIRL Health Overlay",
            asset_file_uri(&request.asset_root_dir, "browser/fallback.html"),
            "Local browser overlay sample for status/warnings.",
        ));
    }

    templates
}

fn media_source(
    role: SceneRole,
    scene_name: &str,
    input_name: &str,
    input_url: &str,
    purpose: &str,
) -> ObsSourceTemplate {
    ObsSourceTemplate {
        scene_role: role,
        scene_name: scene_name.to_string(),
        input_name: input_name.to_string(),
        source_kind: ObsSourceKind::MediaSource,
        input_kind: "ffmpeg_source".to_string(),
        input_settings: json!({
            "input": input_url,
            "is_local_file": false,
            "restart_on_activate": true,
            "close_when_inactive": true,
            "hw_decode": true,
            "clear_on_media_end": false
        }),
        scene_item_enabled: true,
        purpose: purpose.to_string(),
    }
}

fn browser_source(
    role: SceneRole,
    scene_name: &str,
    input_name: impl Into<String>,
    url: String,
    purpose: impl Into<String>,
) -> ObsSourceTemplate {
    ObsSourceTemplate {
        scene_role: role,
        scene_name: scene_name.to_string(),
        input_name: input_name.into(),
        source_kind: ObsSourceKind::BrowserSource,
        input_kind: "browser_source".to_string(),
        input_settings: json!({
            "url": url,
            "width": 1920,
            "height": 1080,
            "fps": 30,
            "shutdown": false,
            "restart_when_active": true
        }),
        scene_item_enabled: true,
        purpose: purpose.into(),
    }
}

fn srt_listener_url(host: &str, port: u16, latency_ms: u32) -> String {
    let latency_us = u64::from(latency_ms).saturating_mul(1_000);
    format!("srt://{host}:{port}?mode=listener&latency={latency_us}")
}

fn asset_file_uri(root_dir: &str, relative_path: &str) -> String {
    let root = root_dir.trim().replace('\\', "/");
    let relative = relative_path.trim().replace('\\', "/");
    format!("file:///$OPENIRL_PROJECT_ROOT/{root}/{relative}")
}

fn role_asset_file(role: SceneRole) -> &'static str {
    match role {
        SceneRole::Live => "live-sample.svg",
        SceneRole::LowSignal => "low-signal.svg",
        SceneRole::Brb => "brb.svg",
        SceneRole::BackupFeed => "backup-feed.svg",
        SceneRole::Privacy => "privacy.svg",
        SceneRole::StartingSoon => "starting-soon.svg",
        SceneRole::Ending => "ending.svg",
    }
}

fn svg_asset(
    role: Option<SceneRole>,
    relative_path: &str,
    title: &str,
    subtitle: &str,
    purpose: &str,
) -> FallbackAsset {
    FallbackAsset {
        role,
        kind: FallbackAssetKind::Svg,
        relative_path: relative_path.to_string(),
        content_type: "image/svg+xml".to_string(),
        purpose: purpose.to_string(),
        contents: fallback_svg(title, subtitle),
    }
}

fn html_asset(relative_path: &str) -> FallbackAsset {
    FallbackAsset {
        role: None,
        kind: FallbackAssetKind::Html,
        relative_path: relative_path.to_string(),
        content_type: "text/html; charset=utf-8".to_string(),
        purpose: "Local browser-source status sample.".to_string(),
        contents: r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width,initial-scale=1">
  <title>OpenIRL Fallback Overlay</title>
  <style>
    html,body{margin:0;width:100%;height:100%;background:transparent;font-family:Inter,Segoe UI,Arial,sans-serif;color:white;overflow:hidden}
    .pill{position:absolute;left:32px;bottom:32px;padding:14px 18px;border-radius:999px;background:rgba(0,0,0,.58);border:1px solid rgba(255,255,255,.2);letter-spacing:.08em;text-transform:uppercase;font-weight:700}
  </style>
</head>
<body>
  <div class="pill">OpenIRL local fallback ready</div>
</body>
</html>
"#.to_string(),
    }
}

fn markdown_asset(relative_path: &str) -> FallbackAsset {
    FallbackAsset {
        role: None,
        kind: FallbackAssetKind::Markdown,
        relative_path: relative_path.to_string(),
        content_type: "text/markdown; charset=utf-8".to_string(),
        purpose: "Operator notes for generated fallback assets.".to_string(),
        contents: "# OpenIRL Generated Fallback Assets\n\nThese files are generated locally for OBS browser/image sources. They contain no stream keys and require no hosted asset service. Replace them with branded assets as needed, then set `assets.overwrite_existing=false` to preserve manual edits.\n".to_string(),
    }
}

fn fallback_svg(title: &str, subtitle: &str) -> String {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="1920" height="1080" viewBox="0 0 1920 1080" role="img" aria-label="{title}">
  <defs>
    <linearGradient id="bg" x1="0" x2="1" y1="0" y2="1">
      <stop offset="0%" stop-color="#111827"/>
      <stop offset="100%" stop-color="#020617"/>
    </linearGradient>
  </defs>
  <rect width="1920" height="1080" fill="url(#bg)"/>
  <rect x="112" y="118" width="1696" height="844" rx="42" fill="rgba(255,255,255,0.035)" stroke="rgba(255,255,255,0.18)" stroke-width="3"/>
  <text x="960" y="492" text-anchor="middle" font-family="Inter, Segoe UI, Arial, sans-serif" font-size="116" font-weight="800" fill="#f8fafc" letter-spacing="8">{title}</text>
  <text x="960" y="604" text-anchor="middle" font-family="Inter, Segoe UI, Arial, sans-serif" font-size="40" font-weight="500" fill="#cbd5e1">{subtitle}</text>
  <text x="960" y="884" text-anchor="middle" font-family="Inter, Segoe UI, Arial, sans-serif" font-size="28" font-weight="700" fill="#94a3b8" letter-spacing="4">OPENIRL LOCAL-FIRST IRL STACK</text>
</svg>
"##,
        title = escape_xml(title),
        subtitle = escape_xml(subtitle)
    )
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn asset_manifest(plan: &FallbackAssetPlan) -> Value {
    json!({
        "root_dir": plan.root_dir.clone(),
        "overwrite_existing": plan.overwrite_existing,
        "assets": plan.assets.iter().map(|asset| json!({
            "role": asset.role,
            "kind": asset.kind,
            "relative_path": asset.relative_path.clone(),
            "content_type": asset.content_type.clone(),
            "purpose": asset.purpose.clone(),
            "bytes": asset.contents.len()
        })).collect::<Vec<_>>()
    })
}

fn safe_join(root: &Path, relative_path: &str) -> Result<PathBuf, SceneTemplateError> {
    let relative = Path::new(relative_path);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(SceneTemplateError::Invalid(format!(
            "asset relative path must stay inside asset root: {relative_path}"
        )));
    }
    Ok(root.join(relative))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_contains_live_and_brb_sources() {
        let scenes = SceneBundle::default_irl();
        let plan = build_scene_materialization_plan(&scenes, &SceneTemplateRequest::default());
        assert!(
            plan.sources
                .iter()
                .any(|source| source.scene_role == SceneRole::Live)
        );
        assert!(
            plan.sources
                .iter()
                .any(|source| source.scene_role == SceneRole::Brb)
        );
        assert!(
            plan.assets
                .assets
                .iter()
                .any(|asset| asset.relative_path == "brb.svg")
        );
    }

    #[test]
    fn safe_join_rejects_parent_paths() {
        let result = safe_join(Path::new("assets"), "../secret.txt");
        assert!(result.is_err());
    }
}
