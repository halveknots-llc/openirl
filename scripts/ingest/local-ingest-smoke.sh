#!/usr/bin/env bash
set -euo pipefail
if [[ "${OPENIRL_LIVE_INGEST_SMOKE:-}" != "1" ]]; then
  echo "Set OPENIRL_LIVE_INGEST_SMOKE=1 after MediaMTX and a publisher are running." >&2
  exit 2
fi

agent_url="${OPENIRL_AGENT_URL:-http://127.0.0.1:7707}"
mediamtx_api="${OPENIRL_MEDIAMTX_API:-http://127.0.0.1:9997/v3/paths/list}"

curl -fsS "$agent_url/api/runtime/readiness" >/dev/null
curl -fsS "$mediamtx_api" >/dev/null
echo "local ingest live smoke reached OpenIRL and MediaMTX"
