$ErrorActionPreference = 'Stop'
Get-Content presets/relay/srtla2-compat.json | ConvertFrom-Json | Out-Null
Write-Host 'self-hosted relay smoke metadata parsed'
