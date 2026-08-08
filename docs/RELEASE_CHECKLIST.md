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
- Windows portable archive and external manifest are independently verified on a second clean Windows runner.
- Every packaged file matches the manifest's path, size, and SHA-256 digest.

## Public repository and supply chain

- GitHub Actions, reusable workflow, and Docker image references are parsed as
  YAML and pinned to reviewed immutable commits or image digests, including
  recursively referenced local actions.
- Only trusted pushes to `main` and protected version tags can receive keyless
  signed provenance; pull requests and manual dispatches cannot request OIDC
  signing credentials or publish releases.
- Dependency-review checks are enabled for pull requests.
- Container base images are pinned by digest and `.dockerignore` excludes credentials, caches, support bundles, and local metadata.
- Secret and redaction canaries pass without publishing raw scan output or generated support bundles.
- The bounded release-artifact scan rejects local build paths, unsafe archive
  entries, credential containers, and high-confidence credential patterns in
  the archive, manifests, checksum, verification report, and release notes.
- A release tag is fetched and re-resolved immediately before publication; its
  commit must still equal the revision whose package and provenance passed.
- An active `v*` tag ruleset blocks tag updates and deletion, immutable releases
  are enabled, and the complete release is assembled as a draft before it is
  published.
- `gh release verify` authenticates the selected tagged release, and
  `gh attestation verify` authenticates all four evidence files against the
  expected signer workflow and exact source tag before local execution.
- Generated artifact directories and files have owner-only permissions where supported.
- Default MediaMTX listeners bind to loopback; any LAN, VPN, or public exposure has explicit authentication and network review.

## Rust package

- `cargo deny check` passes.
- `cargo fmt --all -- --check` passes.
- `cargo clippy --workspace --all-targets -- -D warnings` passes.
- `cargo test --workspace` passes.
- `cargo xtask ci` passes.

## Runtime package

- Packaged `openirl-agent.exe readiness` and `openirl-desktop.exe plan` commands pass on a clean Windows runner.
- OBS WebSocket smoke script passes.
- MediaMTX ingest path works for SRT and RTMP.
- Dashboard loads on a phone.
- Moblin and IRL Pro profile QR flow works.
- Support-bundle export is redacted.
- Relay and tunnel docs are verified by an operator.

## Evidence boundary

Automated source and API checks do not prove live OBS, MediaMTX, mobile encoder, BELABOX, SRTLA, tunnel, WebRTC, or Windows installer behavior. Record the exact smoke script, dependency versions, host platform, and artifact or log evidence before making a live integration claim.

GitHub's signed artifact provenance identifies the workflow and exact files. It
does not replace Authenticode code signing, publisher identity, SmartScreen
reputation, or real integration evidence. See [Release Provenance](RELEASE_PROVENANCE.md).
