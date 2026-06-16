# Maintainer Checks

Use this page when preparing public source changes, alpha packages, or release candidates. It keeps validation claims tied to commands and live dependency evidence.

## Required Source Checks

```bash
python3 scripts/static_validate.py
python3 scripts/audit/handoff_audit.py
python3 scripts/security/security-audit-smoke.py
cargo deny check
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo xtask ci
```

## Runtime Evidence

Run live checks only when the named dependency is actually available:

1. OBS WebSocket smoke against OBS Studio 28+ with authentication enabled.
2. MediaMTX local ingest using SRT and RTMP test publishers.
3. Moblin, IRL Pro, Larix, or BELABOX profile import on real devices or apps.
4. Brownout, BRB, backup ingest, recovery, and support-bundle export.
5. Relay, SRTLA, tunnel, WebRTC, and Windows package scripts on matching hosts.

## Feature Areas to Preserve

- OBS reconciliation and source transforms
- Local MediaMTX, SRT, and RTMP ingest
- Mobile encoder QR/profile generation
- Mobile dashboard control room
- Auth and remote-access guardrails
- Brownout engine and recovery hysteresis
- OBS output and destination health
- Support bundles and timeline diagnostics
- Backup ingest and multi-source failover
- Alerts and moderator operations
- SRTLA bonding compatibility
- Self-hosted relay workflows
- NAT traversal and tunnel integrations
- Public beta package, docs, security, release, WebRTC preview, vertical scenes, and plugin API

## Claim Rules

- Do not claim live OBS, MediaMTX, mobile encoder, BELABOX, SRTLA, tunnel, relay, WebRTC, or Windows package success from source checks alone.
- Do not relax validation scripts to hide a release blocker.
- Do not include stream keys, dashboard tokens, OBS passwords, SRT passphrases, private relay credentials, or unreviewed support bundles in public issues or release artifacts.
