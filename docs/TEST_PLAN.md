# Test Plan

## Unit and integration tests

Run the Rust workspace checks through `cargo xtask ci`.

## Agent API smoke

Run `scripts/smoke/api_smoke.py` against a running agent.

## OBS smoke

Use the OBS WebSocket script to verify status, scene listing, scene switching, stream controls, replay save, and recording controls.

## Ingest smoke

Use MediaMTX with a test publisher to verify SRT/RTMP publication, metrics polling, and OBS media-source visibility.

## Field smoke

Use Moblin and IRL Pro devices to scan generated profiles, publish to local or relay ingest, and trigger brownout/recovery scenarios.
