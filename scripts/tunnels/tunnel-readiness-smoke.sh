#!/usr/bin/env bash
set -euo pipefail
if [[ "${OPENIRL_LIVE_TUNNEL_SMOKE:-}" != "1" ]]; then
  echo "Set OPENIRL_LIVE_TUNNEL_SMOKE=1 and OPENIRL_TUNNEL_CHECK_HOST after a tunnel is configured." >&2
  exit 2
fi

test -f presets/tunnels/wireguard.example.conf
test -f presets/tunnels/frp.example.toml
test -n "${OPENIRL_TUNNEL_CHECK_HOST:-}"
ping -c 1 "$OPENIRL_TUNNEL_CHECK_HOST" >/dev/null
echo "tunnel live smoke reached configured host"
