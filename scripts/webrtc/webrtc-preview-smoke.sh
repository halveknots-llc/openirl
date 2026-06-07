#!/usr/bin/env bash
set -euo pipefail
python3 -m json.tool presets/webrtc/whep-preview.json >/dev/null
echo WHEP preview plan parsed
