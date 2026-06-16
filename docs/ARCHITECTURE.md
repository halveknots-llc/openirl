# Architecture

OpenIRL is a local-first control plane for IRL streaming. The core process is a Rust agent that serves a local API, static dashboard, OBS automation boundary, metric ingestion, profile generation, relay supervision, and support-bundle export.

```text
Moblin / IRL Pro / BELABOX / Larix
        |
        | SRT / SRTLA / RTMP / RIST / WHIP
        v
MediaMTX / SRTLA helper / relay process
        |
        | metrics + contribution media
        v
OpenIRL Agent ---- obs-websocket ---- OBS Studio
        |
        +-- dashboard / profiles / diagnostics / support bundles
```

## Rust crates

- `openirl-core` defines protocols, encoders, deployment modes, health states, scenes, stream metrics, and the feature plan.
- `openirl-health` evaluates stream metrics and recommends scenes.
- `openirl-obs` provides the OBS controller trait, review controller, and WebSocket adapter.
- `openirl-metrics` parses Prometheus/OpenMetrics text and converts router data into stream metrics.
- `openirl-relay-control` supervises process-bound media tools rather than embedding media FFI.
- `openirl-v1` keeps the public-beta feature catalog and package materializer.

## Media boundary

OpenIRL intentionally supervises MediaMTX, SRTLA helpers, and other media tools as external processes. This keeps the Rust agent focused on orchestration, safety, diagnostics, and control while allowing operators to use proven media routers.
