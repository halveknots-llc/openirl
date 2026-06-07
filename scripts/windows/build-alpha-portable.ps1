<#
.SYNOPSIS
  Builds a feature areas Windows portable alpha zip after Rust validation passes.
#>
param(
  [string]$Configuration = 'release',
  [string]$OutDir = 'dist/windows-alpha',
  [switch]$SkipCargoBuild
)

$ErrorActionPreference = 'Stop'
$root = Resolve-Path (Join-Path $PSScriptRoot '..\..')
Set-Location $root

if (-not $SkipCargoBuild) {
  cargo xtask ci
  cargo build --workspace --release --features openirl-agent/obs-websocket
}

$stage = Join-Path $OutDir 'OpenIRL'
if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
New-Item -ItemType Directory -Force -Path $stage | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $stage 'config') | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $stage 'static') | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $stage 'scripts') | Out-Null

Copy-Item "target\release\openirl-agent.exe" (Join-Path $stage 'openirl-agent.exe') -Force
Copy-Item "target\release\openirl-desktop.exe" (Join-Path $stage 'openirl-desktop.exe') -Force
Copy-Item "config\openirl.example.toml" (Join-Path $stage 'config\openirl.example.toml') -Force
Copy-Item "apps\openirl-agent\static\*" (Join-Path $stage 'static') -Recurse -Force
Copy-Item "scripts\smoke\*.ps1" (Join-Path $stage 'scripts') -Force
Copy-Item "README.md" (Join-Path $stage 'README.md') -Force
Copy-Item "docs\runbooks\WINDOWS_OBS_ALPHA.md" (Join-Path $stage 'WINDOWS_OBS_ALPHA.md') -Force

$zip = Join-Path $OutDir 'openirl-windows-portable-alpha.zip'
if (Test-Path $zip) { Remove-Item -Force $zip }
Compress-Archive -Path $stage -DestinationPath $zip -Force
$hash = Get-FileHash -Algorithm SHA256 $zip
"$($hash.Hash.ToLower())  openirl-windows-portable-alpha.zip" | Set-Content -Encoding ASCII "$zip.sha256"
Write-Host "Portable alpha created: $zip"
Write-Host "SHA256: $($hash.Hash.ToLower())"
