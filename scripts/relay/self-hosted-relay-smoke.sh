#!/usr/bin/env bash
set -euo pipefail
if [[ "${OPENIRL_LIVE_RELAY_SMOKE:-}" != "1" ]]; then
  echo "Set OPENIRL_LIVE_RELAY_SMOKE=1 after the relay process and OpenIRL agent are running." >&2
  exit 2
fi

agent_url="${OPENIRL_AGENT_URL:-http://127.0.0.1:7707}"
curl -fsS "$agent_url/api/relay/readiness" | python3 -m json.tool >/dev/null
curl -fsS "$agent_url/api/relay/status" | python3 -m json.tool >/dev/null
echo "self-hosted relay live smoke reached OpenIRL relay endpoints"
