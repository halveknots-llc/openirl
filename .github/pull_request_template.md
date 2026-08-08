## Summary

Describe the change and the operator-facing impact.

## Extension Recipe

Check the path used by this change:

- [ ] Encoder profile
- [ ] Process-bound relay backend
- [ ] Metrics parser
- [ ] Redaction canary
- [ ] Live smoke check
- [ ] No extension recipe applies

Owning crate or script, synthetic fixture, and focused test:

```text

```

## Validation

Check each command that ran successfully:

- [ ] `python3 scripts/static_validate.py`
- [ ] `python3 scripts/audit/handoff_audit.py`
- [ ] `python3 scripts/security/security-audit-smoke.py`
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `cargo xtask ci`

Commands not run and why:

```text

```

## Live Dependency Evidence

Check any live environment that was actually used:

- [ ] OBS Studio and OBS WebSocket
- [ ] MediaMTX ingest
- [ ] Mobile encoder import or QR scan
- [ ] BELABOX or SRTLA receiver
- [ ] Tunnel or relay host
- [ ] Windows package host
- [ ] No live dependency checks were run

## Security and Redaction

- [ ] I followed the [public evidence policy](../docs/PUBLIC_EVIDENCE.md): remove stream credentials, authentication credentials, credential-bearing URLs, private network details, local paths, device identifiers, location-sensitive media, private-production stream IDs, and raw support bundles.
- [ ] This change preserves localhost-first defaults unless broader access is explicitly configured.
- [ ] This change does not log stream keys, SRT passphrases, dashboard tokens, OBS passwords, private relay credentials, or credential-bearing URLs.
- [ ] Any attached support bundle, screenshot, fixture, or report has been reviewed for secrets.
- [ ] New fixtures use only synthetic values, loopback/example endpoints, or documentation IP ranges.
- [ ] I did not attach a raw support bundle, unreviewed log, private topology, device identifier, or location-sensitive media.
- [ ] New live checks require explicit opt-in, use bounded timeouts, and fail closed when prerequisites or assertions fail.

## Docs

- [ ] Public docs, commands, API routes, and validation language were updated where needed.
- [ ] Release or readiness claims are backed by code and validation evidence.
- [ ] Compatibility evidence names exact versions, host, OpenIRL revision, maturity, and unrun dependencies where applicable.
