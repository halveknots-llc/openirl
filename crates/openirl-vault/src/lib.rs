//! Secret redaction and future local vault utilities.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

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
    if value.len() <= 4 {
        return "<redacted>".to_string();
    }
    let suffix = &value[value.len().saturating_sub(4)..];
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
}
