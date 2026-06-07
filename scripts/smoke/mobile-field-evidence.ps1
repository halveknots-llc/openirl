param(
  [string]$BaseUrl = "http://127.0.0.1:7707",
  [string]$OutDir = "artifacts/field",
  [switch]$RunMetricsSimulation
)

$ErrorActionPreference = "Stop"
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null

function Save-Get([string]$Path, [string]$Name) {
  Invoke-RestMethod -Method Get -Uri "$BaseUrl$Path" | ConvertTo-Json -Depth 20 | Set-Content -Encoding UTF8 -Path (Join-Path $OutDir $Name)
}

function Save-Post([string]$Path, [string]$Name) {
  Invoke-RestMethod -Method Post -Uri "$BaseUrl$Path" | ConvertTo-Json -Depth 20 | Set-Content -Encoding UTF8 -Path (Join-Path $OutDir $Name)
}

Save-Get "/health" "health.json"
Save-Get "/api/runtime/readiness" "readiness.json"
Save-Get "/api/field/readiness" "field-readiness.json"
Save-Get "/api/field/validation-plan" "field-validation-plan.json"
Save-Get "/api/field/operator-checklist" "field-operator-checklist.json"
Save-Get "/api/field/device-matrix" "field-device-matrix.json"
Save-Get "/api/metrics/latest" "metrics.json"
Save-Get "/api/session/report" "session-report.json"
Save-Get "/api/session/support-bundle" "support-bundle.json"
Save-Get "/api/field/report-template" "field-report-template.json"

if ($RunMetricsSimulation) {
  Save-Post "/api/metrics/simulate/healthy" "metrics-sim-healthy.json"
  Save-Post "/api/metrics/simulate/brownout" "metrics-sim-brownout.json"
  Save-Post "/api/metrics/simulate/healthy" "metrics-sim-recovery.json"
  Save-Get "/api/field/readiness" "field-readiness-after-sim.json"
}

$evidence = Get-Content -Raw -Path "fixtures/field/evidence-input.sample.json"
Invoke-RestMethod -Method Post -Uri "$BaseUrl/api/field/evidence" -ContentType "application/json" -Body $evidence | ConvertTo-Json -Depth 20 | Set-Content -Encoding UTF8 -Path (Join-Path $OutDir "field-evidence-report.json")

$report = Get-Content -Raw -Path "fixtures/field/field-report.sample.json"
Invoke-RestMethod -Method Post -Uri "$BaseUrl/api/field/report" -ContentType "application/json" -Body $report | ConvertTo-Json -Depth 20 | Set-Content -Encoding UTF8 -Path (Join-Path $OutDir "field-report.json")

Write-Host "Wrote field evidence to $OutDir"
