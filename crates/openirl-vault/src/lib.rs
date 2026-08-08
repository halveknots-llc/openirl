//! Secret redaction and future local vault utilities.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use percent_encoding::percent_decode_str;
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
    let mut output = if let Some((base, query)) = before_fragment.split_once('?') {
        let redacted_query = query
            .split('&')
            .map(redact_query_pair)
            .collect::<Vec<_>>()
            .join("&");
        format!("{base}?{redacted_query}")
    } else {
        before_fragment.to_string()
    };
    if let Some(fragment) = fragment {
        output.push('#');
        output.push_str(&redact_fragment(fragment));
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

        if let Some((name, separator, _value)) = split_option_value(arg) {
            if sensitive_name(option_name(name)) {
                redacted.push(format!("{name}{separator}<redacted>"));
                continue;
            }
        } else if is_option(arg) && sensitive_name(option_name(arg)) {
            redacted.push(arg.clone());
            redact_next = true;
            continue;
        }

        let path_safe = redact_local_path(arg);
        let url_safe = redact_stream_url(&path_safe);
        redacted.push(redact_support_text(&url_safe, false));
    }
    redacted
}

fn is_option(value: &str) -> bool {
    value.starts_with('-') || (value.starts_with('/') && !value[1..].contains(['/', '\\']))
}

fn option_name(value: &str) -> &str {
    value.trim_start_matches(['-', '/'])
}

