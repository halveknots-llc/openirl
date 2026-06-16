# OBS Reconciliation

OpenIRL treats OBS as local production software, not as a cloud control surface. The agent can build an OBS scene/source template, review the intended actions, and apply safe scene switches through the configured OBS controller.

## Source Validation

Run the source checks before testing against a live OBS profile:

```bash
cargo run --package openirl-agent -- check-config --config config/openirl.example.toml
cargo run --package openirl-agent -- artifacts obs-template --config config/openirl.example.toml
cargo test --package openirl-obs
```

Expected evidence:

- the example config validates with a localhost dashboard bind
- the template plan lists the scenes and ingest sources it would create
- unit tests pass without requiring a running OBS instance

## Live OBS Validation

Live validation requires OBS Studio with the built-in WebSocket server enabled and password-protected. Keep OBS WebSocket bound to localhost or a private network.

```bash
OPENIRL_OBS_PASSWORD='<redacted>' \
  cargo run --package openirl-agent -- serve --config config/openirl.example.toml

scripts/obs/reconcile-smoke.sh
```

Record the OBS version, WebSocket bind address, scene collection name, and whether OpenIRL created, updated, or only reviewed scene items. Do not publish OBS passwords, stream keys, or profile exports.

## Current Boundary

Automated Rust validation proves the controller contract, review controller, and template materialization path. It does not prove a specific OBS profile until the live smoke script runs against that profile.
