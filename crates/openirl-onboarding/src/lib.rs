//! First-run setup wizard planning.

use openirl_config::{AppConfig, MetricsSourceKind, ObsAdapterKind};
use openirl_core::{DeploymentMode, EncoderKind, Protocol};
use openirl_profiles::{
    GeneratedProfile, ProfileError, ProfileRequest, generate_profile, support_matrix,
};
use openirl_qr::{QrRender, QrRenderError, QrRenderRequest, render_qr_svg};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Onboarding error.
#[derive(Debug, Error)]
pub enum OnboardingError {
    /// Profile generation failed.
    #[error(transparent)]
    Profile(#[from] ProfileError),
    /// QR render failed.
    #[error(transparent)]
    Qr(#[from] QrRenderError),
    /// Encoder unsupported.
    #[error("encoder is not in the built-in support matrix: {0:?}")]
    UnsupportedEncoder(EncoderKind),
}

/// Wizard request from the dashboard or CLI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct OnboardingRequest {
    /// Encoder app/hardware.
    pub encoder: EncoderKind,
    /// Protocol override. Preferred protocol is used when omitted.
    pub protocol: Option<Protocol>,
    /// Deployment mode.
    pub deployment_mode: DeploymentMode,
    /// Public host override for generated profiles.
    pub public_host: Option<String>,
    /// Contribution bitrate recommendation.
    pub bitrate_kbps: u32,
    /// Render QR SVG for profile payload.
    pub include_qr_svg: bool,
}

impl Default for OnboardingRequest {
    fn default() -> Self {
        Self {
            encoder: EncoderKind::Moblin,
            protocol: None,
            deployment_mode: DeploymentMode::LocalDirect,
            public_host: None,
            bitrate_kbps: 4_500,
            include_qr_svg: true,
        }
    }
}

/// One wizard step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnboardingStep {
    /// 1-based step number.
    pub number: u8,
    /// Step title.
    pub title: String,
    /// Operator action.
    pub action: String,
    /// Success check.
    pub validation: String,
}

/// Config patch preview for the requested mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnboardingConfigPatch {
    /// Public host to save under `[ingest]`.
    pub ingest_public_host: String,
    /// Relay enabled recommendation.
    pub relay_enabled: bool,
    /// Deployment mode to save under `[relay]`.
    pub relay_mode: DeploymentMode,
    /// OBS adapter hint.
    pub obs_adapter_hint: ObsAdapterKind,
    /// Metrics source hint.
    pub metrics_source_hint: MetricsSourceKind,
}

/// Full wizard output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OnboardingPlan {
    /// Request echoed back.
    pub request: OnboardingRequest,
    /// Generated encoder profile.
    pub profile: GeneratedProfile,
    /// Optional QR SVG for the profile URL.
    pub qr: Option<QrRender>,
    /// Ordered setup steps.
    pub steps: Vec<OnboardingStep>,
    /// Config patch preview.
    pub config_patch: OnboardingConfigPatch,
    /// Non-blocking warnings.
    pub warnings: Vec<String>,
}

/// Builds an onboarding plan from config and request.
///
/// # Errors
///
/// Returns an error when the requested encoder/protocol profile cannot be generated.
pub fn plan_onboarding(
    config: &AppConfig,
    request: &OnboardingRequest,
) -> Result<OnboardingPlan, OnboardingError> {
    let support = support_matrix()
        .into_iter()
        .find(|entry| entry.encoder == request.encoder)
        .ok_or(OnboardingError::UnsupportedEncoder(request.encoder))?;
    let protocol = request.protocol.unwrap_or(support.preferred_protocol);
    let host = request
        .public_host
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| config.ingest.public_host.clone());
    let profile_request = ProfileRequest {
        encoder: request.encoder,
        protocol,
        host: host.clone(),
        port: port_for_protocol(config, protocol),
        stream_id: default_stream_id(request.encoder),
        passphrase: Some("replace-me".to_string()),
        latency_ms: config.ingest.default_latency_ms,
        bitrate_kbps: request.bitrate_kbps,
    };
    let profile = generate_profile(&profile_request)?;
    let qr = if request.include_qr_svg {
        Some(render_qr_svg(&QrRenderRequest::new(
            profile.contribution_url.clone(),
            format!("{} {} profile", request.encoder, protocol),
        ))?)
    } else {
        None
    };
    let relay_enabled = !matches!(request.deployment_mode, DeploymentMode::LocalDirect);
    let mut warnings = Vec::new();
    if matches!(protocol, Protocol::Rtmp | Protocol::Rtmps) {
        warnings.push(
            "RTMP is compatibility mode; prefer SRT or SRTLA for mobile IRL contribution."
                .to_string(),
        );
    }
    if relay_enabled && is_loopback_host(&host) {
        warnings.push("relay modes need a reachable public/VPN host, not 127.0.0.1.".to_string());
    }
    Ok(OnboardingPlan {
        request: request.clone(),
        profile,
        qr,
        steps: setup_steps(request, protocol, relay_enabled),
        config_patch: OnboardingConfigPatch {
            ingest_public_host: host,
            relay_enabled,
            relay_mode: request.deployment_mode,
            obs_adapter_hint: ObsAdapterKind::WebSocket,
            metrics_source_hint: if relay_enabled {
                MetricsSourceKind::MediaMtxPrometheus
            } else {
                MetricsSourceKind::Demo
            },
        },
        warnings,
    })
}

