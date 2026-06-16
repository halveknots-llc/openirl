# Local Ingest

OpenIRL models contribution ingest as a local routing problem first. MediaMTX, SRT, RTMP, and related tools stay process-bound while Rust coordinates config, metrics, readiness, and operator evidence.

## Source Validation

```bash
cargo run --package openirl-agent -- check-config --config config/openirl.example.toml
cargo test --package openirl-metrics
cargo test --package openirl-relay-control
```

Expected evidence:

- the ingest and relay settings are visible in the redacted config
- metric parsers accept MediaMTX and SRTLA-shaped samples
- relay process plans stay disabled unless explicitly configured

## Live MediaMTX Validation

Run this only when MediaMTX and a publisher path are available:

```bash
scripts/ingest/local-ingest-smoke.sh
```

A useful field note includes:

- protocol used: SRT, RTMP, or another configured path
- publisher command or encoder app, with secrets removed
- MediaMTX path status before and after publish
- OBS media source URL or dashboard metrics used to confirm contribution media

## Failure Modes

Most first-run failures are bind conflicts, firewall rules, wrong SRT mode, mismatched path names, or OBS pointing at a stale URL. Capture a support bundle after reproducing the issue so maintainers can inspect redacted config and timeline evidence.

## Current Boundary

Source checks prove configuration and parser behavior. They do not prove that this machine can receive a real mobile encoder until MediaMTX and a publisher have been exercised.
