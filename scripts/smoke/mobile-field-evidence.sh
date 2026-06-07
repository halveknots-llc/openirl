#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${OPENIRL_BASE_URL:-http://127.0.0.1:7707}"
OUT_DIR="${OPENIRL_FIELD_ARTIFACTS:-artifacts/field}"
SIMULATE="false"

if [[ "${1:-}" == "--simulate" ]]; then
  SIMULATE="true"
fi

mkdir -p "$OUT_DIR"

fetch() {
  local path="$1"
  local out="$2"
  curl -fsS "$BASE_URL$path" -o "$OUT_DIR/$out"
}

post_empty() {
  local path="$1"
  local out="$2"
  curl -fsS -X POST "$BASE_URL$path" -o "$OUT_DIR/$out"
}

fetch /health health.json
fetch /api/runtime/readiness readiness.json
fetch /api/field/readiness field-readiness.json
fetch /api/field/validation-plan field-validation-plan.json
fetch /api/field/operator-checklist field-operator-checklist.json
fetch /api/field/device-matrix field-device-matrix.json
fetch /api/metrics/latest metrics.json
fetch /api/session/report session-report.json
fetch /api/session/support-bundle support-bundle.json
fetch /api/field/report-template field-report-template.json

if [[ "$SIMULATE" == "true" ]]; then
  post_empty /api/metrics/simulate/healthy metrics-sim-healthy.json
  post_empty /api/metrics/simulate/brownout metrics-sim-brownout.json
  post_empty /api/metrics/simulate/healthy metrics-sim-recovery.json
  fetch /api/field/readiness field-readiness-after-sim.json
fi

curl -fsS -X POST "$BASE_URL/api/field/evidence" \
  -H 'content-type: application/json' \
  --data @fixtures/field/evidence-input.sample.json \
  -o "$OUT_DIR/field-evidence-report.json"

curl -fsS -X POST "$BASE_URL/api/field/report" \
  -H 'content-type: application/json' \
  --data @fixtures/field/field-report.sample.json \
  -o "$OUT_DIR/field-report.json"

printf 'wrote field evidence to %s\n' "$OUT_DIR"