/// Returns wizard option labels for dashboard display.
#[must_use]
pub fn onboarding_options() -> Vec<String> {
    vec![
        "local-direct".to_string(),
        "friend-relay".to_string(),
        "vps-relay".to_string(),
        "backpack-encoder".to_string(),
    ]
}

fn setup_steps(
    request: &OnboardingRequest,
    protocol: Protocol,
    relay_enabled: bool,
) -> Vec<OnboardingStep> {
    let mut steps = vec![
        step(
            1,
            "Start OpenIRL Agent",
            "Run openirl-agent serve and open the dashboard.",
            "GET /health returns ok.",
        ),
        step(
            2,
            "Connect OBS",
            "Enable OBS WebSocket and set OPENIRL_OBS_PASSWORD when using real OBS.",
            "GET /api/obs/status returns connected.",
        ),
    ];
    if relay_enabled {
        steps.push(step(
            3,
            "Start Relay",
            "Enable the MediaMTX/SRTLA process plan for the selected deployment mode.",
            "GET /api/relay/readiness has no blockers.",
        ));
    }
    let profile_step_number = if relay_enabled { 4 } else { 3 };
    steps.push(step(
        profile_step_number,
        "Configure Encoder",
        format!(
            "Import or scan the generated {} profile using {}.",
            protocol, request.encoder
        ),
        "Encoder connects and metrics begin updating.",
    ));
    steps.push(step(
        profile_step_number + 1,
        "Test Brownout",
        "Run a demo brownout sample or briefly reduce encoder bitrate.",
        "Dashboard switches to Low Signal/BRB as configured.",
    ));
    steps.push(step(
        profile_step_number + 2,
        "Go Live",
        "Start stream from OBS after the live source is stable.",
        "OBS stream status is active and current scene is Live.",
    ));
    steps
}

fn step(
    number: u8,
    title: impl Into<String>,
    action: impl Into<String>,
    validation: impl Into<String>,
) -> OnboardingStep {
    OnboardingStep {
        number,
        title: title.into(),
        action: action.into(),
        validation: validation.into(),
    }
}

fn default_stream_id(encoder: EncoderKind) -> String {
    match encoder {
        EncoderKind::Moblin => "moblin-main".to_string(),
        EncoderKind::IrlPro => "irlpro-main".to_string(),
        EncoderKind::Larix => "larix-main".to_string(),
        EncoderKind::Belabox => "belabox-main".to_string(),
        EncoderKind::Obs => "obs-main".to_string(),
        EncoderKind::LiveuLike => "liveu-main".to_string(),
        EncoderKind::Custom => "custom-main".to_string(),
    }
}

fn port_for_protocol(config: &AppConfig, protocol: Protocol) -> u16 {
    match protocol {
        Protocol::Srt => config.ingest.srt_port,
        Protocol::Srtla | Protocol::Srtla2 => config.ingest.srtla_port,
        Protocol::Rtmp | Protocol::Rtmps => config.ingest.rtmp_port,
        Protocol::Rist => config.ingest.srt_port,
        Protocol::Whip | Protocol::Whep | Protocol::EnhancedRtmp => 443,
    }
}

fn is_loopback_host(host: &str) -> bool {
    let normalized = host.trim().to_ascii_lowercase();
    normalized == "localhost" || normalized == "127.0.0.1" || normalized.starts_with("127.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_plan_generates_profile() -> Result<(), OnboardingError> {
        let plan = plan_onboarding(&AppConfig::default(), &OnboardingRequest::default())?;
        assert_eq!(plan.request.encoder, EncoderKind::Moblin);
        assert!(!plan.steps.is_empty());
        assert!(plan.qr.is_some());
        Ok(())
    }
}
