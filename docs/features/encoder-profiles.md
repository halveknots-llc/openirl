# Encoder Profiles

OpenIRL generates encoder profiles and QR-oriented payloads for Moblin, IRL Pro, Larix, and BELABOX-oriented workflows. Profiles should make local contribution paths repeatable without hiding stream IDs, passphrases, or relay choices from the operator.

## Source Validation

```bash
cargo test --package openirl-profiles
cargo run --package openirl-agent -- profile --encoder moblin --protocol srt --host 127.0.0.1 --port 9000 --stream-id openirl-main
cargo run --package openirl-agent -- profile --encoder irl-pro --protocol srt --host 127.0.0.1 --port 9000 --stream-id openirl-main
```

Expected evidence:

- generated profiles use the requested host, port, protocol, and stream ID
- redacted exports do not expose passphrases or dashboard tokens
- unsupported combinations fail clearly instead of generating misleading output

## Device Validation

Profile import requires the real encoder app or hardware:

- Moblin: scan or import the generated contribution profile on iOS
- IRL Pro: import the generated SRT/SRTLA profile on Android
- Larix: verify URL compatibility and latency settings
- BELABOX: review endpoint, stream ID, and passphrase handling before field use

## Field Evidence

Record the encoder app version, device OS, selected protocol, ingest endpoint, and whether the app accepted the generated profile. Remove stream keys, passphrases, private relay hosts, and location-adjacent notes before sharing.

## Current Boundary

Profile generation tests prove serialization and compatibility assumptions. They do not prove a mobile app accepted the profile until the app or device import flow has been run.
