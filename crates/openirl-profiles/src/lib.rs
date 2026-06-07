//! Encoder profile generation.

use openirl_core::{EncoderKind, Protocol};
use openirl_vault::redact_stream_url;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Profile generation error.
#[derive(Debug, Error, Eq, PartialEq)]
pub enum ProfileError {
    /// Unsupported encoder/protocol combination.
    #[error("unsupported profile combination: {encoder:?} + {protocol:?}")]
    Unsupported {
        /// Encoder.
        encoder: EncoderKind,
        /// Protocol.
        protocol: Protocol,
    },
    /// Invalid request.
    #[error("invalid profile request: {0}")]
    Invalid(&'static str),
}

/// Public support matrix entry used by setup UIs.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProfileSupport {
    /// Encoder kind.
    pub encoder: EncoderKind,
    /// Supported protocols for this encoder.
    pub protocols: Vec<Protocol>,
    /// Preferred protocol for IRL contribution.
    pub preferred_protocol: Protocol,
}

/// Returns the built-in encoder support matrix.
#[must_use]
pub fn support_matrix() -> Vec<ProfileSupport> {
    vec![
        ProfileSupport {
            encoder: EncoderKind::Moblin,
            protocols: supported_protocols(EncoderKind::Moblin),
            preferred_protocol: Protocol::Srtla,
        },
        ProfileSupport {
            encoder: EncoderKind::IrlPro,
            protocols: supported_protocols(EncoderKind::IrlPro),
            preferred_protocol: Protocol::Srtla,
        },
        ProfileSupport {
            encoder: EncoderKind::Larix,
            protocols: supported_protocols(EncoderKind::Larix),
            preferred_protocol: Protocol::Srt,
        },
        ProfileSupport {
            encoder: EncoderKind::Belabox,
            protocols: supported_protocols(EncoderKind::Belabox),
            preferred_protocol: Protocol::Srtla,
        },
    ]
}

/// Returns supported protocols for an encoder.
#[must_use]
pub fn supported_protocols(encoder: EncoderKind) -> Vec<Protocol> {
    let candidates = [
        Protocol::Srt,
        Protocol::Srtla,
        Protocol::Srtla2,
        Protocol::Rtmp,
        Protocol::Rtmps,
        Protocol::Rist,
        Protocol::Whip,
    ];
    candidates
        .into_iter()
        .filter(|protocol| supports(encoder, *protocol))
        .collect()
}

/// Profile request.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProfileRequest {
    /// Encoder kind.
    pub encoder: EncoderKind,
    /// Contribution protocol.
    pub protocol: Protocol,
    /// Public host to connect to.
    pub host: String,
    /// Public port to connect to.
    pub port: u16,
    /// Stream ID or mount name.
    pub stream_id: String,
    /// Optional SRT passphrase.
    pub passphrase: Option<String>,
    /// SRT/RIST latency in milliseconds.
    pub latency_ms: u32,
    /// Suggested contribution bitrate.
    pub bitrate_kbps: u32,
}

/// Generated encoder profile.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct GeneratedProfile {
    /// Encoder kind.
    pub encoder: EncoderKind,
    /// Protocol.
    pub protocol: Protocol,
    /// URL with secret values included. Do not log this.
    pub contribution_url: String,
    /// Redacted display URL.
    pub display_url: String,
    /// Suggested OBS source name.
    pub obs_source_name: String,
    /// Human-readable notes.
    pub notes: Vec<String>,
}

/// Generates a profile.
///
/// # Errors
///
/// Returns an error for invalid fields or unsupported combinations.
pub fn generate_profile(request: &ProfileRequest) -> Result<GeneratedProfile, ProfileError> {
    validate(request)?;

    if !supports(request.encoder, request.protocol) {
        return Err(ProfileError::Unsupported {
            encoder: request.encoder,
            protocol: request.protocol,
        });
    }

    let contribution_url = match request.protocol {
        Protocol::Srt | Protocol::Srtla | Protocol::Srtla2 => srt_like_url(request),
        Protocol::Rtmp | Protocol::Rtmps => rtmp_like_url(request),
        Protocol::Rist => rist_like_url(request),
        Protocol::Whip => whip_like_url(request),
        other => {
            return Err(ProfileError::Unsupported {
                encoder: request.encoder,
                protocol: other,
            });
        }
    };

    let notes = notes_for(request);
    let display_url = redact_stream_url(&contribution_url);
    Ok(GeneratedProfile {
        encoder: request.encoder,
        protocol: request.protocol,
        contribution_url,
        display_url,
        obs_source_name: format!("OpenIRL {} {}", request.encoder, request.protocol),
        notes,
    })
}

