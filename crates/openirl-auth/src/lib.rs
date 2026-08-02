//! Dashboard authentication primitives for the local-first control plane.

use http::{Uri, uri::Authority};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use thiserror::Error;

/// Policy snapshot used by the agent and dashboard.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct AuthPolicy {
    /// Whether operator API token checks are enabled.
    pub enabled: bool,
    /// Environment variable containing the dashboard/operator token.
    pub token_env: String,
    /// Whether loopback browser use may proceed without a token.
    pub allow_loopback_without_token: bool,
    /// Whether non-loopback/LAN use requires a token.
    pub require_for_lan: bool,
}

impl AuthPolicy {
    /// Builds a conservative disabled-localhost policy.
    #[must_use]
    pub fn localhost_only(token_env: impl Into<String>) -> Self {
        Self {
            enabled: false,
            token_env: token_env.into(),
            allow_loopback_without_token: true,
            require_for_lan: true,
        }
    }
}

/// Dashboard-safe auth status.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct AuthStatus {
    /// Whether auth is enabled by policy.
    pub enabled: bool,
    /// Whether the token environment variable has a value.
    pub token_configured: bool,
    /// Token environment variable name.
    pub token_env: String,
    /// Required HTTP header format for API clients.
    pub required_header: String,
    /// Non-sensitive warnings.
    pub warnings: Vec<String>,
}

/// Token verification outcome.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct AuthDecision {
    /// True when the request should be allowed.
    pub allowed: bool,
    /// Short reason code.
    pub reason: String,
    /// Human-readable remediation for failed checks.
    pub remediation: Option<String>,
}

/// Auth errors.
#[derive(Debug, Error, Eq, PartialEq)]
pub enum AuthError {
    /// Header is malformed.
    #[error("malformed Authorization header")]
    MalformedHeader,
}

/// Builds a status snapshot without exposing the token.
#[must_use]
pub fn auth_status(policy: &AuthPolicy, token_value: Option<&str>) -> AuthStatus {
    let mut warnings = Vec::new();
    let token_configured = token_value
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false);
    if policy.enabled && !token_configured {
        warnings.push("auth is enabled but the token environment variable is empty".to_string());
    }
    if !policy.enabled && policy.require_for_lan {
        warnings.push(
            "auth is not enabled for localhost-only development; enable it before LAN exposure"
                .to_string(),
        );
    }
    AuthStatus {
        enabled: policy.enabled,
        token_configured,
        token_env: policy.token_env.clone(),
        required_header: "Authorization: Bearer <OPENIRL_DASHBOARD_TOKEN>".to_string(),
        warnings,
    }
}

/// Extracts a bearer token from an Authorization header.
pub fn bearer_token(header_value: &str) -> Result<&str, AuthError> {
    let trimmed = header_value.trim();
    let Some(token) = trimmed.strip_prefix("Bearer ") else {
        return Err(AuthError::MalformedHeader);
    };
    if token.trim().is_empty() {
        return Err(AuthError::MalformedHeader);
    }
    Ok(token.trim())
}

/// Verifies a supplied Authorization header against an optional token.
#[must_use]
pub fn verify_authorization_header(
    policy: &AuthPolicy,
    configured_token: Option<&str>,
    authorization_header: Option<&str>,
    is_loopback_request: bool,
) -> AuthDecision {
    if !policy.enabled && (is_loopback_request && policy.allow_loopback_without_token) {
        return allow("loopback-auth-not-required");
    }

    if !policy.enabled && !policy.require_for_lan {
        return allow("auth-disabled-by-policy");
    }

    let Some(configured_token) = configured_token.filter(|value| !value.trim().is_empty()) else {
        return deny(
            "token-not-configured",
            "Set the configured dashboard token environment variable before enabling LAN or authenticated dashboard use.",
        );
    };

    let Some(header) = authorization_header else {
        return deny(
            "missing-authorization",
            "Send Authorization: Bearer <token>.",
        );
    };

    match bearer_token(header) {
        Ok(token) if constant_time_eq(token.as_bytes(), configured_token.as_bytes()) => {
            allow("token-match")
        }
        Ok(_) => deny("token-mismatch", "Use the current dashboard token value."),
        Err(_) => deny(
            "malformed-authorization",
            "Use Authorization: Bearer <token>.",
        ),
    }
}

