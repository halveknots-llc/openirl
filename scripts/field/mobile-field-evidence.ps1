param(
  [string]$BaseUrl = $(if ($env:OPENIRL_AGENT_URL) { $env:OPENIRL_AGENT_URL } else { 'http://127.0.0.1:7707' }),
  [string]$OutDir = $(if ($env:OPENIRL_FIELD_ARTIFACTS) { $env:OPENIRL_FIELD_ARTIFACTS } else { 'artifacts/field' })
)

$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

Invoke-RestMethod "$BaseUrl/health" | ConvertTo-Json -Depth 20 | Set-Content "$OutDir/health.json"
Invoke-RestMethod "$BaseUrl/api/field/validation-plan" | ConvertTo-Json -Depth 30 | Set-Content "$OutDir/field-validation-plan.json"
Invoke-RestMethod "$BaseUrl/api/field/device-checklists" | ConvertTo-Json -Depth 30 | Set-Content "$OutDir/device-checklists.json"
Invoke-RestMethod "$BaseUrl/api/field/readiness" | ConvertTo-Json -Depth 30 | Set-Content "$OutDir/field-readiness-before.json"

try { Invoke-RestMethod "$BaseUrl/api/metrics/latest" | ConvertTo-Json -Depth 30 | Set-Content "$OutDir/metrics-before.json" } catch {}
try { Invoke-RestMethod -Method Post "$BaseUrl/api/metrics/simulate/brownout" | ConvertTo-Json -Depth 30 | Set-Content "$OutDir/metrics-brownout.json" } catch {}
try { Invoke-RestMethod -Method Post "$BaseUrl/api/metrics/simulate/healthy" | ConvertTo-Json -Depth 30 | Set-Content "$OutDir/metrics-recovery.json" } catch {}
Invoke-RestMethod "$BaseUrl/api/session/support-bundle" | ConvertTo-Json -Depth 40 | Set-Content "$OutDir/support-bundle.json"

$evidence = [ordered]@{
  static_validation_passed = $true
  rust_ci_passed = $false
  windows_alpha_ready = $false
  moblin_profile_generated = $false
  moblin_qr_scanned = $false
  moblin_ingest_seen = $false
  irlpro_profile_generated = $false
  irlpro_qr_scanned = $false
  irlpro_ingest_seen = $false
  belabox_profile_generated = $false
  belabox_config_reviewed = $false
  belabox_ingest_seen = $false
  mediamtx_srt_path_active = $false
  mediamtx_metrics_seen = $false
  obs_connected = $false
  obs_media_source_seen = $false
  healthy_state_seen = $false
  brownout_state_seen = $false
  brb_scene_seen = $false
  recovery_state_seen = $false
  support_bundle_captured = $true
  secrets_redacted = $false
  field_report_written = $false
}
$evidenceJson = $evidence | ConvertTo-Json -Depth 20
$evidenceJson | Set-Content "$OutDir/field-evidence.json"
Invoke-RestMethod -Method Post -ContentType 'application/json' -Body $evidenceJson "$BaseUrl/api/field/evidence" |
  ConvertTo-Json -Depth 30 | Set-Content "$OutDir/field-evidence-report.json"

@'
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
'@ | Set-Content "$OutDir/mobile-field-report.md"

Write-Host "Field evidence saved to $OutDir"
