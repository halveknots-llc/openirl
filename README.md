# OpenIRL

OpenIRL is a local-first control plane for IRL streaming. It gives streamers and production teams an operator-owned alternative to managed Cloud OBS stacks by pairing OBS automation, SRT/SRTLA-friendly ingest, mobile encoder profiles, brownout-aware scene switching, diagnostics, support bundles, and optional self-hosted relay workflows in one Rust workspace.

The goal is simple: keep the live show under the operator's control, make field failures explainable, and avoid turning OBS control, stream keys, or relay credentials into another hosted service dependency.

## Why OpenIRL Exists

IRL streams fail in messy ways: bonded links degrade, mobile encoders reconnect, relay hosts change, OBS keeps running while the contribution feed dies, and moderators need safe controls from a phone. Managed Cloud OBS platforms solve some of this by moving production into someone else's infrastructure. OpenIRL takes the opposite path.

OpenIRL runs on hardware you control. It supervises the local production stack, generates compatible mobile profiles, reacts to stream health, writes support evidence, and keeps live dependency claims tied to checks operators can rerun.

## What OpenIRL Does

| Area | OpenIRL capability |
| --- | --- |
| OBS control | Creates and switches local OBS scenes through a typed controller boundary with a WebSocket adapter. |
| Local ingest | Plans MediaMTX/SRT/RTMP ingest paths and keeps router configuration explicit. |
| Mobile encoders | Generates profiles for Moblin, IRL Pro, Larix, and BELABOX-oriented workflows. |
| Brownout handling | Classifies stream health, explains the decision, and recommends live, low-signal, BRB, backup, or offline scenes. |
| Relay workflows | Models local-direct, self-hosted relay, tunnel, SRTLA, and process-supervised media-router paths. |
| Dashboard | Serves a local mobile-friendly control room for setup, metrics, OBS actions, profile generation, and artifact export. |
| Security | Defaults to localhost, redacts secrets, blocks unsafe public bind, and separates local automation from live dependency validation. |
| Support evidence | Builds session reports, timelines, field evidence, issue payloads, and support-bundle data that maintainers can inspect. |

## Architecture

```text
Moblin / IRL Pro / Larix / BELABOX
          |
          | SRT / SRTLA / RTMP / RIST / WHIP
          v
MediaMTX / SRTLA helper / relay process
          |
          | metrics + contribution media
          v
OpenIRL Agent  ---- obs-websocket ----  OBS Studio
          |
          +-- local dashboard
          +-- encoder profile and QR generation
          +-- brownout and recovery decisions
          +-- support bundles and field reports
          +-- relay and tunnel readiness plans
```

OpenIRL intentionally keeps media engines process-bound. Rust coordinates configuration, readiness, health decisions, redaction, and operator controls; MediaMTX, SRTLA helpers, OBS, and tunnel tools continue doing the protocol-specific media work they are already good at.

## Current Status

OpenIRL is a private-alpha Rust package with automated validation for the local agent, API contracts, config model, feature catalog, profile generation, support artifacts, and safety rules.

The source package includes live smoke scripts for OBS, MediaMTX, relay, mobile encoder, tunnel, and Windows packaging environments. Those checks require the real external tools and devices. Do not treat automated Rust/API validation as proof that a specific OBS profile, MediaMTX host, mobile phone, BELABOX, SRTLA deployment, or Windows installer has passed in your environment until the matching smoke script has run there.

## Repository Layout

```text
apps/openirl-agent/          local API and dashboard server
apps/openirl-desktop/        desktop/tray shell entrypoint
services/openirl-relay/      relay process planning CLI
crates/openirl-core/         protocols, encoders, scenes, health states
crates/openirl-health/       brownout-aware health engine
crates/openirl-config/       config model and validation
crates/openirl-obs/          OBS controller trait and WebSocket adapter
crates/openirl-profiles/     Moblin, IRL Pro, Larix, and BELABOX profile generation
crates/openirl-metrics/      MediaMTX and SRTLA metric parsing
crates/openirl-session/      stream timeline and session reports
crates/openirl-artifacts/    fallback assets, OBS templates, support-bundle export
crates/openirl-v1/           public-beta feature catalog and package materializer
presets/                     encoder, relay, OBS, tunnel, moderation, and production presets
scripts/                     static validation and runtime smoke scripts
docs/                        guides, runbooks, feature docs, security, release checklist
deploy/                      MediaMTX, relay, and Windows deployment assets
fixtures/                    sample evidence and metric data
xtask/                       repository validation task runner
```

## Quickstart

### 1. Install Prerequisites

- Rust toolchain from `rust-toolchain.toml`
- OBS Studio with OBS WebSocket enabled
- MediaMTX when running local ingest checks
- Optional: Moblin, IRL Pro, Larix, or BELABOX for field profile validation

### 2. Validate the Source Package

```bash
python3 scripts/static_validate.py
python3 scripts/audit/handoff_audit.py
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo xtask ci
```

### 3. Inspect the Example Config

```bash
cargo run --package openirl-agent -- check-config --config config/openirl.example.toml
```

The example configuration binds the dashboard to `127.0.0.1:7707`, keeps relay execution disabled by default, redacts secrets in output, and uses a review-safe OBS adapter unless you explicitly configure live OBS WebSocket access.

### 4. Start the Local Agent

```bash
cargo run --package openirl-agent -- serve --config config/openirl.example.toml
```

Open the dashboard:

```text
http://127.0.0.1:7707
```

### 5. Run the API Smoke Check

```bash
python3 scripts/smoke/api_smoke.py
```

If dashboard auth is enabled and `OPENIRL_DASHBOARD_TOKEN` is configured, the smoke script sends `Authorization: Bearer <token>` automatically when that environment variable is present.

