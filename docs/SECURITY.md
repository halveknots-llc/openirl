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

## Browser request boundary

The dashboard is intended to be served by the OpenIRL agent itself. For browser requests, the agent compares the `Origin` authority with the request `Host` authority. A same-origin request may use the configured loopback tokenless path. A different origin is accepted only when it is listed exactly in `api.cors_allowed_origins`, and that request must still authenticate with the dashboard token. `null`, malformed, or untrusted browser origins are rejected before control handlers run.

This boundary limits browser-based cross-site control. It is not a replacement for network isolation, a dashboard token, host firewalling, or an authenticated reverse proxy when the agent is intentionally exposed beyond the local machine.

## Roles

- Owner: all controls.
- Producer: scene switching, recording, replay, markers, selected stream controls.
- Moderator: BRB/scene switching, replay save, markers when permitted.
- Viewer: read-only status.

## Support bundles

Support bundles should be reviewed before sharing. The export path includes redacted config, session timeline, relay plan, metrics snapshot, OBS actions, and field report content.

Redaction covers both structured and text representations. Reviewers should still inspect generated bundles for location-adjacent details, screenshots, vendor-specific logs, and unusual credentials not covered by the standard canaries.
