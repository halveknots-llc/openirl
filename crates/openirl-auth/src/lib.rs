//! Dashboard authentication primitives for the local-first control plane.

use serde::{Deserialize, Serialize};
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
}
