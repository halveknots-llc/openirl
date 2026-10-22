# Security

OpenIRL controls live production software and stores sensitive configuration. The default security posture is local-first and explicit opt-in for broader access.

For vulnerability reporting, supported versions, and disclosure expectations, use the repository-level [security policy](../SECURITY.md).

## Defaults

- Bind the agent to `127.0.0.1` unless LAN access is intentionally enabled.
- Keep dashboard API access same-origin unless explicit CORS origins are configured.
- Treat browser `Origin` and `Host` validation as part of the control boundary; an absent `Origin` is treated as a non-browser client, while a malformed or untrusted browser origin is rejected.
- Require a dashboard token for an explicitly configured cross-origin client; an allowlist entry does not create a tokenless control bypass.
- Require OBS WebSocket authentication.
- Keep OBS WebSocket off the public internet.
- Keep relay execution disabled until configured by the operator.
- Redact dashboard tokens, stream keys, SRT passphrases, relay environment values and arguments, credential-bearing URL paths, sensitive network values, and local paths from support artifacts.
- Write generated support and diagnostic files with owner-only permissions where supported by the host platform.
- Do not include unredacted support bundles, credential-bearing URLs, stream keys, SRT passphrases, dashboard tokens, OBS passwords, or relay credentials in public issues.

## OBS WebSocket authentication

The [OBS WebSocket v5 protocol](https://github.com/obsproject/obs-websocket/blob/master/docs/generated/protocol.md#creating-an-authentication-string) requires a challenge response computed as `Base64(SHA256(Base64(SHA256(password + salt)) + challenge))`. OpenIRL sends that response in the Identify message; it does not persist the intermediate secret or use this function to store password verifiers. Replacing either hash with Argon2, scrypt, bcrypt, or PBKDF2 would prevent authentication with OBS.

This is a protocol compatibility exception for the `rust/weak-sensitive-data-hashing` code-scanning rule, not a general password-hashing recommendation. Keep code scanning enabled and review any other use independently. Fixed wire vectors, changed-challenge checks, and malformed authentication-input tests cover the adapter contract.

The response does not encrypt the connection or authenticate the server. An observer who captures a plaintext WebSocket handshake can attempt offline password guesses. Keep OBS on loopback or use a trusted encrypted tunnel for remote access; do not expose its WebSocket port publicly. Automated protocol tests do not establish live OBS compatibility.

## Browser request boundary

The dashboard is intended to be served by the OpenIRL agent itself. For browser requests, the agent compares the `Origin` authority with the request `Host` authority. A same-origin request may use the configured loopback tokenless path. A different origin is accepted only when it is listed exactly in `api.cors_allowed_origins`, and that request must still authenticate with the dashboard token. `null`, malformed, or untrusted browser origins are rejected before control handlers run.

This boundary limits browser-based cross-site control. It is not a replacement for network isolation, a dashboard token, host firewalling, or an authenticated reverse proxy when the agent is intentionally exposed beyond the local machine.

The agent rejects framing through both `Content-Security-Policy: frame-ancestors 'none'`
and `X-Frame-Options: DENY`, including static pages and error responses. It also
disables MIME sniffing and referrer transmission, and marks API responses
`Cache-Control: no-store` so status, generated profiles, and authentication failures
are not retained by compliant HTTP caches. The CSP additionally blocks base URL
overrides and embedded objects; it is not a full script-source policy.

## Dependency and build integrity

CI, security smokes, and package builds use the committed Cargo lockfile without
resolving new versions. `cargo deny --locked check` refreshes RustSec advisories and
rejects known vulnerabilities and yanked dependencies without advisory exceptions.
Dependabot groups Cargo minor/patch version updates and also checks regex; major
updates remain separate. Pre-1.0 minor updates can still break compatibility and
must pass review and CI. A successful local Cargo update does not prove that the
hosted Dependabot updater has completed its next scheduled run.

Workflow actions and relay container bases are pinned by digest. CI and CodeQL
checkouts do not leave Git credentials available to later build steps. These
controls do not patch the GitHub runner's embedded Node.js runtime, the host's
certificate store, or separately installed OBS, MediaMTX and relay executables.
Keep those components updated through their upstream security channels, and verify
release provenance before executing downloaded binaries.

## Roles

- Owner: all controls.
- Producer: scene switching, recording, replay, markers, selected stream controls.
- Moderator: BRB/scene switching, replay save, markers when permitted.
- Viewer: read-only status.

## Support bundles

Support bundles should be reviewed before sharing. The export path includes redacted config, session timeline, relay plan, metrics snapshot, OBS actions, and field report content.

Redaction covers both structured and text representations. Reviewers should still inspect generated bundles for location-adjacent details, screenshots, vendor-specific logs, and unusual credentials not covered by the standard canaries.
