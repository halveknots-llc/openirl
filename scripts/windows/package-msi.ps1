$ErrorActionPreference = "Stop"

if (-not (Get-Command wix.exe -ErrorAction SilentlyContinue)) {
  Write-Error "WiX v4 CLI (wix.exe) was not found. Install WiX before building the MSI."
}

cargo build --release --package openirl-agent --package openirl-desktop
New-Item -ItemType Directory -Force -Path dist\windows | Out-Null
Copy-Item target\release\openirl-agent.exe dist\windows\openirl-agent.exe -Force
Copy-Item target\release\openirl-desktop.exe dist\windows\openirl-desktop.exe -Force
Copy-Item config\openirl.example.toml dist\windows\openirl.example.toml -Force
wix build deploy\windows\openirl-agent.wxs -o dist\windows\OpenIRL-Agent-alpha.msi
Write-Host "Built dist\windows\OpenIRL-Agent-alpha.msi"
