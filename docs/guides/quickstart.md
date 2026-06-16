# Quickstart

This guide starts with checks that do not require OBS, MediaMTX, mobile encoders, BELABOX, SRTLA, tunnels, or Windows packaging. Move to live checks only when those dependencies are available.

## 1. Install the Toolchain

Install the Rust toolchain named in the repository:

```bash
rustup show
cargo --version
```

The workspace uses Rust 2024 and the `rust-version` declared in `Cargo.toml`.

## 2. Validate the Source Package

Run the local source checks from the repository root:

```bash
python3 scripts/static_validate.py
python3 scripts/audit/handoff_audit.py
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo xtask ci
```

These commands prove source shape, formatting, lint posture, Rust tests, and repository validation. They do not prove that a specific OBS profile, MediaMTX host, mobile phone, relay host, tunnel, BELABOX, SRTLA deployment, or Windows package works in your environment.

## 3. Inspect the Example Config

```bash
cargo run --package openirl-agent -- check-config --config config/openirl.example.toml
```

The example config keeps the dashboard on `127.0.0.1:7707`, keeps relay execution disabled until configured, and reads secrets from environment variables rather than checked-in files.

## 4. Start the Local Agent

```bash
cargo run --package openirl-agent -- serve --config config/openirl.example.toml
```

Open the dashboard:

```text
http://127.0.0.1:7707
```

If dashboard auth is enabled, set the token in the environment named by `security.dashboard_token_env` before starting the agent.

## 5. Run the API Smoke Check

In another shell:

```bash
python3 scripts/smoke/api_smoke.py
```

When `OPENIRL_DASHBOARD_TOKEN` is present, the smoke script sends it as a bearer token. Do not paste that token into issues, logs, screenshots, or support bundles.

## 6. Materialize Local Artifacts

Generate reviewable local artifacts without switching to live OBS control:

```bash
cargo run --package openirl-agent -- artifacts materialize-fallback
cargo run --package openirl-agent -- artifacts obs-template --materialize
cargo run --package openirl-agent -- v1 summary
```

Review generated artifacts before sharing them.

## 7. Move to Live OBS and MediaMTX Checks

Run these only when the named dependency is available:

```bash
scripts/obs/reconcile-smoke.sh
scripts/ingest/local-ingest-smoke.sh
scripts/mobile/profile-compat-smoke.sh
scripts/relay/self-hosted-relay-smoke.sh
scripts/tunnels/tunnel-readiness-smoke.sh
```

Use the matching PowerShell scripts on Windows where available. Record the exact OBS, MediaMTX, encoder, relay, tunnel, and Windows host versions used.

## 8. Troubleshooting

- If the dashboard does not load, confirm the agent is still running and that the configured bind address is local.
- If contribution media is missing, use [No-video troubleshooting](../troubleshooting/no-video.md).
- If live OBS control fails, confirm OBS WebSocket is enabled and password-protected.
- If generated profiles do not import, include the encoder app/device version in a field report.

## Next Reading

- [Documentation index](../README.md)
- [Validation](../VALIDATION.md)
- [Security policy](../../SECURITY.md)
- [Relay guide](relay.md)
