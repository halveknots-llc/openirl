#!/usr/bin/env bash
set -euo pipefail
if [[ "${OPENIRL_LIVE_SRTLA_SMOKE:-}" != "1" ]]; then
  echo "Set OPENIRL_LIVE_SRTLA_SMOKE=1 and OPENIRL_SRTLA_RECEIVER_CMD after the receiver is installed." >&2
  exit 2
fi

python3 -m json.tool presets/relay/srtla2-compat.json >/dev/null
test -n "${OPENIRL_SRTLA_RECEIVER_CMD:-}"
command -v "$OPENIRL_SRTLA_RECEIVER_CMD" >/dev/null
echo "srtla2 compatibility smoke found configured receiver command"
