//! Shared domain model.

use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use uuid::Uuid;

/// Media/control protocols OpenIRL tracks or configures.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Protocol {
    /// Legacy RTMP ingest/output.
    Rtmp,
    /// TLS RTMP ingest/output.
    Rtmps,
    /// Secure Reliable Transport.
    Srt,
    /// SRT link aggregation.
    Srtla,
    /// Next-generation SRTLA variant.
    Srtla2,
    /// Reliable Internet Stream Transport.
    Rist,
    /// WebRTC ingest.
    Whip,
    /// WebRTC egress/preview.
    Whep,
    /// Enhanced RTMP profile.
    EnhancedRtmp,
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Rtmp => "rtmp",
            Self::Rtmps => "rtmps",
            Self::Srt => "srt",
            Self::Srtla => "srtla",
            Self::Srtla2 => "srtla2",
            Self::Rist => "rist",
            Self::Whip => "whip",
            Self::Whep => "whep",
            Self::EnhancedRtmp => "enhanced-rtmp",
        };
        f.write_str(text)
    }
}

/// Encoder/app/hardware family.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum EncoderKind {
    /// Moblin iOS encoder.
    Moblin,
    /// IRL Pro Android encoder.
    IrlPro,
    /// Larix Broadcaster.
    Larix,
    /// BELABOX hardware/software encoder.
    Belabox,
    /// OBS as contribution source.
    Obs,
    /// LiveU-style contribution workflow.
    LiveuLike,
    /// Custom encoder.
    Custom,
}

impl fmt::Display for EncoderKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Moblin => "moblin",
            Self::IrlPro => "irl-pro",
            Self::Larix => "larix",
            Self::Belabox => "belabox",
            Self::Obs => "obs",
            Self::LiveuLike => "liveu-like",
            Self::Custom => "custom",
        };
        f.write_str(text)
    }
}

/// Deployment mode.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeploymentMode {
    /// Phone/backpack connects straight to local OBS machine.
    LocalDirect,
    /// Friend/moderator hosts a relay.
    FriendRelay,
    /// Cheap VPS relay bridges public ingest to local OBS.
    VpsRelay,
    /// Backpack encoder focused workflow.
    BackpackEncoder,
}

impl fmt::Display for DeploymentMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::LocalDirect => "local-direct",
            Self::FriendRelay => "friend-relay",
            Self::VpsRelay => "vps-relay",
            Self::BackpackEncoder => "backpack-encoder",
        };
        f.write_str(text)
    }
}

/// Health state that drives scene automation.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HealthState {
    /// Stream is within expected operating envelope.
    Healthy,
    /// Early warning; do not switch to BRB yet.
    Degraded,
    /// Viewers may see broken video/audio while connection is technically alive.
    Brownout,
    /// BRB/fallback should be visible.
    Brb,
    /// Switch to backup source when configured.
    BackupIngest,
    /// Contribution source is unavailable.
    Offline,
    /// Feed is improving but must remain stable before returning to live.
    RecoveryPending,
}

impl fmt::Display for HealthState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Brownout => "brownout",
            Self::Brb => "brb",
            Self::BackupIngest => "backup-ingest",
            Self::Offline => "offline",
            Self::RecoveryPending => "recovery-pending",
        };
        f.write_str(text)
    }
}

/// OBS scene role used by automation.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SceneRole {
    /// Main live scene.
    Live,
    /// Low-motion/low-bitrate-safe scene.
    LowSignal,
    /// BRB scene.
    Brb,
    /// Backup feed scene.
    BackupFeed,
    /// Privacy/panic scene.
    Privacy,
    /// Starting scene.
    StartingSoon,
    /// Ending scene.
    Ending,
}

impl fmt::Display for SceneRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Live => "live",
            Self::LowSignal => "low-signal",
            Self::Brb => "brb",
            Self::BackupFeed => "backup-feed",
            Self::Privacy => "privacy",
            Self::StartingSoon => "starting-soon",
            Self::Ending => "ending",
        };
        f.write_str(text)
    }
}

