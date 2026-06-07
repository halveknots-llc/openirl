$ErrorActionPreference = 'Stop'
Get-Content presets/webrtc/whep-preview.json | ConvertFrom-Json | Out-Null
Write-Host 'WHEP preview plan parsed'
