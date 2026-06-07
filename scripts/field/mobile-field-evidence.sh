#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${OPENIRL_AGENT_URL:-http://127.0.0.1:7707}"
OUT_DIR="${OPENIRL_FIELD_ARTIFACTS:-artifacts/field}"
mkdir -p "$OUT_DIR"

curl -fsS "$BASE_URL/health" > "$OUT_DIR/health.json"
curl -fsS "$BASE_URL/api/field/validation-plan" > "$OUT_DIR/field-validation-plan.json"
curl -fsS "$BASE_URL/api/field/device-checklists" > "$OUT_DIR/device-checklists.json"
curl -fsS "$BASE_URL/api/field/readiness" > "$OUT_DIR/field-readiness-before.json"
curl -fsS "$BASE_URL/api/metrics/latest" > "$OUT_DIR/metrics-before.json" || true
curl -fsS -X POST "$BASE_URL/api/metrics/simulate/brownout" > "$OUT_DIR/metrics-brownout.json" || true
curl -fsS -X POST "$BASE_URL/api/metrics/simulate/healthy" > "$OUT_DIR/metrics-recovery.json" || true
curl -fsS "$BASE_URL/api/session/support-bundle" > "$OUT_DIR/support-bundle.json"

cat > "$OUT_DIR/field-evidence.json" <<'JSON'
{
  "static_validation_passed": true,
  "rust_ci_passed": false,
  "windows_alpha_ready": false,
  "moblin_profile_generated": false,
  "moblin_qr_scanned": false,
  "moblin_ingest_seen": false,
  "irlpro_profile_generated": false,
  "irlpro_qr_scanned": false,
  "irlpro_ingest_seen": false,
  "belabox_profile_generated": false,
  "belabox_config_reviewed": false,
  "belabox_ingest_seen": false,
  "mediamtx_srt_path_active": false,
  "mediamtx_metrics_seen": false,
  "obs_connected": false,
  "obs_media_source_seen": false,
  "healthy_state_seen": false,
  "brownout_state_seen": false,
  "brb_scene_seen": false,
  "recovery_state_seen": false,
  "support_bundle_captured": true,
  "secrets_redacted": false,
  "field_report_written": false
}
JSON

curl -fsS -H 'content-type: application/json' \
  --data "@$OUT_DIR/field-evidence.json" \
  "$BASE_URL/api/field/evidence" > "$OUT_DIR/field-evidence-report.json"

cat > "$OUT_DIR/mobile-field-report.md" <<'EOF_REPORT'
# OpenIRL Mobile Field Report

## Devices

- Moblin:
- IRL Pro:
- BELABOX:

## Path

Phone/backpack -> OpenIRL/MediaMTX -> OBS -> private output/test profile.

## Brownout / recovery notes

- Brownout trigger:
- Time to BRB/fallback:
- Time to stable recovery:
- False switches:

## Blockers

- Review artifacts/field/field-evidence-report.json.

## Redaction

- Stream keys removed:
- SRT passphrases removed:
- Local/public IPs removed:
EOF_REPORT

printf 'Field evidence saved to %s\n' "$OUT_DIR"