impl FromStr for SceneRole {
    type Err = SceneRoleParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "live" => Ok(Self::Live),
            "low-signal" | "low_signal" | "lowsignal" => Ok(Self::LowSignal),
            "brb" => Ok(Self::Brb),
            "backup-feed" | "backup" | "backup_feed" => Ok(Self::BackupFeed),
            "privacy" | "panic" => Ok(Self::Privacy),
            "starting-soon" | "starting" | "starting_soon" => Ok(Self::StartingSoon),
            "ending" | "end" => Ok(Self::Ending),
            _ => Err(SceneRoleParseError {
                value: value.to_string(),
            }),
        }
    }
}

/// Scene role parse error.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct SceneRoleParseError {
    /// Rejected value.
    pub value: String,
}

impl fmt::Display for SceneRoleParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown scene role: {}", self.value)
    }
}

impl std::error::Error for SceneRoleParseError {}

/// Named OBS scene mapped to a semantic role.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct SceneDefinition {
    /// Stable ID for scene definitions.
    pub id: Uuid,
    /// Semantic scene role.
    pub role: SceneRole,
    /// OBS scene name.
    pub name: String,
}

impl SceneDefinition {
    /// Creates a scene definition with a generated ID.
    #[must_use]
    pub fn new(role: SceneRole, name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            role,
            name: name.into(),
        }
    }
}

/// A full default IRL scene bundle.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct SceneBundle {
    /// Bundle ID.
    pub id: Uuid,
    /// Human-readable bundle name.
    pub name: String,
    /// Scenes in the bundle.
    pub scenes: Vec<SceneDefinition>,
}

impl SceneBundle {
    /// Standard OpenIRL scene set.
    #[must_use]
    pub fn default_irl() -> Self {
        Self::from_names(
            "OpenIRL Default IRL Bundle",
            SceneNames {
                live: "OpenIRL Live".to_string(),
                low_signal: "OpenIRL Low Signal".to_string(),
                brb: "OpenIRL BRB".to_string(),
                backup_feed: "OpenIRL Backup Feed".to_string(),
                privacy: "OpenIRL Privacy".to_string(),
                starting_soon: "OpenIRL Starting Soon".to_string(),
                ending: "OpenIRL Ending".to_string(),
            },
        )
    }

    /// Creates an IRL scene bundle from explicit scene names.
    #[must_use]
    pub fn from_names(name: impl Into<String>, names: SceneNames) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            scenes: vec![
                SceneDefinition::new(SceneRole::Live, names.live),
                SceneDefinition::new(SceneRole::LowSignal, names.low_signal),
                SceneDefinition::new(SceneRole::Brb, names.brb),
                SceneDefinition::new(SceneRole::BackupFeed, names.backup_feed),
                SceneDefinition::new(SceneRole::Privacy, names.privacy),
                SceneDefinition::new(SceneRole::StartingSoon, names.starting_soon),
                SceneDefinition::new(SceneRole::Ending, names.ending),
            ],
        }
    }

    /// Finds the scene name for a role.
    #[must_use]
    pub fn scene_name(&self, role: SceneRole) -> Option<&str> {
        self.scenes
            .iter()
            .find(|scene| scene.role == role)
            .map(|scene| scene.name.as_str())
    }
}

/// Portable scene-name input used by config and APIs.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct SceneNames {
    /// Live scene.
    pub live: String,
    /// Low-signal scene.
    pub low_signal: String,
    /// BRB scene.
    pub brb: String,
    /// Backup-feed scene.
    pub backup_feed: String,
    /// Privacy scene.
    pub privacy: String,
    /// Starting-soon scene.
    pub starting_soon: String,
    /// Ending scene.
    pub ending: String,
}

