<p align="center">
  <img src="assets/openirl-icon.png" alt="OpenIRL app icon" width="144" />
</p>

<h1 align="center">OpenIRL</h1>

<p align="center">
  <strong>Local-first control plane for IRL livestreaming.</strong>
</p>

<p align="center">
  OBS automation, SRT/SRTLA-friendly ingest, mobile encoder profiles, brownout-aware fallback behavior, diagnostics, support bundles, and optional self-hosted relay workflows in one Rust workspace.
</p>

<p align="center">
  <a href="https://github.com/halveknots-llc/openirl/actions/workflows/ci.yml"><img alt="CI" src="https://github.com/halveknots-llc/openirl/actions/workflows/ci.yml/badge.svg" /></a>
  <a href="LICENSE"><img alt="License: MIT OR Apache-2.0" src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-2f855a" /></a>
  <a href="docs/SECURITY.md"><img alt="Local-first security posture" src="https://img.shields.io/badge/security-localhost--first-0f766e" /></a>
  <a href="docs/README.md"><img alt="Documentation" src="https://img.shields.io/badge/docs-operator%20runbooks-2563eb" /></a>
</p>

<p align="center">
  <a href="#quickstart">Quickstart</a> |
  <a href="#what-openirl-does">Capabilities</a> |
  <a href="#validation-model">Validation</a> |
  <a href="#repository-map">Repository map</a> |
  <a href="docs/README.md">Docs</a> |
  <a href="CONTRIBUTING.md">Contributing</a> |
  <a href="SECURITY.md">Security</a>
</p>

## Contents

