# Support Bundles

Support bundles collect enough redacted evidence for maintainers to diagnose setup, ingest, metrics, OBS, relay, and field-report issues without asking operators to paste raw production logs.

## Source Validation

```bash
python3 scripts/security/security-audit-smoke.py
cargo test --package openirl-artifacts
cargo run --package openirl-agent -- artifacts support-bundle --config config/openirl.example.toml
```

Expected evidence:

- generated JSON contains redacted config and validation state
- stream keys, SRT passphrases, dashboard tokens, OBS passwords, relay credentials, bearer tokens, and credential-bearing URLs are scrubbed
- private IP redaction follows `support_bundle_redact_ips`
- field-report markdown is scrubbed before export

## Review Before Sharing

Operators should inspect every bundle before attaching it to an issue. In public issues, include only the smallest redacted excerpt required to explain the failure. Use private vulnerability reporting for security-sensitive behavior.

## Useful Bundle Context

Include:

- command used to start OpenIRL
- config path, with secrets in environment variables
- recent health or metrics transition
- OBS action attempted
- encoder app or relay path involved
- validation commands already run

## Current Boundary

Automated redaction canaries prove known secret patterns. They do not replace human review when bundles include production timelines, location-adjacent details, screenshots, or unusual vendor logs.
