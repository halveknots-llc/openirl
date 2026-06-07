$ErrorActionPreference = 'Stop'
Invoke-RestMethod http://127.0.0.1:7707/api/obs/template | ConvertTo-Json -Depth 10
Invoke-RestMethod -Method Post http://127.0.0.1:7707/api/obs/template/apply
