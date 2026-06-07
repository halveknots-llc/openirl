//! Production workflow models: outputs, vertical scenes, replay, clips, and mod commands.

use openirl_core::{HealthState, SceneBundle, SceneRole};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Output platform.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OutputPlatform {
    /// Twitch RTMP/Enhanced Broadcasting path.
    Twitch,
    /// Kick RTMP path.
    Kick,
    /// YouTube RTMP path.
    YouTube,
    /// Custom RTMP endpoint.
    CustomRtmp,
    /// Custom SRT endpoint.
    CustomSrt,
}

/// Layout intent for a stream output.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VideoLayout {
    /// 16:9 landscape output.
    Horizontal,
    /// 9:16 vertical output.
    Vertical,
    /// Both horizontal and vertical are prepared.
    Dual,
}

/// One output profile plan.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct OutputProfile {
    /// Stable label.
    pub name: String,
    /// Platform.
    pub platform: OutputPlatform,
    /// Layout.
    pub layout: VideoLayout,
    /// Server URL with no stream key.
    pub server_url: String,
    /// Environment variable containing stream key or token.
    pub stream_key_env: String,
    /// Maximum video bitrate.
    pub max_bitrate_kbps: u32,
    /// Whether enabled by default.
    pub enabled: bool,
}

/// Vertical scene plan.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct VerticalScenePlan {
    /// Canvas width.
    pub canvas_width: u32,
    /// Canvas height.
    pub canvas_height: u32,
    /// Crop source guidance.
    pub crop_source: String,
    /// Safe-area notes.
    pub safe_area: Vec<String>,
    /// Scene roles expected to have vertical variants.
    pub scene_roles: Vec<SceneRole>,
}

/// Replay buffer plan.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReplayBufferPlan {
    /// Whether replay buffer should be enabled.
    pub enabled: bool,
    /// Replay duration.
    pub duration_seconds: u32,
    /// Clips folder path.
    pub clips_dir: String,
    /// OBS command exposed by the agent.
    pub save_endpoint: String,
}

/// End-to-end production plan.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProductionPlan {
    /// Output profiles.
    pub outputs: Vec<OutputProfile>,
    /// Vertical output plan.
    pub vertical: VerticalScenePlan,
    /// Replay buffer plan.
    pub replay: ReplayBufferPlan,
    /// Scene bundle used by OBS.
    pub scenes: SceneBundle,
    /// Non-blocking warnings.
    pub warnings: Vec<String>,
}

/// Clip marker request.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ClipMarkerRequest {
    /// Clip title.
    pub title: String,
    /// Optional note.
    pub note: Option<String>,
    /// Tags.
    pub tags: Vec<String>,
}

/// Clip marker.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ClipMarker {
    /// Marker ID.
    pub id: Uuid,
    /// Creation time.
    pub created_at: OffsetDateTime,
    /// Clip title.
    pub title: String,
    /// Optional note.
    pub note: Option<String>,
    /// Tags.
    pub tags: Vec<String>,
    /// Health state when marker was created.
    pub health_state: HealthState,
}

/// Bounded clip-marker store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipMarkerStore {
    markers: Vec<ClipMarker>,
    limit: usize,
}

impl ClipMarkerStore {
    /// Creates a marker store.
    #[must_use]
    pub fn new(limit: usize) -> Self {
        Self {
            markers: Vec::new(),
            limit: limit.max(1),
        }
    }

    /// Adds a marker.
    pub fn add_marker(
        &mut self,
        request: ClipMarkerRequest,
        health_state: HealthState,
    ) -> ClipMarker {
        let marker = ClipMarker {
            id: Uuid::new_v4(),
            created_at: OffsetDateTime::now_utc(),
            title: if request.title.trim().is_empty() {
                "Untitled marker".to_string()
            } else {
                request.title
            },
            note: request.note,
            tags: request.tags,
            health_state,
        };
        self.markers.push(marker.clone());
        if self.markers.len() > self.limit {
            let excess = self.markers.len() - self.limit;
            self.markers.drain(0..excess);
        }
        marker
    }

    /// Lists markers.
    #[must_use]
    pub fn markers(&self) -> Vec<ClipMarker> {
        self.markers.clone()
    }

    /// Clears markers.
    pub fn clear(&mut self) {
        self.markers.clear();
    }
}

impl Default for ClipMarkerStore {
    fn default() -> Self {
        Self::new(256)
    }
}

/// Moderator role.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ModeratorRole {
    /// Owner/operator.
    Owner,
    /// Producer.
    Producer,
    /// Moderator.
    Moderator,
    /// Viewer/read-only.
    Viewer,
}

/// Moderator command request.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ModeratorCommandRequest {
    /// Role claimed by the authenticated operator session.
    pub role: ModeratorRole,
    /// Action label.
    pub action: String,
    /// Optional argument, such as a scene role or marker title.
    pub argument: Option<String>,
}