fn split_option_value(value: &str) -> Option<(&str, char, &str)> {
    if !is_option(value) {
        return None;
    }
    let index = value.find(['=', ':'])?;
    let separator = value[index..].chars().next()?;
    Some((
        &value[..index],
        separator,
        &value[index + separator.len_utf8()..],
    ))
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

fn redact_fragment(fragment: &str) -> String {
    let redacted = fragment
        .split('&')
        .map(redact_query_pair)
        .collect::<Vec<_>>()
        .join("&");
    if redacted != fragment {
        return redacted;
    }
    if let Some((name, _value)) = fragment.split_once(':') {
        if sensitive_name(&percent_decode_str(name).decode_utf8_lossy()) {
            return format!("{name}:<redacted>");
        }
    }
    if fragment
        .trim_start()
        .to_ascii_lowercase()
        .starts_with("bearer ")
    {
        return "<redacted-fragment>".to_string();
    }
    fragment.to_string()
}

fn sensitive_name(value: &str) -> bool {
    let tokens = normalized_name_tokens(value);
    if tokens.last().is_some_and(|token| token == "env") || tokens == ["without", "token"] {
        return false;
    }
    tokens.iter().any(|token| {
        matches!(
            token.as_str(),
            "auth"
                | "authorization"
                | "credential"
                | "credentials"
                | "key"
                | "password"
                | "passphrase"
                | "secret"
                | "signature"
                | "token"
        )
    })
}

fn normalized_name_tokens(value: &str) -> Vec<String> {
    let characters = value.trim().chars().collect::<Vec<_>>();
    let mut normalized = String::with_capacity(value.len());
    for (index, character) in characters.iter().copied().enumerate() {
        if !character.is_ascii_alphanumeric() {
            if !normalized.ends_with(' ') {
                normalized.push(' ');
            }
            continue;
        }
        let previous = index.checked_sub(1).and_then(|item| characters.get(item));
        let next = characters.get(index + 1);
        let starts_word = character.is_ascii_uppercase()
            && (previous.is_some_and(|item| item.is_ascii_lowercase() || item.is_ascii_digit())
                || (previous.is_some_and(|item| item.is_ascii_uppercase())
                    && next.is_some_and(|item| item.is_ascii_lowercase())));
        if starts_word && !normalized.ends_with(' ') {
            normalized.push(' ');
        }
        normalized.push(character.to_ascii_lowercase());
    }
    normalized
        .split_ascii_whitespace()
        .map(ToOwned::to_owned)
        .collect()
}

/// Returns true when a string uses private or host-specific path syntax.
#[must_use]
pub fn is_private_path(value: &str) -> bool {
    let value = value.trim();
    if value.is_empty() {
        return false;
    }
    let bytes = value.as_bytes();
    if value.starts_with(['/', '\\', '~'])
        || (bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':')
    {
        return true;
    }
    value.split(['/', '\\']).any(|component| component == "..")
}

/// Redacts private path syntax without resolving or accessing the path.
#[must_use]
pub fn redact_local_path(value: &str) -> String {
    if is_private_path(value) {
        "<redacted-local-path>".to_string()
    } else {
        value.to_string()
    }
}

/// Validates a portable repository-relative evidence path lexically.
#[must_use]
pub fn is_repository_relative_path(value: &str) -> bool {
    !value.is_empty()
        && value == value.trim()
        && !is_private_path(value)
        && !value.contains(['\\', ':'])
        && !value
            .chars()
            .any(|character| character.is_ascii_control() || character.is_ascii_whitespace())
        && value
            .split('/')
            .all(|component| !component.is_empty() && !matches!(component, "." | ".."))
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
                    || (support_bundle_path_key(key) && child.as_str().is_some_and(is_private_path))
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
    fn redacts_camel_case_query_and_fragment_credentials() {
        let redacted = redact_url(
            "https://relay.example/api?accessToken=query-canary&region=west#apiKey=fragment-canary&view=summary",
        );
        assert_eq!(
            redacted,
            "https://relay.example/api?accessToken=<redacted>&region=west#apiKey=<redacted>&view=summary"
        );
        assert!(!redacted.contains("query-canary"));
        assert!(!redacted.contains("fragment-canary"));
    }

    #[test]
    fn preserves_benign_url_fragments_and_name_exceptions() {
        assert_eq!(
            redact_url("https://relay.example/docs?token_env=OPENIRL_TOKEN#summary"),
            "https://relay.example/docs?token_env=OPENIRL_TOKEN#summary"
        );
        assert_eq!(
            redact_url("https://relay.example/docs?withoutToken=true#section-heading"),
            "https://relay.example/docs?withoutToken=true#section-heading"
        );
        assert_eq!(
            redact_url("https://relay.example/docs?withoutTokenPassword=sensitive-canary#summary"),
            "https://relay.example/docs?withoutTokenPassword=<redacted>#summary"
        );
        assert_eq!(
            redact_url(
                "https://relay.example/docs?authenticationCredentials=sensitive-canary#summary"
            ),
            "https://relay.example/docs?authenticationCredentials=<redacted>#summary"
        );
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
    fn command_arguments_cover_prefix_colon_and_slash_forms() {
        let args = vec![
            "--authorization-header=Bearer command-canary".to_string(),
            "--password:colon-canary".to_string(),
            "/token:slash-canary".to_string(),
            "--accessToken".to_string(),
            "following-canary".to_string(),
            "/private/operator/config.yml".to_string(),
            "--port=9000".to_string(),
        ];
        let redacted = redact_command_args(&args);
        assert_eq!(
            redacted,
            vec![
                "--authorization-header=<redacted>",
                "--password:<redacted>",
                "/token:<redacted>",
                "--accessToken",
                "<redacted>",
                "<redacted-local-path>",
                "--port=9000",
            ]
        );
        let serialized = redacted.join(" ");
        for canary in [
            "command-canary",
            "colon-canary",
            "slash-canary",
            "following-canary",
            "/private/operator",
        ] {
            assert!(!serialized.contains(canary));
        }
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

    #[test]
    fn path_policy_is_cross_platform_and_lexical() {
        for private in [
            "/Users/operator/report.json",
            r"C:\\Users\\operator\\report.json",
            r"\\server\share\report.json",
            "../private/report.json",
            r"..\private\report.json",
            "~/report.json",
        ] {
            assert!(is_private_path(private), "expected private path: {private}");
            assert_eq!(redact_local_path(private), "<redacted-local-path>");
            assert!(!is_repository_relative_path(private));
        }
        for public in [
            "artifacts/report.json",
            "scripts/smoke/demo-mode-smoke.py",
            "not-run",
        ] {
            assert!(!is_private_path(public));
            assert!(is_repository_relative_path(public));
            assert_eq!(redact_local_path(public), public);
        }
        assert!(!is_repository_relative_path("scripts/smoke.py or .ps1"));
    }

    #[test]
    fn support_json_redacts_foreign_platform_paths() {
        let payload = serde_json::json!({
            "root_dir": r"C:\\Users\\operator\\OpenIRL",
            "resolved_path": r"\\server\share\support.json",
            "artifact_reference": "artifacts/report.json"
        });
        let redacted = scrub_support_bundle_value(payload, false);
        assert_eq!(redacted["root_dir"], "<redacted>");
        assert_eq!(redacted["resolved_path"], "<redacted>");
        assert_eq!(redacted["artifact_reference"], "artifacts/report.json");
    }
}