/// Raw stream health metrics sampled by adapters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamMetrics {
    /// Input contribution bitrate in Kbps.
    pub input_bitrate_kbps: u32,
    /// Destination output bitrate in Kbps.
    pub output_bitrate_kbps: u32,
    /// Packet loss percentage from contribution/relay layer.
    pub packet_loss_percent: f32,
    /// Retransmission pressure per second.
    pub retransmits_per_sec: u32,
    /// Round-trip time in milliseconds.
    pub rtt_ms: u32,
    /// Jitter in milliseconds.
    pub jitter_ms: u32,
    /// Connected SRTLA links/modems.
    pub connected_links: u8,
    /// OBS dropped frames per minute.
    pub obs_dropped_frames_per_min: u32,
    /// Encoder FPS estimate.
    pub encoder_fps: f32,
    /// Audio silence window in milliseconds.
    pub audio_silence_ms: u32,
    /// Frozen-frame window in milliseconds.
    pub frozen_frame_ms: u32,
    /// Reconnect count in the current session.
    pub reconnect_count: u32,
    /// Time since last clean video frame in milliseconds.
    pub clean_frame_age_ms: u32,
    /// Monotonic sample timestamp in milliseconds.
    pub timestamp_ms: u64,
}

impl Default for StreamMetrics {
    fn default() -> Self {
        Self {
            input_bitrate_kbps: 5_500,
            output_bitrate_kbps: 6_000,
            packet_loss_percent: 0.0,
            retransmits_per_sec: 0,
            rtt_ms: 80,
            jitter_ms: 10,
            connected_links: 1,
            obs_dropped_frames_per_min: 0,
            encoder_fps: 30.0,
            audio_silence_ms: 0,
            frozen_frame_ms: 0,
            reconnect_count: 0,
            clean_frame_age_ms: 0,
            timestamp_ms: 0,
        }
    }
}

/// Tunable thresholds for health scoring.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HealthThresholds {
    /// Minimum healthy input bitrate.
    pub min_healthy_bitrate_kbps: u32,
    /// Brownout threshold for input bitrate.
    pub min_brownout_bitrate_kbps: u32,
    /// Packet loss where stream becomes degraded.
    pub degraded_packet_loss_percent: f32,
    /// Packet loss where stream becomes brownout.
    pub brownout_packet_loss_percent: f32,
    /// RTT where stream becomes degraded.
    pub degraded_rtt_ms: u32,
    /// RTT where stream becomes brownout.
    pub brownout_rtt_ms: u32,
    /// Clean frame age where feed is treated as frozen/broken.
    pub brb_clean_frame_age_ms: u32,
    /// Audio silence warning window.
    pub audio_silence_warn_ms: u32,
    /// Frozen-frame BRB window.
    pub frozen_frame_brb_ms: u32,
    /// Required stable time before returning to live.
    pub recovery_hold_ms: u32,
}

impl Default for HealthThresholds {
    fn default() -> Self {
        Self {
            min_healthy_bitrate_kbps: 3_500,
            min_brownout_bitrate_kbps: 1_200,
            degraded_packet_loss_percent: 3.0,
            brownout_packet_loss_percent: 8.0,
            degraded_rtt_ms: 350,
            brownout_rtt_ms: 700,
            brb_clean_frame_age_ms: 12_000,
            audio_silence_warn_ms: 8_000,
            frozen_frame_brb_ms: 8_000,
            recovery_hold_ms: 5_000,
        }
    }
}

/// Health decision returned by the state engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HealthDecision {
    /// Selected health state.
    pub state: HealthState,
    /// 0-100 health score.
    pub score: u8,
    /// Explanatory reason strings.
    pub reasons: Vec<String>,
    /// Suggested OBS scene role.
    pub recommended_scene: SceneRole,
}

impl HealthDecision {
    /// Creates a healthy decision.
    #[must_use]
    pub fn healthy() -> Self {
        Self {
            state: HealthState::Healthy,
            score: 100,
            reasons: vec!["all metrics are within thresholds".to_string()],
            recommended_scene: SceneRole::Live,
        }
    }
}