fn validate(request: &ProfileRequest) -> Result<(), ProfileError> {
    if request.host.trim().is_empty() {
        return Err(ProfileError::Invalid("host is required"));
    }
    if request.stream_id.trim().is_empty() {
        return Err(ProfileError::Invalid("stream_id is required"));
    }
    if request.port == 0 {
        return Err(ProfileError::Invalid("port cannot be zero"));
    }
    Ok(())
}

fn supports(encoder: EncoderKind, protocol: Protocol) -> bool {
    match encoder {
        EncoderKind::Moblin => matches!(
            protocol,
            Protocol::Srt
                | Protocol::Srtla
                | Protocol::Srtla2
                | Protocol::Rtmp
                | Protocol::Rtmps
                | Protocol::Rist
                | Protocol::Whip
        ),
        EncoderKind::IrlPro => matches!(
            protocol,
            Protocol::Srt | Protocol::Srtla | Protocol::Rtmp | Protocol::Rtmps
        ),
        EncoderKind::Larix => matches!(
            protocol,
            Protocol::Srt | Protocol::Rtmp | Protocol::Rtmps | Protocol::Rist | Protocol::Whip
        ),
        EncoderKind::Belabox => {
            matches!(protocol, Protocol::Srt | Protocol::Srtla | Protocol::Srtla2)
        }
        EncoderKind::Obs | EncoderKind::LiveuLike | EncoderKind::Custom => true,
    }
}

fn srt_like_url(request: &ProfileRequest) -> String {
    let mut url = format!(
        "srt://{}:{}?mode=caller&streamid={}&latency={}",
        request.host,
        request.port,
        pct(&request.stream_id),
        request.latency_ms
    );
    if let Some(passphrase) = &request.passphrase {
        url.push_str("&passphrase=");
        url.push_str(&pct(passphrase));
        url.push_str("&pbkeylen=16");
    }
    url
}

fn rtmp_like_url(request: &ProfileRequest) -> String {
    format!(
        "{}://{}/live/{}",
        request.protocol,
        host_port(request),
        pct(&request.stream_id)
    )
}

fn rist_like_url(request: &ProfileRequest) -> String {
    format!(
        "rist://{}?stream-id={}&latency={}",
        host_port(request),
        pct(&request.stream_id),
        request.latency_ms
    )
}

fn whip_like_url(request: &ProfileRequest) -> String {
    format!(
        "https://{}/whip/{}",
        host_port(request),
        pct(&request.stream_id)
    )
}

fn host_port(request: &ProfileRequest) -> String {
    format!("{}:{}", request.host, request.port)
}

fn notes_for(request: &ProfileRequest) -> Vec<String> {
    let mut notes = vec![format!(
        "Suggested contribution bitrate: {} Kbps",
        request.bitrate_kbps
    )];
    if matches!(request.protocol, Protocol::Rtmp | Protocol::Rtmps) {
        notes.push(
            "RTMP is compatibility mode; prefer SRT/SRTLA for lossy IRL networks.".to_string(),
        );
    }
    if matches!(request.protocol, Protocol::Srtla | Protocol::Srtla2) {
        notes.push("Bonded mode: verify all cellular links before going live.".to_string());
    }
    notes
}

fn pct(input: &str) -> String {
    input
        .bytes()
        .flat_map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![char::from(b)]
            }
            _ => format!("%{b:02X}").chars().collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moblin_srt_profile_redacts_passphrase() -> Result<(), ProfileError> {
        let request = ProfileRequest {
            encoder: EncoderKind::Moblin,
            protocol: Protocol::Srt,
            host: "example.test".to_string(),
            port: 9000,
            stream_id: "main".to_string(),
            passphrase: Some("secret".to_string()),
            latency_ms: 1800,
            bitrate_kbps: 4500,
        };
        let profile = generate_profile(&request)?;
        assert!(profile.contribution_url.contains("secret"));
        assert!(!profile.display_url.contains("secret"));
        assert!(profile.display_url.contains("passphrase=<redacted>"));
        Ok(())
    }

    #[test]
    fn support_matrix_contains_belabox_srtla() {
        let matrix = support_matrix();
        let belabox = matrix
            .iter()
            .find(|entry| entry.encoder == EncoderKind::Belabox);
        assert!(belabox.is_some());
        if let Some(entry) = belabox {
            assert!(entry.protocols.contains(&Protocol::Srtla));
            assert_eq!(entry.preferred_protocol, Protocol::Srtla);
        }
    }

    #[test]
    fn irl_pro_rejects_whip() {
        let request = ProfileRequest {
            encoder: EncoderKind::IrlPro,
            protocol: Protocol::Whip,
            host: "example.test".to_string(),
            port: 443,
            stream_id: "main".to_string(),
            passphrase: None,
            latency_ms: 0,
            bitrate_kbps: 4500,
        };
        assert!(matches!(
            generate_profile(&request),
            Err(ProfileError::Unsupported { .. })
        ));
    }
}
