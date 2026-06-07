#!/usr/bin/env bash
set -euo pipefail
python3 scripts/static_validate.py
cargo xtask ci
cargo run --package openirl-agent -- check-config --config config/openirl.example.toml
cargo run --package openirl-agent -- onboarding --encoder moblin --mode local-direct --config config/openirl.example.toml --no-qr
cargo run --package openirl-agent -- release-manifest --config config/openirl.example.toml
printf '\nStart the agent in another terminal, then run:\npython3 scripts/smoke/api_smoke.py\n'
