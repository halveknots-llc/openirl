# Relay Guide

Use relay mode only after the local-direct path works. The safest operator sequence is local ingest first, then a private relay or tunnel, then optional SRTLA/bonding checks.

## Recommended Order

1. Validate the source package with `cargo xtask ci`.
2. Start OpenIRL with `config/openirl.example.toml`.
3. Publish a local SRT or RTMP path into MediaMTX.
4. Confirm OBS receives the local contribution media.
5. Add a relay process plan with credentials in environment variables.
6. Test one remote publisher path.
7. Export a support bundle and field report after the test.

## Relay Options

| Path | Use when | Notes |
| --- | --- | --- |
| Local MediaMTX | OBS and encoder are on the same trusted network | Best first proof path. |
| VPS relay | the OBS machine is behind CGNAT or unstable home routing | Keep relay credentials out of config files. |
| WireGuard or private VPN | dashboard or control access must cross networks | Keep OBS WebSocket on localhost or a protected private interface. |
| frp/rathole-style reverse tunnel | no-public-IP networks need a contribution path | Restrict exposed ports and document firewall rules. |
| SRTLA helper | bonded contribution links are required | Validate receiver compatibility before field use. |

## Commands

```bash
cargo run --package openirl-agent -- check-config --config config/openirl.example.toml
cargo run --package openirl-agent -- serve --config config/openirl.example.toml
curl http://127.0.0.1:7707/api/relay/readiness
curl http://127.0.0.1:7707/api/relay/plan
```

Run live relay scripts only when the external process and host are available:

```bash
scripts/relay/self-hosted-relay-smoke.sh
scripts/tunnels/tunnel-readiness-smoke.sh
scripts/relay/srtla2-compat-smoke.sh
```

## Security Rules

- Do not expose OBS WebSocket publicly.
- Do not commit relay credentials, dashboard tokens, or stream keys.
- Use environment variables for passphrases and auth tokens.
- Reject public dashboard bind without auth.
- Review support bundles before sharing.

## Evidence to Capture

Record relay host type, tool versions, exposed ports, firewall rules, publisher command or encoder app, and dashboard readiness output. Remove credentials and private endpoint details before posting public issues.
