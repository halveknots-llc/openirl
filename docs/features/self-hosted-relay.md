# Self-Hosted Relay

Self-hosted relay workflows let operators bridge difficult networks without handing OBS control or stream credentials to a managed Cloud OBS service. OpenIRL plans and supervises relay processes, but external media tools still own protocol-specific behavior.

## Source Validation

```bash
cargo run --package openirl-agent -- check-config --config config/openirl.example.toml
cargo run --package openirl-agent -- serve --config config/openirl.example.toml
curl http://127.0.0.1:7707/api/relay/readiness
curl http://127.0.0.1:7707/api/relay/plan
```

Expected evidence:

- relay execution is disabled by default
- supervisor mode is explicit
- process plans name the executable, args, ports, and credential env vars
- readiness explains missing binaries, bind conflicts, or intentionally disabled relay state

## Live Relay Validation

Run relay smokes only when the target host and media tool are available:

```bash
scripts/relay/self-hosted-relay-smoke.sh
scripts/relay/srtla2-compat-smoke.sh
```

Document the host type, firewall rules, relay executable version, exposed ports, and whether credentials came from environment variables.

## Security Boundary

Do not expose OBS WebSocket publicly. Use a relay for contribution media or control-plane metadata only where the config explicitly allows it. Prefer VPN or private-network access for dashboards and control surfaces.

## Current Boundary

Source validation proves planning and readiness responses. It does not prove throughput, NAT behavior, or cloud-host firewall correctness until a real relay path has been tested.