## Common Commands

```bash
# Product feature catalog
cargo run --package openirl-agent -- features
cargo run --package openirl-agent -- v1 summary
cargo run --package openirl-agent -- v1 features

# Agent and dashboard
cargo run --package openirl-agent -- serve --config config/openirl.example.toml
cargo run --package openirl-agent -- check-config --config config/openirl.example.toml

# Artifact generation
cargo run --package openirl-agent -- artifacts materialize-fallback
cargo run --package openirl-agent -- artifacts obs-template --materialize

# Full repository gate
cargo xtask ci
```

## Live Validation Scripts

Run these only in an environment with the named external dependency available.

| Script | Requires | Purpose |
| --- | --- | --- |
| `scripts/obs/reconcile-smoke.sh` | OBS Studio and OBS WebSocket | Verifies scene/template reconciliation against a real OBS profile. |
| `scripts/ingest/local-ingest-smoke.sh` | MediaMTX and a publisher path | Verifies local ingest routing and contribution media path. |
| `scripts/ingest/backup-failover-smoke.sh` | Primary and backup ingest paths | Exercises failover behavior and backup scene selection. |
| `scripts/mobile/profile-compat-smoke.sh` | Mobile encoder import flow | Checks generated profile compatibility with field devices. |
| `scripts/relay/self-hosted-relay-smoke.sh` | Relay host or local relay process | Verifies relay process plans and readiness behavior. |
| `scripts/relay/srtla2-compat-smoke.sh` | SRTLA-compatible tooling | Checks bonding-oriented configuration and metrics expectations. |
| `scripts/tunnels/tunnel-readiness-smoke.sh` | WireGuard, frp, or rathole setup | Checks tunnel readiness for CGNAT and no-public-IP users. |
| `scripts/webrtc/webrtc-preview-smoke.sh` | WHEP/WebRTC preview stack | Verifies producer preview planning and access boundaries. |
| `scripts/windows/build-alpha-portable.ps1` | Windows build host | Builds the Windows alpha package layout. |

## Security Model

OpenIRL controls production software, so the default posture is conservative:

- The agent binds to localhost unless LAN access is intentionally enabled.
- OBS WebSocket should always be password-protected.
- Public bind without auth is rejected by config validation.
- Dashboard tokens, stream keys, SRT passphrases, OBS passwords, and relay credentials are redacted from reports and support artifacts.
- Relay process execution is opt-in.
- Support bundles are designed for review before sharing.

See [docs/SECURITY.md](docs/SECURITY.md) for the threat model, role model, and support-bundle guidance.

## Configuration

Start with [config/openirl.example.toml](config/openirl.example.toml). The important defaults are:

- `api.bind = "127.0.0.1:7707"`
- `security.require_auth_outside_localhost = true`
- `security.dashboard_token_env = "OPENIRL_DASHBOARD_TOKEN"`
- `obs.password_env = "OPENIRL_OBS_PASSWORD"`
- `relay.enabled = false`
- `relay.supervisor_mode = "dry-run"`
- `metrics.source = "disabled"`

Switch to live OBS only when OBS WebSocket is enabled and password-protected. Switch relay supervision out of review mode only when the MediaMTX or relay executable path, ports, firewall rules, and credentials are deliberate.

## Hardware And Encoder Guides

- [Moblin](docs/hardware/moblin.md)
- [IRL Pro](docs/hardware/irl-pro.md)
- [BELABOX](docs/hardware/belabox.md)
- [Relay guide](docs/guides/relay.md)
- [Quickstart](docs/guides/quickstart.md)
- [No-video troubleshooting](docs/troubleshooting/no-video.md)

## Feature Docs

OpenIRL is organized around feature-level docs instead of pass logs:

- [OBS reconciliation](docs/features/obs-reconciliation.md)
- [Local ingest](docs/features/local-ingest.md)
- [Encoder profiles](docs/features/encoder-profiles.md)
- [Dashboard](docs/features/dashboard.md)
- [Security](docs/features/security.md)
- [Brownout](docs/features/brownout.md)
- [Support bundles](docs/features/support-bundles.md)
- [Backup ingest](docs/features/backup-ingest.md)
- [Self-hosted relay](docs/features/self-hosted-relay.md)
- [NAT and tunnels](docs/features/nat-tunnel.md)
- [SRTLA bonding](docs/features/bonding.md)
- [WebRTC preview](docs/features/webrtc-preview.md)
- [Vertical clips](docs/features/vertical-clips.md)
- [Plugin API](docs/features/plugin-api.md)

## Development Workflow

1. Read [AGENTS.md](AGENTS.md) before changing code or claims.
2. Keep docs, config, API behavior, and validation scripts aligned.
3. Run the smallest relevant check after a focused change.
4. Run `cargo xtask ci` before publishing a broad package change.
5. When touching live integrations, record which real external tools and devices were used.

The workspace uses Rust 2024, inherited lints, and `cargo xtask ci` as the broad validation gate.

## Contribution Boundaries

Good contributions are concrete and locally verifiable:

- improve OBS reconciliation behavior or evidence
- harden MediaMTX/SRT/SRTLA readiness checks
- add device-specific profile compatibility proof
- strengthen redaction and auth coverage
- make support bundles easier to inspect
- improve Windows alpha packaging and operator runbooks
- add integration recipes without weakening local-first defaults

Avoid changing release or readiness language unless the code, docs, and commands support the new claim.

## License

OpenIRL is distributed under dual MIT and Apache-2.0 licensing. See [LICENSE-MIT](LICENSE-MIT) and [LICENSE-APACHE](LICENSE-APACHE).
