---
name: Field report
about: Report behavior from OBS, MediaMTX, mobile encoders, relays, tunnels, or brownout recovery
title: "[Field report]: "
labels: field-report
---

## Route

- Compatibility matrix row ID:
- OpenIRL revision (full commit):
- Local direct, self-hosted relay, tunnel, SRTLA, or other:
- OBS version:
- MediaMTX or relay version:
- Encoder app/device:
- Host operating system and version:
- Non-secret configuration class:
- Network type:

## What Happened

Describe the production path, the failure mode, and how OpenIRL responded.

## Brownout or Recovery State

- Health state:
- Scene selected:
- Backup ingest available:
- Operator action taken:

## Evidence

Attach reviewed, redacted excerpts only:

```text

```

## Validation Boundary

- [ ] This report came from a real live dependency environment.
- [ ] I identified which tools and devices were used.
- [ ] I recorded exact dependency, host, and OpenIRL versions.
- [ ] I identified the narrow evidence maturity and did not infer unrun steps.
- [ ] I removed stream keys, SRT passphrases, dashboard tokens, OBS passwords, private relay credentials, and credential-bearing URLs.
- [ ] I removed private network details, device identifiers, and location-sensitive media.
- [ ] I reviewed every excerpt, screenshot, and log before attaching it; I did not attach a raw support bundle.
