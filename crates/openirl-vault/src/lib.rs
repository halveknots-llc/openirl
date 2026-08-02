//! Secret redaction and future local vault utilities.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use percent_encoding::percent_decode_str;
use regex::Regex;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{path::Path, sync::OnceLock};

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
    redact_stream_path(&redact_url(input))
}

/// Redacts URL userinfo and every sensitive query value while preserving shape.
#[must_use]
pub fn redact_url(input: &str) -> String {
    let without_userinfo = redact_url_userinfo(input);
    let (before_fragment, fragment) = without_userinfo
        .split_once('#')
        .map_or((without_userinfo.as_str(), None), |(before, fragment)| {
            (before, Some(fragment))
        });
    let Some((base, query)) = before_fragment.split_once('?') else {
        return without_userinfo;
    };
    let redacted_query = query
        .split('&')
        .map(redact_query_pair)
        .collect::<Vec<_>>()
        .join("&");
    let mut output = format!("{base}?{redacted_query}");
    if let Some(fragment) = fragment {
        output.push('#');
        output.push_str(fragment);
    }
    output
}

/// Redacts values supplied through sensitive command-line arguments.
#[must_use]
pub fn redact_command_args(args: &[String]) -> Vec<String> {
    let mut redacted = Vec::with_capacity(args.len());
    let mut redact_next = false;
    for arg in args {
        if redact_next {
            redacted.push("<redacted>".to_string());
            redact_next = false;
            continue;
        }

        if let Some((name, _value)) = arg.split_once('=') {
            if sensitive_name(name.trim_start_matches('-')) {
                redacted.push(format!("{name}=<redacted>"));
                continue;
            }
        } else if arg.starts_with('-') && sensitive_name(arg.trim_start_matches('-')) {
            redacted.push(arg.clone());
            redact_next = true;
            continue;
        }

        redacted.push(redact_stream_url(arg));
    }
    redacted
}

fn redact_url_userinfo(input: &str) -> String {
    let Some(scheme_end) = input.find("://") else {
        return input.to_string();
    };
    let authority_start = scheme_end + 3;
    let authority_end = input[authority_start..]
        .find(['/', '?', '#'])
        .map_or(input.len(), |relative| authority_start + relative);
    let authority = &input[authority_start..authority_end];
    let Some(userinfo_end) = authority.rfind('@') else {
        return input.to_string();
    };
    let host_start = authority_start + userinfo_end + 1;
    let mut output = String::with_capacity(input.len());
    output.push_str(&input[..authority_start]);
    output.push_str("<redacted-userinfo>@");
    output.push_str(&input[host_start..]);
    output
}

fn redact_query_pair(pair: &str) -> String {
    let Some((key, _value)) = pair.split_once('=') else {
        return pair.to_string();
    };
    if sensitive_name(&percent_decode_str(key).decode_utf8_lossy()) {
        format!("{key}=<redacted>")
    } else {
        pair.to_string()
    }
}

fn sensitive_name(value: &str) -> bool {
    let normalized = value.trim().to_ascii_lowercase().replace(['-', '.'], "_");
    if normalized.ends_with("_env") || normalized.contains("without_token") {
        return false;
    }
    let compact = normalized.replace('_', "");
    matches!(
        normalized.as_str(),
        "auth" | "authorization" | "key" | "secret" | "signature" | "token"
    ) || normalized.contains("password")
        || normalized.contains("passphrase")
        || normalized.ends_with("_secret")
        || normalized.ends_with("_token")
        || compact.contains("streamkey")
        || compact.contains("apikey")
        || compact.contains("privatekey")
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
            let is_env_pair =
                map.get("key").and_then(Value::as_str).is_some() && map.contains_key("value");
            for (key, child) in map {
                if support_bundle_secret_key(key)
                    || (is_env_pair && key == "value")
                    || (support_bundle_path_key(key)
                        && child.as_str().is_some_and(is_absolute_path))
                {
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

fn support_bundle_path_key(key: &str) -> bool {
    let key = key.to_ascii_lowercase().replace('-', "_");
    key == "path"
        || key == "root_dir"
        || key == "working_dir"
        || key == "resolved_path"
        || key.ends_with("_path")
        || key.ends_with("_dir")
}

fn is_absolute_path(value: &str) -> bool {
    Path::new(value).is_absolute()
}

static PRIVATE_KEY_RE: OnceLock<Option<Regex>> = OnceLock::new();
static BEARER_RE: OnceLock<Option<Regex>> = OnceLock::new();
static URL_USERINFO_RE: OnceLock<Option<Regex>> = OnceLock::new();
static QUERY_SECRET_RE: OnceLock<Option<Regex>> = OnceLock::new();
static ASSIGNMENT_SECRET_RE: OnceLock<Option<Regex>> = OnceLock::new();
static RTMP_PATH_RE: OnceLock<Option<Regex>> = OnceLock::new();
static SRT_PATH_RE: OnceLock<Option<Regex>> = OnceLock::new();
static IPV4_RE: OnceLock<Option<Regex>> = OnceLock::new();
static BRACKETED_IPV6_RE: OnceLock<Option<Regex>> = OnceLock::new();

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
    redacted = redact_stream_path(&redacted);

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
    let redacted_ipv6 =
        if let Some(regex) = cached_regex(&BRACKETED_IPV6_RE, r"\[(?P<ip>[0-9A-Fa-f:]{2,})\]") {
            regex
                .replace_all(input, |captures: &regex::Captures<'_>| {
                    let value = captures.name("ip").map_or("", |capture| capture.as_str());
                    if value == "::1" || value.eq_ignore_ascii_case("0:0:0:0:0:0:0:1") {
                        format!("[{value}]")
                    } else {
                        "<redacted-ipv6>".to_string()
                    }
                })
                .into_owned()
        } else {
            input.to_string()
        };
    let Some(regex) = cached_regex(
        &IPV4_RE,
        r"\b(?:(?:25[0-5]|2[0-4]\d|1?\d?\d)\.){3}(?:25[0-5]|2[0-4]\d|1?\d?\d)\b",
    ) else {
        return redacted_ipv6;
    };

    regex
        .replace_all(&redacted_ipv6, |captures: &regex::Captures<'_>| {
            let value = captures.get(0).map_or("", |capture| capture.as_str());
            if value.starts_with("127.") || value == "0.0.0.0" {
                value.to_string()
            } else {
                "<redacted-ip>".to_string()
            }
        })
        .into_owned()
}

