# OpenIRL Handoff Tasks

Use this package as a feature-oriented Rust repository. Do not reintroduce numbered development pass labels in filenames, docs, APIs, or status fields.

## Required first checks

```bash
python3 scripts/static_validate.py
python3 scripts/audit/handoff_audit.py
cargo xtask ci
```

## Primary implementation validation

1. Compile the workspace and resolve Rust compiler findings.
2. Run the OBS WebSocket smoke script against OBS Studio 28+ with authentication enabled.
3. Validate MediaMTX local ingest using SRT and RTMP test publishers.
4. Scan generated profiles on Moblin and IRL Pro devices.
5. Verify brownout, BRB, backup ingest, recovery, and support-bundle export.
6. Review redaction outputs before sharing field reports.
7. Build the public-beta source package and checksum.

## Feature areas to preserve

- OBS reconciliation and source transforms
- Local MediaMTX/SRT/RTMP ingest
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
