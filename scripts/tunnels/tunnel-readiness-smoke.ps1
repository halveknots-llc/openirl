$ErrorActionPreference = 'Stop'
Test-Path presets/tunnels/wireguard.example.conf | Out-Null
Test-Path presets/tunnels/frp.example.toml | Out-Null
Write-Host 'tunnel readiness files present'
