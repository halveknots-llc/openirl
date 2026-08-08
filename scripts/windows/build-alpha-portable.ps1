<#
.SYNOPSIS
  Builds a source-pinned Windows portable alpha artifact after all local gates pass.
#>
param(
  [string]$OutDir = 'dist/windows-alpha',
  [string]$ExpectedRevision = $env:OPENIRL_EXPECTED_REVISION
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Assert-LastExitCode([string]$Description) {
  if ($LASTEXITCODE -ne 0) {
    throw "$Description failed with exit code $LASTEXITCODE"
  }
}

function Assert-CleanWorktree([string]$Stage) {
  $status = @(git -c core.quotepath=true status --porcelain=v1 --untracked-files=all)
  Assert-LastExitCode 'Git worktree inspection'
  if ($status.Count -ne 0) {
    $visible = @($status | Select-Object -First 20) -join '; '
    $suffix = if ($status.Count -gt 20) { '; additional paths omitted' } else { '' }
    throw "Release packaging requires a clean worktree at $Stage; detected $($status.Count) tracked or untracked path(s): $visible$suffix"
  }
}

function Copy-TrackedFile([string]$Source, [string]$Destination) {
  $pathspec = $Source.Replace('\', '/')
  git ls-files --error-unmatch -- $pathspec | Out-Null
  Assert-LastExitCode "Tracked-file check for $Source"
  $parent = Split-Path -Parent $Destination
  if ($parent) {
    New-Item -ItemType Directory -Force -Path $parent | Out-Null
  }
  Copy-Item $pathspec $Destination -Force
}

function Write-Utf8NoBom([string]$Path, [string]$Content) {
  $encoding = [System.Text.UTF8Encoding]::new($false)
  [System.IO.File]::WriteAllText($Path, $Content, $encoding)
}

function New-DeterministicZip(
  [string]$InputRoot,
  [string]$ArchivePath,
  [long]$Timestamp
) {
  Add-Type -AssemblyName System.IO.Compression
  if (Test-Path $ArchivePath) {
    Remove-Item -Force $ArchivePath
  }
  $files = @(Get-ChildItem -Path $InputRoot -File -Recurse | Sort-Object {
      [System.IO.Path]::GetRelativePath($InputRoot, $_.FullName).Replace('\', '/')
    })
  $stream = [System.IO.File]::Open(
    $ArchivePath,
    [System.IO.FileMode]::CreateNew,
    [System.IO.FileAccess]::Write,
    [System.IO.FileShare]::None
  )
  try {
    $archive = [System.IO.Compression.ZipArchive]::new(
      $stream,
      [System.IO.Compression.ZipArchiveMode]::Create,
      $false
    )
    try {
      foreach ($file in $files) {
        $relative = [System.IO.Path]::GetRelativePath($InputRoot, $file.FullName).Replace('\', '/')
        $entry = $archive.CreateEntry(
          $relative,
          [System.IO.Compression.CompressionLevel]::Optimal
        )
        $entry.LastWriteTime = [System.DateTimeOffset]::FromUnixTimeSeconds($Timestamp)
        $input = $file.OpenRead()
        $output = $entry.Open()
        try {
          $input.CopyTo($output)
        }
        finally {
          $output.Dispose()
          $input.Dispose()
        }
      }
    }
    finally {
      $archive.Dispose()
    }
  }
  finally {
    $stream.Dispose()
  }
}

$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
Set-Location $root
$distRoot = [System.IO.Path]::GetFullPath((Join-Path $root 'dist'))
$outFull = [System.IO.Path]::GetFullPath((Join-Path $root $OutDir))
$requiredPrefix = $distRoot + [System.IO.Path]::DirectorySeparatorChar
if (-not $outFull.StartsWith($requiredPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
  throw 'OutDir must resolve beneath the repository dist directory'
}

Assert-CleanWorktree 'pre-validation'
git fsck --full --no-progress
Assert-LastExitCode 'Git object verification'

$revision = (git rev-parse --verify HEAD).Trim()
Assert-LastExitCode 'Source revision lookup'
if ($revision -notmatch '^[0-9a-f]{40}$') {
  throw 'Source revision is not a full lowercase Git commit'
}
if ($ExpectedRevision -and $revision -ne $ExpectedRevision.Trim().ToLowerInvariant()) {
  throw 'Checked-out source revision does not match the expected workflow revision'
}

cargo xtask ci
Assert-LastExitCode 'OpenIRL source validation'
Assert-CleanWorktree 'post-validation'

if ($env:RUSTFLAGS -or $env:CARGO_ENCODED_RUSTFLAGS) {
  throw 'Release packaging does not accept caller-provided Rust compiler flags'
}

$commitEpoch = [long](git show -s --format=%ct HEAD).Trim()
Assert-LastExitCode 'Source timestamp lookup'
$env:CARGO_INCREMENTAL = '0'
$env:SOURCE_DATE_EPOCH = $commitEpoch.ToString()
$env:CARGO_ENCODED_RUSTFLAGS = "--remap-path-prefix=$root=."
try {
  cargo build --locked --release --package openirl-agent --package openirl-desktop --features openirl-agent/obs-websocket
  Assert-LastExitCode 'Windows release build'
}
finally {
  Remove-Item Env:CARGO_ENCODED_RUSTFLAGS -ErrorAction SilentlyContinue
  Remove-Item Env:CARGO_INCREMENTAL -ErrorAction SilentlyContinue
  Remove-Item Env:SOURCE_DATE_EPOCH -ErrorAction SilentlyContinue
}

$binarySources = @(
  'target\release\openirl-agent.exe',
  'target\release\openirl-desktop.exe'
)
foreach ($binary in $binarySources) {
  if (-not (Test-Path -PathType Leaf $binary)) {
    throw "Required package binary is missing: $binary"
  }
}
Assert-CleanWorktree 'post-build'

if (Test-Path $outFull) {
  Remove-Item -Recurse -Force $outFull
}
New-Item -ItemType Directory -Force -Path $outFull | Out-Null
$stage = Join-Path $outFull 'OpenIRL'
New-Item -ItemType Directory -Force -Path $stage | Out-Null

Copy-Item $binarySources[0] (Join-Path $stage 'openirl-agent.exe') -Force
Copy-Item $binarySources[1] (Join-Path $stage 'openirl-desktop.exe') -Force
Copy-TrackedFile 'config\openirl.example.toml' (Join-Path $stage 'config\openirl.example.toml')
Copy-TrackedFile 'README.md' (Join-Path $stage 'README.md')
Copy-TrackedFile 'docs\runbooks\WINDOWS_OBS_ALPHA.md' (Join-Path $stage 'WINDOWS_OBS_ALPHA.md')

$staticFiles = @(Get-ChildItem 'apps\openirl-agent\static' -File -Recurse)
foreach ($file in $staticFiles) {
  $source = [System.IO.Path]::GetRelativePath($root, $file.FullName)
  $relative = [System.IO.Path]::GetRelativePath((Join-Path $root 'apps\openirl-agent\static'), $file.FullName)
  Copy-TrackedFile $source (Join-Path $stage (Join-Path 'static' $relative))
}
$smokeFiles = @(Get-ChildItem 'scripts\smoke' -File -Filter '*.ps1')
foreach ($file in $smokeFiles) {
  $source = [System.IO.Path]::GetRelativePath($root, $file.FullName)
  Copy-TrackedFile $source (Join-Path $stage (Join-Path 'scripts' $file.Name))
}

Write-Utf8NoBom (Join-Path $stage 'source-revision.txt') "$revision`n"

$metadata = cargo metadata --locked --no-deps --format-version 1 | ConvertFrom-Json
Assert-LastExitCode 'Cargo metadata collection'
$agentPackage = @($metadata.packages | Where-Object { $_.name -eq 'openirl-agent' })
if ($agentPackage.Count -ne 1) {
  throw 'Cargo metadata did not identify exactly one openirl-agent package'
}
$rustcVersion = (rustc --version).Trim()
Assert-LastExitCode 'Rust compiler version lookup'
$rustcVerbose = @(rustc -vV)
Assert-LastExitCode 'Rust target lookup'
$rustHost = (($rustcVerbose | Where-Object { $_ -like 'host:*' }) -replace '^host:\s*', '').Trim()
if (-not $rustHost) {
  throw 'Rust target host was not reported'
}

$fileRecords = @()
$payloadFiles = @(Get-ChildItem -Path $stage -File -Recurse | Sort-Object {
    [System.IO.Path]::GetRelativePath($stage, $_.FullName).Replace('\', '/')
  })
foreach ($file in $payloadFiles) {
  $relative = [System.IO.Path]::GetRelativePath($stage, $file.FullName).Replace('\', '/')
  if ($relative -match '(^|/)(\._|\.env($|\.)|\.git($|/))' -or $relative -match '(?i)support-bundle') {
    throw "Unsafe package path was staged: $relative"
  }
  $fileRecords += [ordered]@{
    path = $relative
    size = $file.Length
    sha256 = (Get-FileHash -Algorithm SHA256 $file.FullName).Hash.ToLowerInvariant()
  }
}

$manifest = [ordered]@{
  schema_version = 1
  package = 'openirl-windows-portable-alpha'
  package_version = $agentPackage[0].version
  source_revision = $revision
  source_commit_epoch = $commitEpoch
  build = [ordered]@{
    profile = 'release'
    target = $rustHost
    rustc = $rustcVersion
    cargo_lock_sha256 = (Get-FileHash -Algorithm SHA256 'Cargo.lock').Hash.ToLowerInvariant()
    source_path_remapped = $true
    runner_image = if ($env:ImageOS) { $env:ImageOS } else { 'local-windows' }
    runner_image_version = if ($env:ImageVersion) { $env:ImageVersion } else { 'not-recorded' }
  }
  validation = [ordered]@{
    source_gate = 'cargo xtask ci'
    clean_worktree = $true
    git_object_check = 'passed'
    package_allowlist = 'explicit-tracked-inputs'
    artifact_secret_scan = 'required-before-publication'
  }
  integration_evidence = [ordered]@{
    windows_package_build = 'built-on-windows-host'
    obs_studio = 'not-run'
    mediamtx = 'not-run'
    mobile_encoder = 'not-run'
    belabox = 'not-run'
    srtla = 'not-run'
  }
  files = $fileRecords
}

$manifestPath = Join-Path $stage 'package-manifest.json'
$manifestJson = $manifest | ConvertTo-Json -Depth 8
Write-Utf8NoBom $manifestPath "$manifestJson`n"

$zip = Join-Path $outFull 'openirl-windows-portable-alpha.zip'
New-DeterministicZip $outFull $zip $commitEpoch
$zipHash = (Get-FileHash -Algorithm SHA256 $zip).Hash.ToLowerInvariant()
$checksumPath = "$zip.sha256"
Write-Utf8NoBom $checksumPath "$zipHash  openirl-windows-portable-alpha.zip`n"
$externalManifest = Join-Path $outFull 'openirl-windows-portable-alpha.manifest.json'
Copy-Item $manifestPath $externalManifest -Force

python scripts\security\release-artifact-scan.py --archive $zip --manifest $externalManifest --scan-file $checksumPath --forbid-local-root $root
Assert-LastExitCode 'Release artifact secret scan'

Write-Host "Portable alpha created: $zip"
Write-Host "Source revision: $revision"
Write-Host "SHA256: $zipHash"