/// Returns whether a browser origin matches the request Host authority.
///
/// A missing origin is treated as a non-browser client. An explicit `null`
/// origin, malformed origin, or missing Host header is never considered same
/// origin.
#[must_use]
pub fn browser_origin_is_same_origin(origin: Option<&str>, host: Option<&str>) -> bool {
    let Some(origin) = origin.map(str::trim).filter(|value| !value.is_empty()) else {
        return true;
    };
    let Some(host) = host.map(str::trim).filter(|value| !value.is_empty()) else {
        return false;
    };
    if origin.eq_ignore_ascii_case("null") {
        return false;
    }
    let Ok(uri) = origin.parse::<Uri>() else {
        return false;
    };
    let Some(scheme) = uri.scheme_str() else {
        return false;
    };
    if !matches!(scheme, "http" | "https")
        || uri
            .path_and_query()
            .is_some_and(|value| value.as_str() != "/")
    {
        return false;
    }
    let Some(origin_authority) = uri.authority() else {
        return false;
    };
    let Ok(host_authority) = host.parse::<Authority>() else {
        return false;
    };
    authorities_match(origin_authority, &host_authority, scheme)
}

/// Returns whether a browser origin is same-origin or explicitly configured.
#[must_use]
pub fn browser_origin_is_allowed(
    origin: Option<&str>,
    host: Option<&str>,
    allowed_origins: &[String],
) -> bool {
    let Some(origin) = origin.map(str::trim).filter(|value| !value.is_empty()) else {
        return true;
    };
    if origin.eq_ignore_ascii_case("null") {
        return false;
    }
    if browser_origin_is_same_origin(Some(origin), host) {
        return true;
    }
    allowed_origins
        .iter()
        .any(|allowed| allowed.trim().eq_ignore_ascii_case(origin))
}

/// Returns whether a request authority is the configured direct loopback listener.
///
/// This is deliberately stricter than same-origin comparison. Browser-controlled
/// `Host` and `Origin` values may agree while naming an untrusted DNS host, so
/// tokenless access must also be anchored to the listener address.
#[must_use]
pub fn browser_request_is_trusted_local(
    origin: Option<&str>,
    host: Option<&str>,
    bind: SocketAddr,
) -> bool {
    if !request_host_is_trusted_local(host, bind) {
        return false;
    }
    let Some(origin) = origin.map(str::trim).filter(|value| !value.is_empty()) else {
        return true;
    };
    let Ok(uri) = origin.parse::<Uri>() else {
        return false;
    };
    uri.scheme_str() == Some("http") && browser_origin_is_same_origin(Some(origin), host)
}

/// Returns whether the Host authority names the configured loopback listener.
#[must_use]
pub fn request_host_is_trusted_local(host: Option<&str>, bind: SocketAddr) -> bool {
    if !bind.ip().is_loopback() {
        return false;
    }
    let Some(host) = host.map(str::trim).filter(|value| !value.is_empty()) else {
        return false;
    };
    let Ok(authority) = host.parse::<Authority>() else {
        return false;
    };
    if authority.as_str().contains('@') {
        return false;
    }
    let port_matches = authority
        .port_u16()
        .map_or(bind.port() == 80, |port| port == bind.port());
    port_matches && local_hostname(authority.host())
}

fn authorities_match(origin: &Authority, host: &Authority, scheme: &str) -> bool {
    if origin.as_str().contains('@') || host.as_str().contains('@') {
        return false;
    }
    origin.host().eq_ignore_ascii_case(host.host())
        && effective_port(origin, scheme) == effective_port(host, scheme)
}

fn effective_port(authority: &Authority, scheme: &str) -> Option<u16> {
    authority.port_u16().or(match scheme {
        "http" => Some(80),
        "https" => Some(443),
        _ => None,
    })
}

