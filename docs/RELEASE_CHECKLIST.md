# Release Checklist

## Source package

- Static validation passes.
- Source-readiness audit passes through `python3 scripts/audit/handoff_audit.py`.
- JSON and TOML files parse.
- No legacy numbered pass labels remain.
- No unfinished markers remain.
- Checksum generated.

## Rust package

- `cargo deny check` passes.
- `cargo fmt --all -- --check` passes.
- `cargo clippy --workspace --all-targets -- -D warnings` passes.
- `cargo test --workspace` passes.
- `cargo xtask ci` passes.

## Runtime package

- OBS WebSocket smoke script passes.
- MediaMTX ingest path works for SRT and RTMP.
- Dashboard loads on a phone.
- Moblin and IRL Pro profile QR flow works.
- Support-bundle export is redacted.
- Relay and tunnel docs are verified by an operator.
