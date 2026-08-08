$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
Set-Location $root

Write-Warning 'public evidence review: remove stream credentials, authentication credentials, credential-bearing URLs, private network details, local paths, device identifiers, location-sensitive media, private-production stream IDs, and raw support bundles'

$status = git status --porcelain --untracked-files=all
if ($status) {
  throw 'refusing to package a dirty worktree; commit or clean all changes first'
}

$revision = (git rev-parse HEAD).Trim()
$staging = Join-Path ([System.IO.Path]::GetTempPath()) ("openirl-public-beta-" + [guid]::NewGuid())
$archive = Join-Path $staging 'source.tar'
New-Item -ItemType Directory -Force $staging | Out-Null
try {
  git archive --format=tar --worktree-attributes --output=$archive HEAD docs presets issue_templates plugin
  if ($LASTEXITCODE -ne 0) { throw 'git archive failed' }
  tar -xf $archive -C $staging
  if ($LASTEXITCODE -ne 0) { throw 'tar extraction failed' }

  $output = Join-Path $root 'artifacts\v1-public-beta'
  if (Test-Path $output) { Remove-Item -Recurse -Force $output }
  New-Item -ItemType Directory -Force $output | Out-Null
  Copy-Item -Recurse -Force (Join-Path $staging 'docs'),(Join-Path $staging 'presets'),(Join-Path $staging 'issue_templates'),(Join-Path $staging 'plugin') $output
  Set-Content -Encoding UTF8 -Path (Join-Path $output 'package-manifest.json') -Value ('{"schema_version":1,"source_revision":"' + $revision + '","paths":["docs","presets","issue_templates","plugin"]}')
  Write-Host "public beta package refreshed from $revision"
}
finally {
  if (Test-Path $staging) { Remove-Item -Recurse -Force $staging }
}
