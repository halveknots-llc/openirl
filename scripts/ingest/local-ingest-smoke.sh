#!/usr/bin/env bash
set -euo pipefail
echo 'Start MediaMTX with presets/relay/mediamtx.openirl.local.yml, then publish to srt://127.0.0.1:9000?streamid=openirl-main'
curl -fsS http://127.0.0.1:7707/api/runtime/readiness
