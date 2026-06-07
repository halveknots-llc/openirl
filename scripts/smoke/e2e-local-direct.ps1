$ErrorActionPreference = "Stop"
python scripts/static_validate.py
cargo xtask ci
cargo run --package openirl-agent -- check-config --config config/openirl.example.toml
cargo run --package openirl-agent -- onboarding --encoder moblin --mode local-direct --config config/openirl.example.toml --no-qr
cargo run --package openirl-agent -- release-manifest --config config/openirl.example.toml
Write-Host "Start the agent in another terminal, then run: python scripts/smoke/api_smoke.py"
