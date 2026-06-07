# OpenIRL Agent Instructions

## Project Overview

OpenIRL is a local-first Rust control plane for IRL streaming with OBS automation, local ingest routing, mobile encoder profiles, brownout-aware fallback behavior, dashboard controls, diagnostics, support bundles, and optional self-hosted relay workflows.

## Build And Test

Use these commands before claiming readiness:

```bash
python3 scripts/static_validate.py
python3 scripts/audit/handoff_audit.py
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo xtask ci
```

Run smoke scripts only when their external dependencies are actually available. Do not claim live OBS, MediaMTX, mobile encoder, BELABOX, or Windows installer success unless those checks ran against the real dependency.

## Coding Conventions

- Keep Rust modules typed, small, and local-first.
- Preserve the workspace lint posture: unsafe Rust is forbidden and Clippy warnings are denied in validation.
- Prefer process-bound integrations for MediaMTX, SRTLA, and relay tools before any native FFI.
- Keep OBS WebSocket integration behind explicit configuration and do not expose OBS control publicly by default.
- Do not add empty implementations, false success paths, or panic-based fallbacks in production code.

## Security Rules

- Do not log stream keys, SRT passphrases, dashboard tokens, auth secrets, OBS passwords, or private relay credentials.
- Redact secrets in logs, API snapshots, support bundles, generated reports, and shareable profile exports.
- Bind dashboard and OBS control paths to localhost by default.
- Require explicit opt-in for LAN bind, relay exposure, or public routing.
- Reject public bind without auth.

## Documentation Rules

- Describe product areas, not numbered historical passes.
- Keep documented CLI commands and API routes aligned with real code.
- Separate local automated validation from live dependency validation.
- Keep Discord and managed cloud services optional.

## Change Boundaries

- Do not weaken validators to make failures disappear.
- Do not change release or security claims unless code, docs, and tests support the claim.
- Do not replace process-bound media integrations with custom protocol implementations without review.
- Preserve Windows-first alpha packaging while keeping Linux and macOS paths possible.