fn redact_stream_path(input: &str) -> String {
    let output = replace_support_pattern(
        input,
        &RTMP_PATH_RE,
        r"(?i)(?P<prefix>\b(?:rtmp|rtmps)://[^/\s?#]+/[^/\s?#]+/)[^/\s?#]+",
        "${prefix}<redacted>",
    );
    replace_support_pattern(
        &output,
        &SRT_PATH_RE,
        r"(?i)(?P<prefix>\b(?:srt|srtla)://[^/\s?#]+/)[^/\s?#]+",
        "${prefix}<redacted>",
    )
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
    fn redacts_every_duplicate_and_encoded_query_secret() {
        let redacted = redact_url(
            "http://operator:password@localhost:9998/metrics?token=first&%74oken=second&safe=value#summary",
        );
        assert_eq!(
            redacted,
            "http://<redacted-userinfo>@localhost:9998/metrics?token=<redacted>&%74oken=<redacted>&safe=value#summary"
        );
        assert!(!redacted.contains("first"));
        assert!(!redacted.contains("second"));
        assert!(!redacted.contains("password"));
    }

    #[test]
    fn command_arguments_normalize_hyphenated_secret_names() {
        let args = vec![
            "--stream-key=alpha".to_string(),
            "--api-key".to_string(),
            "bravo".to_string(),
            "--endpoint=srt://localhost:9000?token=charlie".to_string(),
            "--port=9000".to_string(),
        ];
        assert_eq!(
            redact_command_args(&args),
            vec![
                "--stream-key=<redacted>".to_string(),
                "--api-key".to_string(),
                "<redacted>".to_string(),
                "--endpoint=srt://localhost:9000?token=<redacted>".to_string(),
                "--port=9000".to_string(),
            ]
        );
    }

    #[test]
    fn redacts_path_based_stream_credentials() {
        let redacted = redact_stream_url("rtmp://relay.example/live/path-secret");
        assert_eq!(redacted, "rtmp://relay.example/live/<redacted>");
        assert!(!redacted.contains("path-secret"));
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
    fn support_text_redacts_ipv6_and_path_credentials() {
        let redacted = redact_support_text(
            "rtmp://relay.example/live/path-secret [2001:db8::42] [::1]",
            true,
        );
        assert!(!redacted.contains("path-secret"));
        assert!(redacted.contains("<redacted-ipv6>"));
        assert!(redacted.contains("[::1]"));
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

    #[test]
    fn support_json_redacts_relay_environment_values() {
        let payload = serde_json::json!({
            "env": [{"key": "OPENIRL_SRT_PASSPHRASE", "value": "relay-secret"}]
        });
        let redacted = scrub_support_bundle_value(payload, false);
        assert_eq!(redacted["env"][0]["key"], "OPENIRL_SRT_PASSPHRASE");
        assert_eq!(redacted["env"][0]["value"], "<redacted>");
    }

    #[test]
    fn contributor_redaction_fixture_removes_every_canary() -> Result<(), Box<dyn std::error::Error>>
    {
        let payload: Value = serde_json::from_str(include_str!(
            "../../../fixtures/contributing/redaction-canary.sample.json"
        ))?;
        let redacted = serde_json::to_string(&scrub_support_bundle_value(payload, true))?;
        for canary in [
            "synthetic-dashboard-canary",
            "synthetic-obs-canary",
            "synthetic-srt-canary",
            "192.0.2.10",
        ] {
            assert!(!redacted.contains(canary));
        }
        assert!(redacted.contains("<redacted>"));
        assert!(redacted.contains("<redacted-ip>"));
        Ok(())
    }

    #[test]
    fn support_json_redacts_absolute_artifact_paths() {
        let payload = serde_json::json!({
            "root_dir": "/private/operator/support-bundles/abc",
            "relative_path": "artifacts/report.json"
        });
        let redacted = scrub_support_bundle_value(payload, false);
        assert_eq!(redacted["root_dir"], "<redacted>");
        assert_eq!(redacted["relative_path"], "artifacts/report.json");
    }
}