fn local_hostname(host: &str) -> bool {
    let normalized = host
        .trim_matches(['[', ']'])
        .strip_suffix('.')
        .unwrap_or_else(|| host.trim_matches(['[', ']']));
    normalized.eq_ignore_ascii_case("localhost")
        || normalized
            .parse::<IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

fn allow(reason: &str) -> AuthDecision {
    AuthDecision {
        allowed: true,
        reason: reason.to_string(),
        remediation: None,
    }
}

fn deny(reason: &str, remediation: &str) -> AuthDecision {
    AuthDecision {
        allowed: false,
        reason: reason.to_string(),
        remediation: Some(remediation.to_string()),
    }
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut diff = 0u8;
    for (a, b) in left.iter().zip(right.iter()) {
        diff |= a ^ b;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bearer_header_is_parsed() -> Result<(), AuthError> {
        assert_eq!(bearer_token("Bearer abc")?, "abc");
        Ok(())
    }

    #[test]
    fn token_mismatch_is_denied() {
        let policy = AuthPolicy {
            enabled: true,
            token_env: "OPENIRL_DASHBOARD_TOKEN".to_string(),
            allow_loopback_without_token: false,
            require_for_lan: true,
        };
        let decision =
            verify_authorization_header(&policy, Some("secret"), Some("Bearer other"), true);
        assert!(!decision.allowed);
    }

    #[test]
    fn lan_control_requires_configured_token_even_when_auth_is_disabled() {
        let policy = AuthPolicy {
            enabled: false,
            token_env: "OPENIRL_DASHBOARD_TOKEN".to_string(),
            allow_loopback_without_token: true,
            require_for_lan: true,
        };
        let decision = verify_authorization_header(&policy, None, None, false);
        assert!(!decision.allowed);
        assert_eq!(decision.reason, "token-not-configured");
    }

    #[test]
    fn configured_token_allows_authenticated_control() {
        let policy = AuthPolicy {
            enabled: true,
            token_env: "OPENIRL_DASHBOARD_TOKEN".to_string(),
            allow_loopback_without_token: false,
            require_for_lan: true,
        };
        let decision =
            verify_authorization_header(&policy, Some("secret"), Some("Bearer secret"), false);
        assert!(decision.allowed);
        assert_eq!(decision.reason, "token-match");
    }

    #[test]
    fn same_origin_matches_request_host() {
        assert!(browser_origin_is_same_origin(
            Some("http://127.0.0.1:7707"),
            Some("127.0.0.1:7707")
        ));
        assert!(!browser_origin_is_same_origin(
            Some("https://example.test"),
            Some("127.0.0.1:7707")
        ));
        assert!(browser_origin_is_same_origin(
            Some("http://localhost"),
            Some("localhost:80")
        ));
    }

    #[test]
    fn tokenless_local_origin_must_name_configured_listener() -> Result<(), std::net::AddrParseError>
    {
        let bind = "127.0.0.1:7707".parse()?;
        assert!(browser_request_is_trusted_local(
            Some("http://localhost:7707"),
            Some("localhost:7707"),
            bind
        ));
        assert!(browser_request_is_trusted_local(
            Some("http://127.0.0.1:7707"),
            Some("127.0.0.1:7707"),
            bind
        ));
        assert!(!browser_request_is_trusted_local(
            Some("http://untrusted.example:7707"),
            Some("untrusted.example:7707"),
            bind
        ));
        assert!(!browser_request_is_trusted_local(
            Some("https://localhost:7707"),
            Some("localhost:7707"),
            bind
        ));
        Ok(())
    }

    #[test]
    fn public_listener_never_qualifies_for_tokenless_host() -> Result<(), std::net::AddrParseError>
    {
        assert!(!request_host_is_trusted_local(
            Some("localhost:7707"),
            "0.0.0.0:7707".parse()?
        ));
        Ok(())
    }

    #[test]
    fn cross_origin_requires_explicit_allowlist() {
        let allowed = vec!["https://dashboard.example.test".to_string()];
        assert!(browser_origin_is_allowed(
            Some("https://dashboard.example.test"),
            Some("127.0.0.1:7707"),
            &allowed
        ));
        assert!(!browser_origin_is_allowed(
            Some("https://attacker.example.test"),
            Some("127.0.0.1:7707"),
            &allowed
        ));
    }

    #[test]
    fn null_origin_is_rejected() {
        assert!(!browser_origin_is_allowed(
            Some("null"),
            Some("127.0.0.1:7707"),
            &[]
        ));
    }
}
