$ErrorActionPreference = 'Stop'
Write-Host 'Start MediaMTX with presets/relay/mediamtx.openirl.local.yml and publish SRT to openirl-main'
Invoke-RestMethod http://127.0.0.1:7707/api/runtime/readiness
