# OpenIRL Documentation

Start here if you are evaluating, operating, or contributing to OpenIRL.

## First Run

- [Quickstart](guides/quickstart.md): run the deterministic local demo, inspect the scoped readiness report, then move into live OBS and MediaMTX checks when those dependencies are available.
- [Validation](VALIDATION.md): authoritative source and runtime validation commands.
- [Compatibility evidence](COMPATIBILITY.md): versioned source, local-runtime, integration, field, and release claims.
- [Maintainer checks](MAINTAINER_CHECKS.md): release-claim rules and live dependency evidence checklist.
- [Release checklist](RELEASE_CHECKLIST.md): package and live-environment gates for alpha releases.
- [Release provenance](RELEASE_PROVENANCE.md): Windows package manifest, independent verification, signed attestation, and claim boundaries.

## System Design

- [Architecture](ARCHITECTURE.md): workspace shape and process-bound integration model.
- [Protocols](PROTOCOLS.md): ingest and contribution-media protocol boundaries.
- [Migration guide](MIGRATION_GUIDE.md): guidance for moving from ad hoc scripts or managed Cloud OBS workflows.

## Operator Guides

- [Relay guide](guides/relay.md): self-hosted relay and process-supervised routing.
- [No-video troubleshooting](troubleshooting/no-video.md): first-response checks for missing contribution media.
- [OBS automation](OBS_AUTOMATION.md): OBS scene and WebSocket guidance.

## Contributor Guides

- [Contributor recipes](contributing/README.md): typed synthetic fixtures and focused gates for profiles, relays, metrics, redaction, and live smoke checks.
- [Public evidence safety](PUBLIC_EVIDENCE.md): required review classes enforced at every public submission surface.
- [Encoder profile recipe](contributing/encoder-profile.md)
- [Relay backend recipe](contributing/relay-backend.md)
- [Metrics parser recipe](contributing/metrics-parser.md)
- [Redaction canary recipe](contributing/redaction-canary.md)
- [Live smoke recipe](contributing/live-smoke.md)

## Hardware and Encoder Guides

- [Moblin](hardware/moblin.md)
- [IRL Pro](hardware/irl-pro.md)
- [BELABOX](hardware/belabox.md)

Use the [compatibility matrix](COMPATIBILITY.md) to distinguish generated-profile
source checks from real app, device, relay, and network evidence.

## Feature Areas

- [OBS reconciliation](features/obs-reconciliation.md)
- [Local ingest](features/local-ingest.md)
- [Encoder profiles](features/encoder-profiles.md)
- [Dashboard](features/dashboard.md)
- [Security guardrails](features/security.md)
- [Brownout](features/brownout.md)
- [Support bundles](features/support-bundles.md)
- [Alpha source package](features/alpha-source-package.md)
- [Backup ingest](features/backup-ingest.md)
- [Self-hosted relay](features/self-hosted-relay.md)
- [NAT and tunnels](features/nat-tunnel.md)
- [SRTLA bonding](features/bonding.md)
- [WebRTC preview](features/webrtc-preview.md)
- [Vertical clips](features/vertical-clips.md)
- [Plugin API](features/plugin-api.md)

## Runbooks

- [Mobile field alpha](runbooks/MOBILE_FIELD_ALPHA.md)
- [Windows OBS alpha](runbooks/WINDOWS_OBS_ALPHA.md)
- [Artifact export alpha](runbooks/ARTIFACT_EXPORT_ALPHA.md)
- [V1 public beta](runbooks/V1_PUBLIC_BETA.md)

## Security

- [Security policy](../SECURITY.md): vulnerability reporting and sensitive-data rules.
- [Security model](SECURITY.md): local-first defaults, roles, and support-bundle posture.
- [Public beta security](features/public-beta-security.md): release-facing guardrails.
