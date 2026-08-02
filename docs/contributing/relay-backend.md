# Add a Relay Backend

OpenIRL keeps media routers and relay tools process-bound. A backend contribution
defines discovery, typed arguments, readiness, redaction, and supervision around
an external executable; it does not implement SRT, SRTLA, RTMP, or router
protocol internals in Rust.

## Ownership map

| Surface | Responsibility |
| --- | --- |
| `crates/openirl-config/src/lib.rs` | Serialized process kind, defaults, validation, and disabled configuration |
| `crates/openirl-relay-control/src/lib.rs` | Runtime backend, executable candidates, protocols, launch plan, bounded logs, and child supervision |
| `apps/openirl-agent/src/main.rs` | Config-to-runtime mapping and dashboard/API ownership |
| `services/openirl-relay/src/main.rs` | Standalone relay-service mapping |
| `config/openirl.example.toml` | Localhost-first, disabled example |
| `scripts/relay/` | Source readiness and guarded live smoke |
| `docs/features/self-hosted-relay.md` | Operator configuration and exposure boundary |

## Implementation steps

1. Start with `fixtures/contributing/relay-process.sample.json`. Keep `enabled`
   and `auto_start` false, listeners on loopback, and every credential value
   synthetic.
2. Add a `RelayProcessKind` only when config needs a stable public name. Add the
   corresponding `RelayBackend`, executable candidates, and planned protocols.
3. Update both config-to-runtime mappings: the local agent and standalone relay
   service are separate entrypoints.
4. Pass the executable and each argument separately to `tokio::process::Command`.
   Do not construct a shell command string, inherit the full parent environment,
   or add native media FFI.
5. Keep process start behind explicit `enabled` configuration. Preserve bounded
   stdout/stderr reads, retained-line limits, kill-on-drop behavior, localhost
   endpoints, and shared redaction.
6. Add a disabled launch-plan test using a synthetic fixture. Assert candidate
   discovery, protocol planning, redacted arguments, and no process start.

If the backend needs credentials, accept environment variable names in config
and resolve values only at process launch. Public launch plans, API responses,
logs, and support evidence must never contain those values.

## Smallest local gate

```bash
cargo test --package openirl-config --package openirl-relay-control --package openirl-relay
cargo clippy --package openirl-config --package openirl-relay-control --package openirl-relay --all-targets -- -D warnings
python3 scripts/security/security-audit-smoke.py
```

Run `cargo xtask ci` after the focused gate because the agent and relay service
share configuration but construct supervisors independently.

## Live evidence boundary

Source tests may prove parsing, process plans, disabled defaults, log bounds, and
redaction. A live claim requires the exact external binary and version on a
named host, successful readiness and lifecycle behavior, observed media or
metrics where applicable, and reviewed redacted evidence. Public bind, VPN, or
relay exposure needs a separate authentication, firewall, and transport review.
