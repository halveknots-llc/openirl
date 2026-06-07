# Validation

## Static checks

```bash
python3 scripts/static_validate.py
python3 scripts/audit/handoff_audit.py
```

## Rust checks

```bash
cargo xtask ci
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
cargo test --workspace
```

## Runtime checks

Runtime checks require local tools and devices that are not available in this container:

- OBS Studio 28+ with WebSocket enabled and password-protected
- MediaMTX for local routing and metrics
- Moblin or IRL Pro device for profile scans
- Optional BELABOX or SRTLA receiver for bonding validation
- Windows workstation for MSI/portable packaging checks
