<#
.SYNOPSIS
  Independently verifies a Windows portable alpha artifact and packaged CLIs.
#>
param(
  [string]$ArtifactDir = 'dist/windows-alpha',
  [Parameter(Mandatory = $true)]
  [string]$ExpectedRevision,
  [string]$ReportPath = 'dist/windows-alpha/openirl-windows-portable-alpha.verification.json'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Assert-LastExitCode([string]$Description) {
  if ($LASTEXITCODE -ne 0) {
    throw "$Description failed with exit code $LASTEXITCODE"
  }
}

function Write-Utf8NoBom([string]$Path, [string]$Content) {
  $parent = Split-Path -Parent $Path
  if ($parent) {
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
  }
  $encoding = [System.Text.UTF8Encoding]::new($false)
  [System.IO.File]::WriteAllText($Path, $Content, $encoding)
}

$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
Set-Location $root
$artifactRoot = (Resolve-Path $ArtifactDir).Path
$zip = Join-Path $artifactRoot 'openirl-windows-portable-alpha.zip'
$checksum = "$zip.sha256"
$externalManifest = Join-Path $artifactRoot 'openirl-windows-portable-alpha.manifest.json'
foreach ($path in @($zip, $checksum, $externalManifest)) {
  if (-not (Test-Path -PathType Leaf $path)) {
    throw "Required release artifact is missing: $path"
  }
}

$expected = $ExpectedRevision.Trim().ToLowerInvariant()
if ($expected -notmatch '^[0-9a-f]{40}$') {
  throw 'ExpectedRevision must be a full lowercase Git commit'
}

python scripts\security\release-artifact-scan.py --archive $zip --manifest $externalManifest --forbid-local-root $root
Assert-LastExitCode 'Release artifact secret scan'

$checksumLine = (Get-Content -Raw $checksum).Trim()
if ($checksumLine -notmatch '^([0-9a-f]{64})  openirl-windows-portable-alpha\.zip$') {
  throw 'Checksum file does not use the expected SHA256 format'
}
$recordedZipHash = $Matches[1]
$actualZipHash = (Get-FileHash -Algorithm SHA256 $zip).Hash.ToLowerInvariant()
if ($recordedZipHash -ne $actualZipHash) {
  throw 'Portable archive checksum does not match'
}

$manifest = Get-Content -Raw $externalManifest | ConvertFrom-Json
if ($manifest.schema_version -ne 1 -or $manifest.package -ne 'openirl-windows-portable-alpha') {
  throw 'Package manifest identity or schema is invalid'
}
if ($manifest.source_revision -ne $expected) {
  throw 'Package manifest source revision does not match the expected revision'
}
if ($manifest.build.target -notmatch 'windows') {
  throw 'Package manifest does not identify a Windows Rust target'
}
if ($manifest.build.cargo_lock_sha256 -notmatch '^[0-9a-f]{64}$' -or
    [long]$manifest.source_commit_epoch -le 0) {
  throw 'Package manifest does not contain valid source build inputs'
}
if ($manifest.validation.source_gate -ne 'cargo xtask ci') {
  throw 'Package manifest does not record the required source gate'
}
foreach ($integration in @('obs_studio', 'mediamtx', 'mobile_encoder', 'belabox', 'srtla')) {
  if ($manifest.integration_evidence.$integration -ne 'not-run') {
    throw "Package manifest overstates live integration evidence for $integration"
  }
}

$extractRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("openirl-verify-" + [Guid]::NewGuid().ToString('N'))
try {
  Expand-Archive -Path $zip -DestinationPath $extractRoot
  $packageRoot = Join-Path $extractRoot 'OpenIRL'
  if (-not (Test-Path -PathType Container $packageRoot)) {
    throw 'Portable archive does not contain the OpenIRL package root'
  }

  $embeddedManifest = Join-Path $packageRoot 'package-manifest.json'
  if (-not (Test-Path -PathType Leaf $embeddedManifest)) {
    throw 'Portable archive does not contain its package manifest'
  }
  $embeddedHash = (Get-FileHash -Algorithm SHA256 $embeddedManifest).Hash.ToLowerInvariant()
  $externalHash = (Get-FileHash -Algorithm SHA256 $externalManifest).Hash.ToLowerInvariant()
  if ($embeddedHash -ne $externalHash) {
    throw 'Embedded and external package manifests differ'
  }

  $records = @($manifest.files)
  $recordMap = @{}
  foreach ($record in $records) {
    if ($record.path -match '(^|/)\.\.?($|/)' -or $recordMap.ContainsKey($record.path)) {
      throw "Package manifest contains an unsafe or duplicate path: $($record.path)"
    }
    $recordMap[$record.path] = $record
  }
  foreach ($requiredPath in @(
      'openirl-agent.exe',
      'openirl-desktop.exe',
      'config/openirl.example.toml',
      'static/index.html',
      'source-revision.txt'
    )) {
    if (-not $recordMap.ContainsKey($requiredPath)) {
      throw "Package manifest is missing a required payload: $requiredPath"
    }
  }
  $actualFiles = @(Get-ChildItem -Path $packageRoot -File -Recurse | Where-Object {
      $_.FullName -ne $embeddedManifest
    })
  if ($actualFiles.Count -ne $records.Count) {
    throw 'Package file count does not match the manifest'
  }
  foreach ($file in $actualFiles) {
    $relative = [System.IO.Path]::GetRelativePath($packageRoot, $file.FullName).Replace('\', '/')
    if (-not $recordMap.ContainsKey($relative)) {
      throw "Package contains an unmanifested file: $relative"
    }
    $record = $recordMap[$relative]
    if ([long]$record.size -ne $file.Length) {
      throw "Package file size differs from the manifest: $relative"
    }
    $hash = (Get-FileHash -Algorithm SHA256 $file.FullName).Hash.ToLowerInvariant()
    if ($hash -ne $record.sha256) {
      throw "Package file hash differs from the manifest: $relative"
    }
  }

  $packagedRevision = (Get-Content -Raw (Join-Path $packageRoot 'source-revision.txt')).Trim()
  if ($packagedRevision -ne $expected) {
    throw 'Packaged source revision does not match the expected revision'
  }

  $readiness = & (Join-Path $packageRoot 'openirl-agent.exe') readiness | ConvertFrom-Json
  Assert-LastExitCode 'Packaged agent readiness check'
  if ($readiness.mode -ne 'standard' -or $readiness.summary.live_environment.satisfied -ne 0) {
    throw 'Packaged agent readiness output inferred live integration evidence'
  }
  $desktopPlan = & (Join-Path $packageRoot 'openirl-desktop.exe') plan | ConvertFrom-Json
  Assert-LastExitCode 'Packaged desktop plan check'
  if (-not $desktopPlan.menu_items -or $desktopPlan.dashboard_url -ne 'http://127.0.0.1:7707/') {
    throw 'Packaged desktop plan output is invalid'
  }

  $report = [ordered]@{
    schema_version = 1
    package = 'openirl-windows-portable-alpha'
    source_revision = $expected
    archive_sha256 = $actualZipHash
    manifest_sha256 = $externalHash
    verifier = 'scripts/windows/verify-alpha-portable.ps1'
    checks = [ordered]@{
      checksum = 'passed'
      manifest = 'passed'
      enumerated_files = 'passed'
      artifact_secret_scan = 'passed'
      agent_readiness = 'passed'
      desktop_plan = 'passed'
    }
    live_integrations = [ordered]@{
      obs_studio = 'not-run'
      mediamtx = 'not-run'
      mobile_encoder = 'not-run'
      belabox = 'not-run'
      srtla = 'not-run'
    }
  }
  $reportJson = $report | ConvertTo-Json -Depth 6
  Write-Utf8NoBom ([System.IO.Path]::GetFullPath((Join-Path $root $ReportPath))) "$reportJson`n"
}
finally {
  if (Test-Path $extractRoot) {
    Remove-Item -Recurse -Force $extractRoot
  }
}

Write-Host "Portable alpha verified for source revision $expected"
