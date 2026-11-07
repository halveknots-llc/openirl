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

## Local check

Run `scripts/check` from the repo root before every push. Exit 0 means the change is ready to push; CI runs the same script with `--all`, so a local pass predicts a green PR.

- It runs only the steps your change touches and skips docs-only changes, so it usually takes seconds. `scripts/check --all` runs every step; `scripts/check --list` shows them.
- A failing step prints the tail of its output. Fix the code, not the gate: change `scripts/check` only to add or repair a step, never to skip one or weaken a test to get a pass.
- If the same step still fails after three real attempts, stop and report the step, what you tried, and your best guess at the cause.
- Steps named in `KNOWN_RED` near the top of `scripts/check` already fail on the default branch, so they warn instead of block. When you fix one, take it off that list.

## Merging
After the local check passes, push and open the PR, then queue it to squash-merge itself once required checks pass. Don't wait or poll: GitHub merges and deletes the branch.
- Local gh: `gh pr merge <N> --auto --squash`
- Claude Code cloud sessions (GraphQL is blocked there): `gh api -X PUT repos/halveknots-llc/<repo>/pulls/<N>/ccr/auto_merge`
Never use `--admin` or merge by hand to skip checks. In repos with no required checks, `--auto` merges right away, so the local check is the only gate. If auto-merge turns off (merge conflict or base changed), fix the branch and queue it again.
