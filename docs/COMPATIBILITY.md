# Compatibility Evidence

OpenIRL publishes a versioned compatibility matrix at
[`compatibility/matrix-v1.json`](../compatibility/matrix-v1.json). The matrix is
an evidence ledger, not a blanket support claim. Each row identifies the exact
OpenIRL revision, dependency family, proof level, result, rerunnable check, and
public-safe artifact reference.

## Reading a row

`result` describes what happened at the row's declared `maturity`. A passed
source check does not establish that a real OBS, MediaMTX, encoder, relay,
network, or Windows environment worked.

| Maturity | Evidence required |
| --- | --- |
| `modeled` | The workflow and interfaces are represented, but no source behavior is claimed. |
| `source-validated` | Typed contracts and automated source tests passed. |
| `local-runtime-validated` | A deterministic local process path passed without the named external dependency. |
| `integration-validated` | Exact versions passed against the real dependency in a controlled environment. |
| `field-validated` | A real operator, device, and network session passed with reviewed evidence. |
| `released` | A downloadable artifact carries matching package and provenance evidence. |

`not-recorded` is intentional wherever no real integration version or host has
been established. `not-run` is evidence, not a defect: it prevents a modeled
path from being mistaken for tested compatibility.

## Current baseline

The checked-in baseline records source validation for the OBS WebSocket v5,
MediaMTX SRT configuration, Moblin, IRL Pro, Larix, BELABOX, SRTLA process, and
backup-ingest contracts. Brownout and recovery behavior also carry deterministic
local-runtime evidence. Real OBS, MediaMTX, mobile-device, SRTLA, network, and
Windows release evidence remains unclaimed until those environments run.

## Reproduce and validate

Regenerate the baseline from the typed Rust model using the exact source commit
and review date recorded in the file:

```bash
cargo run --package openirl-agent -- compatibility-matrix \
  --revision 73d54bea13b5d02bb5d5b91c54cc74e49cc2a66d \
  --reviewed-on 2026-08-01
```

Validate a proposed matrix before review:

```bash
cargo run --package openirl-agent -- compatibility-validate \
  --file compatibility/matrix-v1.json
```

Validation rejects malformed revisions, duplicate row IDs, mismatched source
revisions, absolute or parent-relative artifact paths, sensitive text, and
integration-or-higher claims without concrete versions, hosts, and evidence.

## Contribute field evidence

Follow the [public evidence policy](PUBLIC_EVIDENCE.md): before publishing,
remove stream credentials, authentication credentials, credential-bearing URLs,
private network details, local paths, device identifiers, location-sensitive
media, private-production stream IDs, and raw support bundles.

1. Start from [`fixtures/field/compatibility-evidence.template.json`](../fixtures/field/compatibility-evidence.template.json).
2. Record exact public versions, host platform, OpenIRL commit, configuration class, and repository-relative smoke command.
3. Run the named real dependency and report the narrow result that actually occurred.
4. Review every excerpt and attachment before opening a field report.
5. Submit the proposed row and reviewed evidence through the [field report template](../.github/ISSUE_TEMPLATE/field_report.md).

Never publish stream keys, passphrases, dashboard tokens, OBS passwords, relay
credentials, private IPs or hostnames, unreviewed support bundles, or media that
reveals sensitive locations. Maintainers may preserve a minimal redacted excerpt
or public test artifact; raw production evidence stays off the public repository.
