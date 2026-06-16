# OpenIRL Public Launch Readiness Spec

Generated: 2026-06-16 13:10 PDT

This spec defines the work needed to make OpenIRL credible as a public open-source alpha. It separates what can be proven by local automation from what requires real live streaming dependencies.

## Launch Principles

- Preserve local-first operation and operator ownership.
- Keep OBS control, dashboard access, relay exposure, stream keys, and support bundles conservative by default.
- Make first-run contribution possible without requiring a full live IRL rig.
- Keep release claims tied to commands, files, and live dependency evidence.
- Prefer public contributor surfaces over internal handoff language.

## Readiness Areas

### 1. Legal and Package Identity

Required outcomes:

- Full MIT and Apache-2.0 license texts are present.
- A root license selector explains the dual-license choice.
- Cargo workspace metadata points to the public GitHub repository.
- Package metadata includes a clear description, README, keywords, categories, license, authors, and Rust version.

Current status:

- Implemented for launch preparation: license texts, root license selector, and Cargo metadata.

### 2. Community Health

Required outcomes:

- Public contribution guide.
- Code of Conduct.
- Support policy.
- Security vulnerability reporting policy.
- GitHub-recognized issue templates for bugs, field reports, and feature requests.
- Pull request template that requires validation and live-dependency disclosure.

Current status:

- Implemented for launch preparation: root community files and `.github` templates.

### 3. Newcomer Documentation

Required outcomes:

- README presents the project as public source with alpha runtime status.
- Quickstart has a no-live-dependency path before OBS and MediaMTX checks.
- Docs index routes readers to validation, security, hardware, features, runbooks, and troubleshooting.
- Changelog describes user-facing alpha capabilities and known runtime limitations.

Current status:

- Implemented for launch preparation: README status, docs index, quickstart, and changelog.

### 4. Validation and CI

Required outcomes:

- `cargo xtask ci` is the broad local source gate.
- CI runs the same broad gate.
- Clippy warnings are denied in the broad gate.
- Security smoke is part of the broad gate.
- Handoff audit remains part of public-readiness validation until the repository no longer depends on it.
- Code scanning runs on public pushes, pull requests, weekly schedule, and manual dispatch.
- Dependency update checks run weekly for Cargo and GitHub Actions.

Current status:

- Implemented for launch preparation: `xtask ci`, GitHub Actions CI, CodeQL scanning workflow, Dependabot update checks, validation docs, release checklist, and security smoke coverage.
- Static validation and handoff audit passed after AppleDouble sidecar cleanup.

### 5. Security and Local-First Safety

Required outcomes:

- Root security policy explains private reporting and sensitive-data rules.
- Docs security model links to the root policy.
- Config validation rejects public bind without auth.
- Redaction coverage protects dashboard tokens, stream keys, SRT passphrases, OBS passwords, relay credentials, and credential-bearing URLs.
- Support bundle and issue guidance tells users to review exports before sharing.

Current status:

- Policy and docs updates implemented for launch preparation.
- Code-level safety updates implemented for launch preparation:
  - `--bind` overrides are applied before config validation.
  - error-level config validation findings stop agent startup.
  - `/api/*` routes require non-loopback auth through shared middleware.
  - control auth uses the actual peer address when available.
  - support-bundle JSON and field-report markdown run through a final scrub pass.
  - security smoke now executes config validation, unsafe bind rejection, and support-bundle redaction canaries.
  - support-bundle API export paths are constrained to relative paths under the configured bundle root.
  - runtime readiness separates agent readiness, source validation, and live dependency readiness.

### 6. Public Package Hygiene

Required outcomes:

- Generated public package includes public-facing docs and templates.
- Internal process artifacts are not presented as the primary contributor path.
- Public package scripts preserve docs, presets, issue templates, plugin manifests, and validation evidence.
- Generated artifacts avoid local absolute paths unless they are explicitly diagnostic.

Current status:

- Implemented for launch preparation. Root-level internal process files were removed, public maintainer checks were added, validators now require public community surfaces, release artifact names use public alpha wording, and first-tier operator docs now include concrete commands, expected evidence, and validation boundaries.

### 7. Live Dependency Evidence

Required outcomes:

- OBS smoke evidence from a real OBS Studio and authenticated OBS WebSocket environment.
- MediaMTX SRT/RTMP ingest smoke evidence.
- Mobile profile import evidence from at least one supported encoder app.
- Relay, tunnel, SRTLA, BELABOX, WebRTC, and Windows packaging checks are either proven or explicitly scoped as not yet validated for the alpha.

Current status:

- Not claimed. The current environment has not proven live external dependencies.

### 8. Maintainer Operations

Required outcomes:

- Clear triage labels or issue template labels.
- Public support expectations.
- Release checklist and validation docs stay synchronized.
- Future changes keep release claims tied to commands and live evidence.

Current status:

- Support expectations and issue labels added through templates.
- Label creation itself is a GitHub repository operation and should be done after push if desired.

## Verification Evidence

- `python3 scripts/static_validate.py`: passed on 2026-06-16 after AppleDouble cleanup.
- `python3 scripts/audit/handoff_audit.py`: passed on 2026-06-16 after AppleDouble cleanup.
- `cargo fmt --all -- --check`: passed on 2026-06-16.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed on 2026-06-16.
- `cargo test --workspace`: passed on 2026-06-16.
- `python3 scripts/security/security-audit-smoke.py`: passed on 2026-06-16.
- `cargo xtask ci`: passed on 2026-06-16.
- Sanitized browser-rendered media captured: `reports/public-readiness/assets/public-readiness-clean.png`.
- GitHub repository visibility: public on 2026-06-16.
- Remote `main`: `e39b1000cb22fcb52c582b7c8ad13e4f93efc1c0` before final readiness-log update.
- GitHub Actions `ci` / `rust`: passed on the public `main` branch after the workflow update to `actions/checkout@v6`.
- CodeQL workflow added for Rust and GitHub Actions analysis with `github/codeql-action@v4`; first public analysis runs after this hardening push.
- Dependabot update configuration added for Cargo and GitHub Actions.
- GitHub security setup: private vulnerability reporting, Dependabot security updates, secret scanning, push protection, non-provider pattern scanning, and validity checks enabled.
- Branch protection: `main` requires the `rust` status check, strict branch currency, linear history, no force pushes, no deletion, and conversation resolution.

## Next Implementation Targets

1. Add live dependency evidence when OBS, MediaMTX, mobile encoders, BELABOX, SRTLA, tunnels, or Windows packaging hosts are available.
2. Keep this readiness report public-safe as a maintainer evidence trail.
3. Re-run the documented validation gate before each release claim or public alpha package cut.