- [Why OpenIRL](#why-openirl)
- [What OpenIRL Does](#what-openirl-does)
- [Status](#status)
- [Quickstart](#quickstart)
- [Common Commands](#common-commands)
- [Validation Model](#validation-model)
- [Architecture](#architecture)
- [Repository Map](#repository-map)
- [Documentation Map](#documentation-map)
- [Security Model](#security-model)
- [Contributing](#contributing)
- [License](#license)

## Why OpenIRL

IRL streams fail in messy ways: bonded links degrade, mobile encoders reconnect, relay hosts change, OBS keeps running while the contribution feed dies, and moderators need safe controls from a phone. Managed Cloud OBS platforms solve some of this by moving production into someone else's infrastructure. OpenIRL takes the opposite path.

OpenIRL runs on hardware you control. It supervises the local production stack, generates compatible mobile profiles, reacts to stream health, writes support evidence, and keeps live dependency claims tied to checks operators can rerun.

The project goal is straightforward: keep the live show under the operator's control, make field failures explainable, and avoid turning OBS control, stream keys, or relay credentials into another hosted-service dependency.

## What OpenIRL Does

| Area | Capability | Where to go |
| --- | --- | --- |
| OBS control | Creates and switches local OBS scenes through a typed controller boundary with a WebSocket adapter. | [OBS reconciliation](docs/features/obs-reconciliation.md) |
| Local ingest | Plans MediaMTX, SRT, RTMP, SRTLA, RIST, and WHIP contribution paths while keeping router configuration explicit. | [Local ingest](docs/features/local-ingest.md) |
| Mobile encoders | Generates profiles for Moblin, IRL Pro, Larix, and BELABOX-oriented workflows. | [Encoder profiles](docs/features/encoder-profiles.md) |
| Brownout handling | Classifies stream health, explains the decision, and recommends live, low-signal, BRB, backup, privacy, or offline scenes. | [Brownout](docs/features/brownout.md) |
| Relay workflows | Models local-direct, self-hosted relay, tunnel, SRTLA, and process-supervised media-router paths. | [Self-hosted relay](docs/features/self-hosted-relay.md) |
| Dashboard | Serves a localhost mobile-friendly control room for setup, metrics, OBS actions, profile generation, and artifact export. | [Dashboard](docs/features/dashboard.md) |
| Support evidence | Builds session reports, timelines, field evidence, issue payloads, and support-bundle data maintainers can inspect. | [Support bundles](docs/features/support-bundles.md) |
| Security | Defaults to localhost, redacts secrets, blocks unsafe public bind, and separates automated checks from live dependency validation. | [Security model](docs/SECURITY.md) |

## Status

OpenIRL is public source with an alpha runtime package. The repository includes automated validation for the local agent, API contracts, config model, feature catalog, profile generation, support artifacts, and safety rules.

The source package also includes live smoke scripts for OBS, MediaMTX, relay, mobile encoder, tunnel, WebRTC preview, and Windows packaging environments. Those checks require the real external tools and devices. Treat automated Rust/API validation as source-level proof; treat live OBS, MediaMTX, mobile encoder, BELABOX, SRTLA, tunnel, and Windows-package success as proven only after the matching smoke script runs in that environment.

## Quickstart

### 1. Install Prerequisites

- Rust toolchain from [rust-toolchain.toml](rust-toolchain.toml)
- OBS Studio with OBS WebSocket enabled for live OBS checks
- MediaMTX when running local ingest checks
- Optional: Moblin, IRL Pro, Larix, or BELABOX for field profile validation

### 2. Validate the Source Package

```bash
python3 scripts/static_validate.py
python3 scripts/audit/handoff_audit.py
python3 scripts/security/security-audit-smoke.py
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo xtask ci
```

### 3. Inspect the Example Config

```bash
cargo run --package openirl-agent -- check-config --config config/openirl.example.toml
```

The example configuration binds the dashboard to `127.0.0.1:7707`, keeps relay execution disabled by default, redacts secrets in output, and uses a review-safe OBS adapter unless live OBS WebSocket access is explicitly configured.

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

| Goal | Command |
| --- | --- |
| Print the product feature catalog | `cargo run --package openirl-agent -- features` |
| Print the v1/public-beta summary | `cargo run --package openirl-agent -- v1 summary` |
| Validate a config | `cargo run --package openirl-agent -- check-config --config config/openirl.example.toml` |
| Serve the local dashboard | `cargo run --package openirl-agent -- serve --config config/openirl.example.toml` |
| Print the desktop/tray shell plan | `cargo run --package openirl-desktop -- plan` |
| Materialize fallback assets | `cargo run --package openirl-agent -- artifacts materialize-fallback` |
| Materialize an OBS template | `cargo run --package openirl-agent -- artifacts obs-template --materialize` |
| Run the broad repository gate | `cargo xtask ci` |

## Validation Model

OpenIRL separates source-level validation from live dependency validation.

| Validation level | What it proves | Representative commands |
| --- | --- | --- |
| Static repository validation | Required files exist, text markers are clean, JSON/TOML parses, and handoff docs stay aligned. | `python3 scripts/static_validate.py`, `python3 scripts/audit/handoff_audit.py` |
| Rust workspace validation | Formatting, Clippy, tests, and workspace gates pass on the local codebase. | `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo xtask ci` |
| API/dashboard validation | The local agent serves expected endpoints and dashboard routes. | `cargo run --package openirl-agent -- serve --config config/openirl.example.toml`, `python3 scripts/smoke/api_smoke.py` |
| Live dependency validation | OBS, MediaMTX, encoders, relay hosts, SRTLA tooling, tunnels, WebRTC preview, or Windows packaging work in a real environment. | Run the matching script from [Live Smoke Scripts](#live-smoke-scripts). |

### Live Smoke Scripts

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

OpenIRL intentionally keeps media engines process-bound. Rust coordinates configuration, readiness, health decisions, redaction, and operator controls; MediaMTX, SRTLA helpers, OBS, and tunnel tools continue doing protocol-specific media work.

## Repository Map

| Path | Purpose |
| --- | --- |
| [apps/openirl-agent](apps/openirl-agent/) | Local API and dashboard server. |
| [apps/openirl-desktop](apps/openirl-desktop/) | Desktop/tray shell entrypoint. |
| [services/openirl-relay](services/openirl-relay/) | Relay process planning CLI. |
| [crates/openirl-core](crates/openirl-core/) | Protocols, encoders, scenes, health states, and feature catalog. |
| [crates/openirl-health](crates/openirl-health/) | Brownout-aware health engine. |
| [crates/openirl-config](crates/openirl-config/) | Config model and validation. |
| [crates/openirl-obs](crates/openirl-obs/) | OBS controller trait and WebSocket adapter. |
| [crates/openirl-profiles](crates/openirl-profiles/) | Moblin, IRL Pro, Larix, and BELABOX profile generation. |
| [crates/openirl-metrics](crates/openirl-metrics/) | MediaMTX and SRTLA metric parsing. |
| [crates/openirl-session](crates/openirl-session/) | Stream timeline and session reports. |
| [crates/openirl-artifacts](crates/openirl-artifacts/) | Fallback assets, OBS templates, support-bundle export, and alpha package layout. |
| [crates/openirl-v1](crates/openirl-v1/) | Public-beta feature catalog and package materializer. |
| [presets](presets/) | Encoder, relay, OBS, tunnel, moderation, and production presets. |
| [scripts](scripts/) | Static validation, smoke checks, security checks, and runtime helpers. |
| [docs](docs/) | Guides, runbooks, feature docs, security, release checklist, and validation rules. |
| [deploy](deploy/) | MediaMTX, relay, and Windows deployment assets. |
| [fixtures](fixtures/) | Sample evidence and metric data. |
| [xtask](xtask/) | Repository validation task runner. |

## Documentation Map

| Start here | Use it for |
| --- | --- |
| [Documentation index](docs/README.md) | Full navigation across guides, runbooks, feature docs, and security docs. |
| [Quickstart guide](docs/guides/quickstart.md) | First run, local dashboard, config inspection, and API smoke flow. |
| [Architecture](docs/ARCHITECTURE.md) | Workspace structure and process-bound integration model. |
| [Validation](docs/VALIDATION.md) | Authoritative source and runtime validation commands. |
| [Maintainer checks](docs/MAINTAINER_CHECKS.md) | Release-claim rules and live dependency evidence checklist. |
| [Release checklist](docs/RELEASE_CHECKLIST.md) | Package and live-environment gates for alpha releases. |
| [Security model](docs/SECURITY.md) | Local-first defaults, roles, public bind posture, and support-bundle guidance. |
| [No-video troubleshooting](docs/troubleshooting/no-video.md) | First-response checks for missing contribution media. |

Feature docs:
[OBS reconciliation](docs/features/obs-reconciliation.md),
[local ingest](docs/features/local-ingest.md),
[encoder profiles](docs/features/encoder-profiles.md),
[dashboard](docs/features/dashboard.md),
[brownout](docs/features/brownout.md),
[backup ingest](docs/features/backup-ingest.md),
[self-hosted relay](docs/features/self-hosted-relay.md),
[NAT and tunnels](docs/features/nat-tunnel.md),
[SRTLA bonding](docs/features/bonding.md),
[WebRTC preview](docs/features/webrtc-preview.md),
[vertical clips](docs/features/vertical-clips.md),
[plugin API](docs/features/plugin-api.md), and
[support bundles](docs/features/support-bundles.md).

Hardware guides:
[Moblin](docs/hardware/moblin.md),
[IRL Pro](docs/hardware/irl-pro.md), and
[BELABOX](docs/hardware/belabox.md).

Runbooks:
[Windows OBS alpha](docs/runbooks/WINDOWS_OBS_ALPHA.md),
[mobile field alpha](docs/runbooks/MOBILE_FIELD_ALPHA.md),
[artifact export alpha](docs/runbooks/ARTIFACT_EXPORT_ALPHA.md), and
[v1 public beta](docs/runbooks/V1_PUBLIC_BETA.md).

## Security Model

OpenIRL controls production software, so the default posture is conservative:

- The agent binds to localhost unless LAN access is intentionally enabled.
- OBS WebSocket should always be password-protected.
- Public bind without auth is rejected by config validation.
- Dashboard tokens, stream keys, SRT passphrases, OBS passwords, and relay credentials are redacted from reports and support artifacts.
- Relay process execution is opt-in.
- Support bundles are designed for review before sharing.

Start with [config/openirl.example.toml](config/openirl.example.toml). Important defaults:

```toml
api.bind = "127.0.0.1:7707"
security.require_auth_outside_localhost = true
security.dashboard_token_env = "OPENIRL_DASHBOARD_TOKEN"
obs.password_env = "OPENIRL_OBS_PASSWORD"
relay.enabled = false
relay.supervisor_mode = "dry-run"
metrics.source = "disabled"
```

See [SECURITY.md](SECURITY.md) for vulnerability reporting and [docs/SECURITY.md](docs/SECURITY.md) for the threat model, role model, and support-bundle guidance.

## Contributing

Good contributions are concrete and locally verifiable:

- improve OBS reconciliation behavior or evidence
- harden MediaMTX, SRT, RTMP, or SRTLA readiness checks
- add device-specific profile compatibility proof
- strengthen redaction and auth coverage
- make support bundles easier to inspect
- improve Windows alpha packaging and operator runbooks
- add integration recipes while preserving local-first defaults

Read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request. Keep docs, config, API behavior, and validation scripts aligned, and record which real external tools or devices were used when touching live integrations.

## License

OpenIRL is distributed under dual MIT and Apache-2.0 licensing. See [LICENSE](LICENSE), [LICENSE-MIT](LICENSE-MIT), and [LICENSE-APACHE](LICENSE-APACHE).
