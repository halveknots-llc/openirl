# Security

OpenIRL controls live production software and stores sensitive configuration. The default security posture is local-first and explicit opt-in for broader access.

For vulnerability reporting, supported versions, and disclosure expectations, use the repository-level [security policy](../SECURITY.md).

## Defaults

- Bind the agent to `127.0.0.1` unless LAN access is intentionally enabled.
- Keep dashboard API access same-origin unless explicit CORS origins are configured.
- Require OBS WebSocket authentication.
- Keep OBS WebSocket off the public internet.
- Keep relay execution disabled until configured by the operator.
- Redact dashboard tokens, stream keys, SRT passphrases, and sensitive network values from support artifacts.
- Do not include unredacted support bundles, credential-bearing URLs, stream keys, SRT passphrases, dashboard tokens, OBS passwords, or relay credentials in public issues.

## Roles

- Owner: all controls.
- Producer: scene switching, recording, replay, markers, selected stream controls.
- Moderator: BRB/scene switching, replay save, markers when permitted.
- Viewer: read-only status.

## Support bundles

Support bundles should be reviewed before sharing. The export path includes redacted config, session timeline, relay plan, metrics snapshot, OBS actions, and field report content.
