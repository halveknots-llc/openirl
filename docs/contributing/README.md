# Contributor Recipes

These recipes map OpenIRL's supported extension points to the smallest safe
change, typed synthetic fixture, focused test, public documentation, and live
evidence boundary. Start here instead of inferring an extension pattern from a
single crate.

| Change | Recipe | Owning surface |
| --- | --- | --- |
| Mobile encoder or hardware profile | [Encoder profile](encoder-profile.md) | `openirl-core`, `openirl-profiles`, agent CLI/dashboard, presets |
| Process-supervised media or relay tool | [Relay backend](relay-backend.md) | `openirl-config`, `openirl-relay-control`, agent and relay entrypoints |
| Prometheus exporter or SRTLA status format | [Metrics parser](metrics-parser.md) | `openirl-metrics` |
| New secret, endpoint, path, or log shape | [Redaction canary](redaction-canary.md) | `openirl-vault`, security smoke |
| Real OBS, router, device, relay, tunnel, or host check | [Live smoke](live-smoke.md) | matching `scripts/` integration area and compatibility evidence |

## Shared contract

Every extension contribution should include:

1. A synthetic fixture under `fixtures/contributing/` or the owning feature's
   fixture directory. Use `example.test`, loopback addresses, documentation IP
   ranges, and values explicitly named `synthetic-...-canary`.
2. A focused test in the owning crate or script. The fixture should be consumed
   by the same typed parser or redactor used in production.
3. The smallest relevant local gate plus `cargo xtask ci` before review.
4. Updated operator documentation and, when compatibility changes, a proposed
   evidence row with an exact OpenIRL revision.
5. A precise statement of which real dependencies ran and which remained
   `not-run`.

Do not put production credentials, private endpoints, device identifiers,
location-sensitive media, ignored runtime artifacts, or raw support bundles in
a fixture, commit, issue, or pull request. A vulnerability or suspected real
credential exposure belongs in the private process described by
[`SECURITY.md`](../../SECURITY.md).

## Clean-checkout gate

Run the focused command named by the recipe, then the repository gate:

```bash
python3 scripts/static_validate.py
python3 scripts/audit/handoff_audit.py
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo xtask ci
```

Live tools and devices are never prerequisites for these local source gates.
They are separate evidence steps and must fail closed when their named
dependency is unavailable.
