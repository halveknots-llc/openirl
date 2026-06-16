# No Video in OBS

Use this checklist when the encoder appears to publish but OBS shows no contribution media.

## Fast Checks

1. Confirm OpenIRL is running on the expected config.
2. Confirm MediaMTX or the relay process is listening on the expected port.
3. Confirm the encoder URL uses the same host, port, protocol, mode, and stream ID.
4. Confirm OBS media source points at the active local or relay path.
5. Check dashboard metrics and the latest support timeline.
6. Export a support bundle after reproducing the failure.

## Commands

```bash
cargo run --package openirl-agent -- check-config --config config/openirl.example.toml
curl http://127.0.0.1:7707/api/config/redacted
curl http://127.0.0.1:7707/api/metrics/latest
curl http://127.0.0.1:7707/api/obs/status
```

If MediaMTX is involved, inspect its path status and logs. If a relay is involved, inspect:

```bash
curl http://127.0.0.1:7707/api/relay/readiness
curl http://127.0.0.1:7707/api/relay/status
```

## Common Causes

- encoder is publishing to the wrong host or LAN address
- SRT caller/listener mode is inverted
- path or stream ID differs between encoder and MediaMTX
- firewall allows TCP but blocks UDP
- OBS media source cached an old URL
- relay process is planned but disabled
- credentials are present in the encoder but missing from the relay environment

## What to Attach to an Issue

Attach reviewed, redacted excerpts only:

- OpenIRL command line and config path
- redacted `/api/config/validation`
- redacted metrics status
- OBS media source URL with credentials removed
- support-bundle timeline after reproducing the issue

Do not attach stream keys, passphrases, dashboard tokens, OBS passwords, private relay credentials, or unreviewed screenshots.
