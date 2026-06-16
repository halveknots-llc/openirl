//! Secret redaction and future local vault utilities.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use regex::Regex;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::OnceLock;

/// Local secret reference. This is not a full vault yet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretRef {
    /// Stable secret label.
    pub label: String,
    /// Redacted preview.
    pub redacted: String,
    /// SHA-256 fingerprint, URL-safe base64 encoded.
    pub fingerprint: String,
}

/// Wraps a secret and computes non-sensitive metadata.
#[must_use]
pub fn describe_secret(label: impl Into<String>, secret: &SecretString) -> SecretRef {
    let raw = secret.expose_secret();
    let digest = Sha256::digest(raw.as_bytes());
    SecretRef {
        label: label.into(),
        redacted: redact_value(raw),
        fingerprint: URL_SAFE_NO_PAD.encode(digest),
    }
}

/// Redacts a generic secret-like value.
#[must_use]
pub fn redact_value(value: &str) -> String {
    if value.is_empty() {
        return "<empty>".to_string();
    }
    if value.chars().count() <= 4 {
        return "<redacted>".to_string();
    }
    let suffix = value
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    format!("<redacted:{suffix}>")
}

/// Redacts stream-key-like query params from an ingest URL.
#[must_use]
pub fn redact_stream_url(input: &str) -> String {
    let sensitive = ["passphrase", "stream_key", "key", "token", "password"];
    let mut output = input.to_string();
    for key in sensitive {
        output = redact_query_key(&output, key);
    }
    output
}

fn redact_query_key(input: &str, key: &str) -> String {
    let pattern = format!("{key}=");
    let Some(start) = input.find(&pattern) else {
        return input.to_string();
    };
    let value_start = start + pattern.len();
    let value_end = input[value_start..]
        .find('&')
        .map_or(input.len(), |relative| value_start + relative);
    let mut output = String::with_capacity(input.len());
    output.push_str(&input[..value_start]);
    output.push_str("<redacted>");
    output.push_str(&input[value_end..]);
    output
}

/// Redacts a support-bundle JSON payload without changing non-sensitive shape.
#[must_use]
pub fn scrub_support_bundle_value(mut value: Value, redact_ips: bool) -> Value {
    scrub_json_value(&mut value, redact_ips);
    value
}

fn scrub_json_value(value: &mut Value, redact_ips: bool) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if support_bundle_secret_key(key) {
                    *child = Value::String("<redacted>".to_string());
                } else {
                    scrub_json_value(child, redact_ips);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                scrub_json_value(item, redact_ips);
            }
        }
        Value::String(text) => {
            *text = redact_support_text(text, redact_ips);
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

fn support_bundle_secret_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase().replace('-', "_");
    if key.ends_with("_env") || key.contains("without_token") {
        return false;
    }

    key == "token"
        || key.ends_with("_token")
        || key.contains("access_token")
        || key.contains("refresh_token")
        || key.contains("dashboard_token")
        || key.contains("password")
        || key.contains("passphrase")
        || key.contains("stream_key")
        || key.contains("streamkey")
        || key.contains("private_key")
        || key.contains("authorization")
        || key.contains("secret")
}

static PRIVATE_KEY_RE: OnceLock<Option<Regex>> = OnceLock::new();
static BEARER_RE: OnceLock<Option<Regex>> = OnceLock::new();
static URL_USERINFO_RE: OnceLock<Option<Regex>> = OnceLock::new();
static QUERY_SECRET_RE: OnceLock<Option<Regex>> = OnceLock::new();
static ASSIGNMENT_SECRET_RE: OnceLock<Option<Regex>> = OnceLock::new();
static IPV4_RE: OnceLock<Option<Regex>> = OnceLock::new();

fn cached_regex(
    cell: &'static OnceLock<Option<Regex>>,
    pattern: &'static str,
) -> Option<&'static Regex> {
    cell.get_or_init(|| Regex::new(pattern).ok()).as_ref()
}

