<#
.SYNOPSIS
  Captures feature areas alpha evidence from a running OpenIRL agent.
#>
param(
  [string]$BaseUrl = 'http://127.0.0.1:7707',
  [string]$OutDir = 'artifacts/alpha',
  [switch]$RunObsSmoke,
  [switch]$RunMetricsSimulation
)

$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

function Save-Endpoint {
  param([string]$Path, [string]$FileName)
  $uri = "$BaseUrl$Path"
  $target = Join-Path $OutDir $FileName
  Invoke-RestMethod -Method Get -Uri $uri | ConvertTo-Json -Depth 50 | Set-Content -Encoding UTF8 $target
  return $target
}

$files = [ordered]@{}
$files.health = Save-Endpoint '/health' 'health.json'
$files.readiness = Save-Endpoint '/api/runtime/readiness' 'readiness.json'
$files.alpha = Save-Endpoint '/api/alpha/readiness' 'alpha-readiness.json'
$files.support_bundle = Save-Endpoint '/api/session/support-bundle' 'support-bundle.json'
$files.release = Save-Endpoint '/api/release/manifest' 'release-manifest.json'

if ($RunMetricsSimulation) {
  $metrics = Invoke-RestMethod -Method Post -Uri "$BaseUrl/api/metrics/simulate/healthy"
  $metrics | ConvertTo-Json -Depth 50 | Set-Content -Encoding UTF8 (Join-Path $OutDir 'metrics.json')
  $files.metrics = Join-Path $OutDir 'metrics.json'
}

if ($RunObsSmoke) {
  & "$PSScriptRoot\obs-websocket-smoke.ps1" -Action Status -OutFile (Join-Path $OutDir 'obs-smoke.json')
  $files.obs_smoke = Join-Path $OutDir 'obs-smoke.json'
}

$summary = [ordered]@{
  generated_at = (Get-Date).ToUniversalTime().ToString('o')
  base_url = $BaseUrl
  files = $files
  note = 'Review files for secrets before sharing publicly.'
}
$summary | ConvertTo-Json -Depth 20 | Set-Content -Encoding UTF8 (Join-Path $OutDir 'alpha-evidence-index.json')
$summary | ConvertTo-Json -Depth 20
