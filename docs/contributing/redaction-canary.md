# Add a Redaction Canary

Add a canary whenever a new secret name, credential transport, endpoint shape,
process argument, support field, artifact path, or log format enters OpenIRL.
Canaries are visibly synthetic values used to prove a public surface does not
retain sensitive input.

## Ownership map

| Surface | Responsibility |
| --- | --- |
| `crates/openirl-vault/src/lib.rs` | Shared URL, command, text, JSON, IP, and path redaction |
| `scripts/security/security-audit-smoke.py` | Executable config, bind, auth, and support-bundle canaries |
| `scripts/security/release-artifact-scan.py` | Bounded public archive and high-confidence credential scan |
| `fixtures/contributing/redaction-canary.sample.json` | Typed synthetic support-evidence input |
| `docs/SECURITY.md` | Public data handling and threat boundary |

## Implementation steps

1. Name the synthetic value `synthetic-<surface>-canary`. Never derive a test
   value from an operator credential, password manager, environment variable, or
   production support bundle.
2. Route the new surface through the narrow shared function: `redact_url`,
   `redact_command_args`, `redact_support_text`, or
   `scrub_support_bundle_value`. Extend the central detector only when an
   existing function cannot recognize the new shape.
3. Preserve non-sensitive structure so an operator can still diagnose the
   issue. Replace values, userinfo, path credentials, private addresses, or
   absolute local paths without deleting unrelated fields.
4. Add a negative assertion for every canary and a positive assertion for the
   expected redaction marker. Also verify allowed environment variable names and
   loopback addresses remain useful when that is the intended policy.
5. Exercise every public serialization path affected: logs, API payloads,
   generated profiles, support bundles, field reports, manifests, and shareable
   exports are separate sinks.

Avoid writing a second feature-local redactor. Duplicated secret-name lists
diverge and leave new encodings or argument forms exposed.

## Smallest local gate

```bash
cargo test --package openirl-vault
python3 scripts/security/security-audit-smoke.py
python3 scripts/security/release-artifact-scan.py --self-test
```

Run the affected crate tests and `cargo xtask ci` before review.

## Public evidence boundary

Passing a canary proves only the tested shape and output path. It does not prove
that arbitrary secrets can be made safe after collection. Prefer not collecting
or serializing a sensitive field at all. Report an actual leak privately through
`SECURITY.md`; do not add the leaked value as a regression fixture.
