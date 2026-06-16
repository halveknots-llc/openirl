# IRL Pro

IRL Pro is the primary Android validation target for SRT and SRTLA-oriented mobile contribution paths.

## Setup Path

1. Validate OpenIRL source checks.
2. Start the local ingest or relay path.
3. Generate an IRL Pro profile from OpenIRL.
4. Import the profile on the Android device.
5. Publish into MediaMTX or the relay.
6. Confirm dashboard metrics and OBS scene readiness.

## Profile Command

```bash
cargo run --package openirl-agent -- profile \
  --encoder irl-pro \
  --protocol srt \
  --host 127.0.0.1 \
  --port 9000 \
  --stream-id openirl-main
```

Use the address reachable from the Android device. For field use, prefer environment-backed credentials and document the network type without publishing private endpoints.

## Evidence

Record IRL Pro version, Android version, protocol, endpoint host class, accepted profile fields, and whether OpenIRL metrics saw the contribution path. Remove secrets before attaching logs or screenshots.
