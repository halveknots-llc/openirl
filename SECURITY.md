# Security Policy

OpenIRL controls local production software, OBS automation, ingest routing, stream health decisions, and support evidence. Treat security reports as production-sensitive even when the runtime is still alpha.

## Supported Versions

| Version | Security support |
| --- | --- |
| `main` | Supported for source-level security fixes |
| `0.1.0-alpha.0` | Supported while it matches current alpha package guidance |

## Reporting a Vulnerability

Please report vulnerabilities through [GitHub private vulnerability reporting](https://github.com/halveknots-llc/openirl/security/advisories/new). If that channel is unavailable, open a minimal public issue asking maintainers to restore the private report path without including sensitive details.

Do not paste any of the following into public issues, discussions, pull requests, screenshots, or unreviewed support bundles:

- stream keys
- SRT passphrases
- dashboard tokens
- OBS WebSocket passwords
- private relay credentials
- credential-bearing URLs
- unredacted support bundles

## Scope

Security-sensitive areas include:

- dashboard authentication and remote access
- public bind and LAN exposure controls
- OBS WebSocket credential handling
- support-bundle and field-report redaction
- stream-key, relay, tunnel, and encoder profile handling
- generated artifacts that could expose private production details

## Disclosure Expectations

Maintainers will acknowledge security reports as quickly as practical, investigate with the reporter, and coordinate fixes before broad public disclosure when the issue could expose operators or live production systems. Reports that require live OBS, MediaMTX, encoder, relay, or Windows packaging evidence should state which environment was used without revealing secrets.
