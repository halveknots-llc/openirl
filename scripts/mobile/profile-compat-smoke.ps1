$ErrorActionPreference = 'Stop'
Get-ChildItem presets/encoders/*.json | ForEach-Object { Get-Content $_ | ConvertFrom-Json | Out-Null }
Write-Host 'profile compatibility json parsed'
