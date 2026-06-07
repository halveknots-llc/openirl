# Security

OpenIRL controls live production software and stores sensitive configuration. The default security posture is local-first and explicit opt-in for broader access.

## Defaults

- Bind the agent to `127.0.0.1` unless LAN access is intentionally enabled.
- Require OBS WebSocket authentication.
- Keep OBS WebSocket off the public internet.
- Keep relay execution disabled until configured by the operator.
- Redact dashboard tokens, stream keys, SRT passphrases, and sensitive network values from support artifacts.

## Roles

- Owner: all controls.
- Producer: scene switching, recording, replay, markers, selected stream controls.
- Moderator: BRB/scene switching, replay save, markers when permitted.
- Viewer: read-only status.

## Support bundles

Support bundles should be reviewed before sharing. The export path includes redacted config, session timeline, relay plan, metrics snapshot, OBS actions, and field report content.
