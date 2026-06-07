#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${OPENIRL_AGENT_URL:-http://127.0.0.1:7707}"
OUT_DIR="${OPENIRL_FIELD_ARTIFACTS:-artifacts/field}"
mkdir -p "$OUT_DIR"

cat > "$OUT_DIR/mediamtx-srt-field-smoke.md" <<'REPORT'
# MediaMTX SRT Field Smoke

This script captures OpenIRL-side state. A real encoder must publish separately.

Manual publish examples:

- Moblin/IRL Pro: scan OpenIRL profile QR and start stream.
- BELABOX: set SRT/SRTLA endpoint and start encoder.
- FFmpeg lab source: publish a short SRT test pattern to the configured SRT listener.
REPORT

curl -fsS "$BASE_URL/api/relay/status" > "$OUT_DIR/relay-status.json" || true
curl -fsS "$BASE_URL/api/metrics/sources" > "$OUT_DIR/metrics-sources.json" || true
curl -fsS -X POST "$BASE_URL/api/metrics/poll" > "$OUT_DIR/metrics-poll.json" || true
curl -fsS "$BASE_URL/api/field/readiness" > "$OUT_DIR/field-readiness-after-poll.json" || true
printf 'MediaMTX/OpenIRL field smoke artifacts saved to %s\n' "$OUT_DIR"
