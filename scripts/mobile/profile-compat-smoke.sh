#!/usr/bin/env bash
set -euo pipefail
for f in presets/encoders/*.json; do python3 -m json.tool "$f" >/dev/null; done
echo profile compatibility json parsed
