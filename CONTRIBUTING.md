# Contributing to OpenIRL

OpenIRL welcomes practical contributions that make local-first IRL production safer, clearer, and easier to operate. The best changes are concrete, locally verifiable, and honest about which live dependencies were used.

## Good First Areas

- OBS scene reconciliation and operator evidence
- MediaMTX, SRT, RTMP, and SRTLA readiness checks
- Mobile encoder profile compatibility for Moblin, IRL Pro, Larix, and BELABOX workflows
- Dashboard safety, auth, and moderator controls
- Brownout, backup ingest, and recovery behavior
- Support-bundle redaction and inspection flows
- Windows alpha packaging and operator runbooks
- Documentation that keeps local automated checks separate from live dependency checks

## Development Setup

Install the Rust toolchain named in `rust-toolchain.toml`, then run:

```bash
python3 scripts/static_validate.py
python3 scripts/audit/handoff_audit.py
python3 scripts/security/security-audit-smoke.py
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo xtask ci
```

Live smoke scripts require the named external tool or device. Do not claim OBS, MediaMTX, mobile encoder, BELABOX, SRTLA, tunnel, or Windows installer validation unless that exact check ran in that environment.

## Pull Request Expectations

- Keep changes focused and explain the operator impact.
- Keep docs, CLI commands, API behavior, and validation scripts aligned.
- Preserve localhost-first defaults and auth requirements for broader network exposure.
- Redact stream keys, SRT passphrases, dashboard tokens, OBS passwords, and relay credentials from logs, reports, fixtures, and screenshots.
- Add or update focused tests when behavior changes.
- Include the validation commands you ran and the commands you intentionally did not run.

## Contribution Licensing

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in OpenIRL is licensed as `Apache-2.0 OR MIT`, without any additional terms or conditions.

## Security-Sensitive Changes

For vulnerabilities or sensitive production issues, follow [SECURITY.md](SECURITY.md) instead of opening a public issue. Public issues should not include stream keys, dashboard tokens, OBS passwords, private relay credentials, or unreviewed support bundles.