/// Redacts known support-bundle, field-report, and log secret patterns.
#[must_use]
pub fn redact_support_text(input: &str, redact_ips: bool) -> String {
    let mut redacted = input.to_string();
    redacted = replace_support_pattern(
        &redacted,
        &PRIVATE_KEY_RE,
        r"(?is)-----BEGIN [^-]*PRIVATE KEY-----.*?-----END [^-]*PRIVATE KEY-----",
        "[redacted-private-key]",
    );
    redacted = replace_support_pattern(
        &redacted,
        &BEARER_RE,
        r"(?i)Bearer\s+[A-Za-z0-9._~+/=-]+",
        "Bearer <redacted>",
    );
    redacted = replace_support_pattern(
        &redacted,
        &URL_USERINFO_RE,
        r"(?i)(?P<prefix>[a-z][a-z0-9+.-]*://[^/\s:@]+:)[^@\s/]+@",
        "${prefix}<redacted>@",
    );
    redacted = replace_support_pattern(
        &redacted,
        &QUERY_SECRET_RE,
        r#"(?i)(?P<prefix>[?&](?:passphrase|token|stream[_-]?key|password|secret|authorization|auth)=)[^&\s"'<>)]*"#,
        "${prefix}<redacted>",
    );
    redacted = replace_support_pattern(
        &redacted,
        &ASSIGNMENT_SECRET_RE,
        r#"(?im)(?P<prefix>\b(?:password|passphrase|stream[_-]?key|secret|token|bearer[_-]?token|access[_-]?token|refresh[_-]?token|dashboard[_-]?token|obs[_-]?password)\b\s*[:=]\s*)["']?[^"',\n\r}]+["']?"#,
        "${prefix}<redacted>",
    );

    if redact_ips {
        redact_ip_addresses(&redacted)
    } else {
        redacted
    }
}

fn replace_support_pattern(
    input: &str,
    cell: &'static OnceLock<Option<Regex>>,
    pattern: &'static str,
    replacement: &str,
) -> String {
    if let Some(regex) = cached_regex(cell, pattern) {
        regex.replace_all(input, replacement).into_owned()
    } else {
        input.to_string()
    }
}

fn redact_ip_addresses(input: &str) -> String {
    let Some(regex) = cached_regex(
        &IPV4_RE,
        r"\b(?:(?:25[0-5]|2[0-4]\d|1?\d?\d)\.){3}(?:25[0-5]|2[0-4]\d|1?\d?\d)\b",
    ) else {
        return input.to_string();
    };

    regex
        .replace_all(input, |captures: &regex::Captures<'_>| {
            let value = captures.get(0).map_or("", |capture| capture.as_str());
            if value.starts_with("127.") || value == "0.0.0.0" {
                value.to_string()
            } else {
                "<redacted-ip>".to_string()
            }
        })
        .into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_passphrase() {
        let redacted =
            redact_stream_url("srt://x:9000?streamid=main&passphrase=secret&latency=1800");
        assert!(redacted.contains("passphrase=<redacted>"));
        assert!(!redacted.contains("secret"));
    }

    #[test]
    fn describes_secret_without_exposing_full_value() {
        let secret = SecretString::from("topsecret1234".to_string());
        let described = describe_secret("test", &secret);
        assert_eq!(described.label, "test");
        assert!(!described.redacted.contains("topsecret"));
        assert!(!described.fingerprint.is_empty());
    }

    #[test]
    fn redacts_non_ascii_secret_without_byte_slicing() {
        assert_eq!(redact_value("秘密値1234"), "<redacted:1234>");
        assert_eq!(redact_value("秘密"), "<redacted>");
    }

    #[test]
    fn support_text_redacts_tokens_urls_and_ips() {
        let redacted = redact_support_text(
            "Authorization: Bearer abc.123\nsrt://relay:9000?passphrase=secret\n--token=field-token\nrelay=10.23.45.67",
            true,
        );
        assert!(redacted.contains("Bearer <redacted>"));
        assert!(redacted.contains("passphrase=<redacted>"));
        assert!(redacted.contains("--token=<redacted>"));
        assert!(redacted.contains("relay=<redacted-ip>"));
        assert!(!redacted.contains("abc.123"));
        assert!(!redacted.contains("secret"));
        assert!(!redacted.contains("field-token"));
        assert!(!redacted.contains("10.23.45.67"));
    }

    #[test]
    fn support_json_redacts_secret_keys_but_keeps_env_names() {
        let payload = serde_json::json!({
            "dashboard_token": "super-secret",
            "dashboard_token_env": "OPENIRL_DASHBOARD_TOKEN",
            "note": "OBS password = obs-password-canary",
            "host": "10.23.45.67"
        });
        let redacted = scrub_support_bundle_value(payload, true);
        assert_eq!(redacted["dashboard_token"], "<redacted>");
        assert_eq!(redacted["dashboard_token_env"], "OPENIRL_DASHBOARD_TOKEN");
        assert_eq!(redacted["note"], "OBS password = <redacted>");
        assert_eq!(redacted["host"], "<redacted-ip>");
    }
}
