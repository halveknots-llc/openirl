# Release Checklist

## Source package

- Worktree is clean before packaging; public packages are assembled from a Git archive rather than ignored or untracked worktree files.
- Package manifest records the source revision and contains no credentials, local paths, support bundles, or generated caches.
- Static validation passes.
- Source-readiness audit passes through `python3 scripts/audit/handoff_audit.py`.
- JSON and TOML files parse.
- No legacy numbered pass labels remain.
- No unfinished markers remain.
- Checksum generated.

## Public repository and supply chain

- GitHub Actions references are pinned to reviewed immutable commits.
- Dependency-review checks are enabled for pull requests.
- Container base images are pinned by digest and `.dockerignore` excludes credentials, caches, support bundles, and local metadata.
- Secret and redaction canaries pass without publishing raw scan output or generated support bundles.
- Generated artifact directories and files have owner-only permissions where supported.
- Default MediaMTX listeners bind to loopback; any LAN, VPN, or public exposure has explicit authentication and network review.

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

## Evidence boundary

Automated source and API checks do not prove live OBS, MediaMTX, mobile encoder, BELABOX, SRTLA, tunnel, WebRTC, or Windows installer behavior. Record the exact smoke script, dependency versions, host platform, and artifact or log evidence before making a live integration claim.
