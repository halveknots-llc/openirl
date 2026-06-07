# Artifact Export Runbook

Use this runbook to generate fallback assets, OBS scene templates, field reports, and support bundles.

```bash
cargo run --package openirl-agent -- artifacts materialize-fallback --config config/openirl.example.toml
cargo run --package openirl-agent -- artifacts obs-template --config config/openirl.example.toml --materialize
cargo run --package openirl-agent -- artifacts support-bundle --config config/openirl.example.toml
```
