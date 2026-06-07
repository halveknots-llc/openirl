#!/usr/bin/env bash
set -euo pipefail
test -f presets/tunnels/wireguard.example.conf
test -f presets/tunnels/frp.example.toml
echo tunnel readiness files present
