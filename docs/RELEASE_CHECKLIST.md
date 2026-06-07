# Release Checklist

## Source package

- Static validation passes.
- Handoff audit passes.
- JSON and TOML files parse.
- No legacy numbered pass labels remain.
- No unfinished markers remain.
- Checksum generated.

## Rust package

- `cargo xtask ci` passes.
- `cargo fmt --all -- --check` passes.
- `cargo clippy --workspace --all-targets` passes.
- `cargo test --workspace` passes.

## Runtime package

- OBS WebSocket smoke script passes.
- MediaMTX ingest path works for SRT and RTMP.
- Dashboard loads on a phone.
- Moblin and IRL Pro profile QR flow works.
- Support-bundle export is redacted.
- Relay and tunnel docs are verified by an operator.
