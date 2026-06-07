#!/usr/bin/env bash
set -euo pipefail
BASE_URL="${BASE_URL:-http://127.0.0.1:7707}"
OUT_DIR="${OUT_DIR:-artifacts/alpha}"
mkdir -p "$OUT_DIR"

fetch() {
  local path="$1"
  local file="$2"
  curl --fail --silent --show-error "$BASE_URL$path" > "$OUT_DIR/$file"
}

fetch /health health.json
fetch /api/runtime/readiness readiness.json
fetch /api/alpha/readiness alpha-readiness.json
fetch /api/release/manifest release-manifest.json
fetch /api/session/support-bundle support-bundle.json
curl --fail --silent --show-error -X POST "$BASE_URL/api/metrics/simulate/healthy" > "$OUT_DIR/metrics.json"
python3 - <<'PY'
from pathlib import Path
print('alpha smoke artifacts:')
for path in sorted(Path('artifacts/alpha').glob('*.json')):
    print('-', path)
PY