/// Command authorization decision.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct CommandDecision {
    /// Whether the command may execute.
    pub allowed: bool,
    /// Normalized action.
    pub action: String,
    /// Reason.
    pub reason: String,
    /// Permission name.
    pub permission: String,
}

/// Builds a production plan from the active scene bundle.
#[must_use]
pub fn default_production_plan(scenes: &SceneBundle) -> ProductionPlan {
    ProductionPlan {
        outputs: vec![
            output("twitch-horizontal", OutputPlatform::Twitch, VideoLayout::Horizontal, "rtmp://live.twitch.tv/app", "OPENIRL_TWITCH_STREAM_KEY", 6_000, true),
            output("youtube-horizontal", OutputPlatform::YouTube, VideoLayout::Horizontal, "rtmp://a.rtmp.youtube.com/live2", "OPENIRL_YOUTUBE_STREAM_KEY", 9_000, false),
            output("kick-horizontal", OutputPlatform::Kick, VideoLayout::Horizontal, "rtmps://fa-gatekeeper.kick.com/app", "OPENIRL_KICK_STREAM_KEY", 8_000, false),
            output("vertical-custom", OutputPlatform::CustomRtmp, VideoLayout::Vertical, "rtmp://127.0.0.1/live", "OPENIRL_VERTICAL_STREAM_KEY", 4_500, false),
        ],
        vertical: VerticalScenePlan {
            canvas_width: 1080,
            canvas_height: 1920,
            crop_source: "Center-crop the Live source with face-aware manual override in OBS.".to_string(),
            safe_area: vec![
                "Keep captions and chat overlays away from top/bottom platform UI zones.".to_string(),
                "Use Low Signal and BRB variants that remain legible on phones.".to_string(),
            ],
            scene_roles: vec![SceneRole::Live, SceneRole::LowSignal, SceneRole::Brb, SceneRole::Privacy],
        },
        replay: ReplayBufferPlan {
            enabled: true,
            duration_seconds: 90,
            clips_dir: "clips/openirl".to_string(),
            save_endpoint: "/api/production/replay/save".to_string(),
        },
        scenes: scenes.clone(),
        warnings: vec![
            "OBS output settings remain local to OBS until SetStreamServiceSettings smoke tests are completed.".to_string(),
            "Do not store stream keys in config; use environment variables.".to_string(),
        ],
    }
}

/// Evaluates whether a moderator command is allowed.
#[must_use]
pub fn evaluate_moderator_command(request: &ModeratorCommandRequest) -> CommandDecision {
    let action = request.action.trim().to_ascii_lowercase();
    let permission = permission_for_action(&action).to_string();
    let allowed = match request.role {
        ModeratorRole::Owner => true,
        ModeratorRole::Producer => matches!(
            action.as_str(),
            "switch-scene"
                | "save-replay"
                | "add-marker"
                | "privacy"
                | "status"
                | "start-recording"
                | "stop-recording"
        ),
        ModeratorRole::Moderator => matches!(
            action.as_str(),
            "switch-scene" | "save-replay" | "add-marker" | "privacy" | "status"
        ),
        ModeratorRole::Viewer => matches!(action.as_str(), "status"),
    };
    CommandDecision {
        allowed,
        action,
        reason: if allowed {
            "role permits action".to_string()
        } else {
            "role does not permit action".to_string()
        },
        permission,
    }
}

fn output(
    name: &str,
    platform: OutputPlatform,
    layout: VideoLayout,
    server_url: &str,
    stream_key_env: &str,
    max_bitrate_kbps: u32,
    enabled: bool,
) -> OutputProfile {
    OutputProfile {
        name: name.to_string(),
        platform,
        layout,
        server_url: server_url.to_string(),
        stream_key_env: stream_key_env.to_string(),
        max_bitrate_kbps,
        enabled,
    }
}

fn permission_for_action(action: &str) -> &'static str {
    match action {
        "start-stream" | "stop-stream" => "stream-control",
        "start-recording" | "stop-recording" | "save-replay" => "recording-control",
        "switch-scene" | "privacy" => "scene-control",
        "add-marker" => "marker-write",
        "status" => "status-read",
        _ => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moderator_cannot_stop_stream() {
        let request = ModeratorCommandRequest {
            role: ModeratorRole::Moderator,
            action: "stop-stream".to_string(),
            argument: None,
        };
        let decision = evaluate_moderator_command(&request);
        assert!(!decision.allowed);
    }

    #[test]
    fn marker_store_is_bounded() {
        let mut store = ClipMarkerStore::new(1);
        store.add_marker(
            ClipMarkerRequest {
                title: "a".to_string(),
                note: None,
                tags: Vec::new(),
            },
            HealthState::Healthy,
        );
        store.add_marker(
            ClipMarkerRequest {
                title: "b".to_string(),
                note: None,
                tags: Vec::new(),
            },
            HealthState::Brownout,
        );
        assert_eq!(store.markers().len(), 1);
    }
}
