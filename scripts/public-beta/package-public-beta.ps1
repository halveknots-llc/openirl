$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force artifacts/v1-public-beta | Out-Null
Copy-Item -Recurse -Force docs,presets,issue_templates,plugin artifacts/v1-public-beta
Write-Host 'public beta package refreshed'
