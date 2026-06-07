#!/usr/bin/env bash
set -euo pipefail
python3 -m json.tool presets/relay/srtla2-compat.json >/dev/null
echo srtla2 compatibility plan parsed
