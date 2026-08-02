# Validation

## Static checks

```bash
python3 scripts/static_validate.py
python3 scripts/audit/handoff_audit.py
python3 scripts/security/security-audit-smoke.py
python3 scripts/smoke/demo-mode-smoke.py
cargo deny check
```

## Rust checks

```bash
cargo deny check
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo xtask ci
```

`cargo xtask ci` runs static validation, handoff audit, security smoke, deterministic demo smoke, Cargo advisory/license/source policy through `cargo deny check`, formatting, Clippy with warnings denied, and the workspace test suite.

The handoff audit is the historical script name for the source-readiness audit. It verifies that public docs, feature inventory, package evidence, and readiness claims stay aligned.

## Runtime checks

The deterministic local runtime check requires no external media dependency:

```bash
cargo run --package openirl-agent -- demo
python3 scripts/smoke/demo-mode-smoke.py
```

Its `/api/readiness` output keeps source command results and all live dependencies in `not-run` state. This prevents a successful local demonstration from being interpreted as field compatibility.

Runtime checks require local tools and devices that are not available in this container:

- OBS Studio 28+ with WebSocket enabled and password-protected
- MediaMTX for local routing and metrics
- Moblin or IRL Pro device for profile scans
- Optional BELABOX or SRTLA receiver for bonding validation
- Windows workstation for MSI/portable packaging checks
