# Changelog

All notable OpenIRL changes are documented here. OpenIRL follows a pre-1.0 alpha versioning model while APIs, package shape, and live integration workflows mature.

## Unreleased

### Security and release hardening

- Added browser-origin validation to the API and control authentication boundary; configured cross-origin clients still require a dashboard token.
- Expanded structured and text redaction for relay environment values, paired secret arguments, credential-bearing URL paths, IPv6 addresses, and absolute local paths.
- Added bounded relay-log and HTTP-metrics reads plus owner-only permissions for generated artifact directories and files.
- Changed bundled MediaMTX examples to loopback media listeners by default and documented the review required for broader exposure.
- Changed public beta packaging to use a clean Git archive with a source-revision manifest and guarded unvalidated Windows packaging skips.
- Pinned GitHub Actions and container base-image inputs and added pull-request dependency review.

## 0.1.0-alpha.0

### Added

- Rust workspace for a local-first IRL streaming control plane.
- Local agent and dashboard entrypoint for configuration checks, status, profile generation, and artifact export.
- OBS controller boundary with a WebSocket adapter behind explicit configuration.
- MediaMTX, SRT, RTMP, SRTLA, relay, tunnel, and WebRTC-oriented planning surfaces.
- Encoder profile generation for Moblin, IRL Pro, Larix, and BELABOX-oriented workflows.
- Brownout-aware health classification, recovery decisions, and scene recommendations.
- Support-bundle, session timeline, field evidence, and diagnostics data models.
- Public source documentation for architecture, validation, security, hardware guides, feature areas, runbooks, and troubleshooting.
- GitHub issue templates, pull request guidance, support policy, security policy, and dual-license files for public collaboration.

### Security

- Localhost-first dashboard defaults.
- Public bind rejection when authentication is absent.
- Secret redaction guidance for dashboard tokens, stream keys, SRT passphrases, OBS passwords, relay credentials, support bundles, generated reports, and profile exports.
- Vulnerability reporting policy for production-sensitive issues.

### Validation

- `cargo xtask ci` runs static validation, the source-readiness audit, security smoke, formatting, Clippy with warnings denied, and the workspace test suite.
- Cargo dependency advisory, license, source, and duplicate-version policy is checked with `cargo deny check`.
- Live smoke scripts are included for OBS, MediaMTX, mobile profile compatibility, relay, tunnel, WebRTC preview, support bundles, and Windows packaging environments.

### Known Limitations

- The runtime package is alpha and should be validated against each operator's real OBS, MediaMTX, encoder, relay, tunnel, and Windows packaging environment before production use.
- Live dependency checks are intentionally separate from automated Rust/API validation.
- Managed cloud services and Discord workflows remain optional rather than default dependencies.
