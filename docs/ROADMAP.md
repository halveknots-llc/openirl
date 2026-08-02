# Product Roadmap

OpenIRL is a public-source alpha local-first control plane for IRL livestreaming. This roadmap separates what is present in the source package from the evidence and product work still needed to make it dependable for more operators and contributors.

## Current source status

The repository currently includes:

- a Rust workspace with a local agent and mobile-friendly dashboard
- typed OBS control boundaries and process-bound MediaMTX, SRT, SRTLA, relay, tunnel, and WebRTC planning surfaces
- encoder profile generation for Moblin, IRL Pro, Larix, and BELABOX-oriented workflows
- brownout-aware health classification, fallback scene decisions, metrics parsing, session evidence, and support-bundle export
- localhost-first binding, explicit LAN/auth validation, browser-origin checks, structured redaction, bounded process and HTTP reads, and owner-restricted generated artifacts
- source-level validation, security checks, dependency policy, release scripts, live-smoke entry points, and public contributor documentation
- a two-runner Windows portable workflow with per-file manifests, bounded artifact scanning, and keyless signed provenance for trusted builds
- contributor recipes backed by typed synthetic fixtures for profiles, relay processes, metrics, redaction, and live-smoke evidence
- a deterministic local demo and scoped readiness report that keep source, local-runtime, and live-environment evidence separate
- a machine-validated compatibility matrix that pins every row to an exact source revision and evidence maturity

Automated source validation is not live field proof. OBS, MediaMTX, mobile encoder, BELABOX, SRTLA, tunnel, WebRTC, and Windows packaging claims remain environment-specific until the matching smoke checks run with the real dependency.

## Premier public-repo milestones

### 1. Trusted first run

- Keep the quickstart under ten minutes on a clean development machine.
- Keep the deterministic demo fixtures stable and expand them only with public-safe synthetic data.
- Keep the scoped readiness report aligned with new source, local-runtime, and live dependency gates.
- Expand dashboard API and responsive-browser coverage for setup, auth, profile export, diagnostics, and support bundles.
- Publish safe screenshots or short recordings only after checking them for tokens, network details, locations, and private operator data.

### 2. Field reliability

- Maintain the repeatable compatibility matrix for OBS, MediaMTX, Moblin, IRL Pro, Larix, BELABOX, and common SRT/SRTLA paths as reviewed field results arrive.
- Exercise brownout detection, backup ingest, scene fallback, recovery, and operator-visible explanations under degraded network conditions.
- Add bounded telemetry and diagnostic counters that help operators answer "what failed, when, and what did OpenIRL do?" without collecting stream content by default.
- Record host, dependency, configuration class, and artifact evidence for every live result.

### 3. Interoperable self-hosted relays

- Document supported MediaMTX and SRTLA versions with tested configuration examples.
- Add a private-network/VPN deployment path with explicit authentication, firewall, and certificate guidance.
- Add process-supervision integration tests using deterministic fake media tools before expanding live adapters.
- Make relay readiness, restart behavior, log retention, and credential redaction observable and testable.

### 4. Reproducible releases

- Maintain source-pinned, repeatable alpha artifacts for supported platforms with revisions, checksums, and explicit build inputs.
- Extend keyless provenance and independent verification as additional host platforms become supported.
- Keep package CLI smoke tests on actual target hosts, especially the Windows-first alpha path.
- Keep CI inputs pinned, dependency review active, and release claims linked to evidence.

### 5. Contributor and community growth

- Maintain a small set of starter issues with reproducible fixtures and clear acceptance criteria.
- Keep contributor recipes and their typed fixtures aligned as extension surfaces evolve.
- Publish release notes that explain operator impact, compatibility, security changes, and known limitations.
- Add issue labels and triage rules that route bug reports, field evidence, security reports, documentation, and good-first contributions quickly.
- Prefer demos that show a complete local workflow over feature-count marketing.

## Public issue sequencing

### P0: trust and safety

- close any secret, auth-bypass, unsafe-bind, redaction, or package-integrity regression before adding new protocol surface
- keep security and release gates required for pull requests
- ensure every public artifact and example is sanitized and reproducible

### P1: adoption and field proof

- quickstart/demo mode and API smoke matrix
- real OBS and MediaMTX compatibility evidence
- mobile encoder import evidence across supported apps
- brownout/failover field scenarios and operator-facing diagnostics
- Windows package proof and signed release workflow

### P2: breadth and ecosystem

- SRTLA bonding and relay interoperability matrix
- WHEP/WebRTC producer preview proof
- vertical scene and clip workflows
- plugin API stabilization and contributor SDK examples

## Explicit non-goals

- managed cloud service requirements or mandatory telemetry
- default public relay exposure
- collecting or uploading stream content as a prerequisite for local operation
- claiming live device, protocol, or installer support without matching evidence
