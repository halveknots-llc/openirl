param(
  [string]$BaseUrl = $(if ($env:OPENIRL_AGENT_URL) { $env:OPENIRL_AGENT_URL } else { 'http://127.0.0.1:7707' }),
  [string]$OutDir = $(if ($env:OPENIRL_FIELD_ARTIFACTS) { $env:OPENIRL_FIELD_ARTIFACTS } else { 'artifacts/field' })
)

$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
@'
# MediaMTX SRT Field Smoke

This script captures OpenIRL-side state. A real encoder must publish separately.

Manual publish examples:

- Moblin/IRL Pro: scan OpenIRL profile QR and start stream.
- BELABOX: set SRT/SRTLA endpoint and start encoder.
- FFmpeg lab source: publish a short SRT test pattern to the configured SRT listener.
'@ | Set-Content "$OutDir/mediamtx-srt-field-smoke.md"

try { Invoke-RestMethod "$BaseUrl/api/relay/status" | ConvertTo-Json -Depth 30 | Set-Content "$OutDir/relay-status.json" } catch {}
try { Invoke-RestMethod "$BaseUrl/api/metrics/sources" | ConvertTo-Json -Depth 30 | Set-Content "$OutDir/metrics-sources.json" } catch {}
try { Invoke-RestMethod -Method Post "$BaseUrl/api/metrics/poll" | ConvertTo-Json -Depth 30 | Set-Content "$OutDir/metrics-poll.json" } catch {}
try { Invoke-RestMethod "$BaseUrl/api/field/readiness" | ConvertTo-Json -Depth 30 | Set-Content "$OutDir/field-readiness-after-poll.json" } catch {}
Write-Host "MediaMTX/OpenIRL field smoke artifacts saved to $OutDir"
