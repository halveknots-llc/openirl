# Windows OBS Alpha Runbook

## Verify before launch

For a tagged prerelease, select the release by its expected version tag and
download the zip, checksum, manifest, and verification report into an otherwise
empty directory. Authenticate the release, compare the archive SHA-256, and
verify every downloaded file against the tag's source ref and the repository's
packaging workflow:

```powershell
$repo = 'halveknots-llc/openirl'
$tag = 'v0.1.0-alpha.0'
$workflow = "$repo/.github/workflows/windows-package.yml"

gh release verify $tag --repo $repo
if ($LASTEXITCODE -ne 0) { throw 'Release authentication failed' }

$expected = ((Get-Content .\openirl-windows-portable-alpha.zip.sha256) -split '\s+')[0].ToLowerInvariant()
$actual = (Get-FileHash -Algorithm SHA256 .\openirl-windows-portable-alpha.zip).Hash.ToLowerInvariant()
if ($actual -ne $expected) { throw 'Archive checksum mismatch' }

$files = @(
  'openirl-windows-portable-alpha.zip',
  'openirl-windows-portable-alpha.zip.sha256',
  'openirl-windows-portable-alpha.manifest.json',
  'openirl-windows-portable-alpha.verification.json'
)
foreach ($file in $files) {
  gh attestation verify ".\$file" `
    --repo $repo `
    --signer-workflow $workflow `
    --source-ref "refs/tags/$tag"
  if ($LASTEXITCODE -ne 0) { throw "Provenance verification failed: $file" }
}
$releaseAuthenticated = $true
```

The provenance attestation does not provide an Authenticode publisher signature.
Review [Release Provenance](../RELEASE_PROVENANCE.md) before bypassing any
Windows reputation warning. The portable workflow is the supported alpha
package path; the MSI script remains an operator-reviewed experimental path.

## Local package verification

Complete the tagged-release authentication above first. Resolve the expected
revision from the independently selected tag, use a clean checkout at that
revision, and place the zip, checksum, and manifest in `dist\windows-alpha`.
Write the new local verification report outside that evidence directory so a
prior report cannot be mistaken for an input:

```powershell
if ($releaseAuthenticated -ne $true) { throw 'Complete tagged-release authentication first' }
$revision = gh api "repos/$repo/commits/$tag" --jq '.sha'
if ($LASTEXITCODE -ne 0) { throw 'Expected tag revision lookup failed' }
$revision = $revision.Trim()
if ($revision -notmatch '\A[0-9a-f]{40}\z') { throw 'Expected tag did not resolve to a full commit' }

git checkout --detach $revision
if ($LASTEXITCODE -ne 0) { throw 'Expected revision checkout failed' }

.\scripts\windows\verify-alpha-portable.ps1 `
  -ArtifactDir dist\windows-alpha `
  -ExpectedRevision $revision `
  -ReportPath .\local-verification.json
```

This local verification supplements release and attestation authentication; it
does not establish artifact authenticity by itself. It reruns the archive,
manifest, inventory, secret-scan, and packaged CLI checks. It does not contact
OBS or MediaMTX.

## OBS integration check

1. Install the exact OBS Studio version you intend to report.
2. Enable OBS WebSocket, require a password, and keep it on a trusted local path.
3. Configure `OPENIRL_OBS_PASSWORD` outside committed files and shell history.
4. Start the verified agent with an operator-reviewed localhost configuration.
5. Run `scripts/smoke/obs-websocket-smoke.ps1` against the real OBS instance.
6. Record the OpenIRL revision, Windows and OBS versions, smoke command, narrow
   result, and reviewed redacted evidence in the compatibility process.
7. Remove credentials and private network or location data before attaching any
   excerpt. Never attach a raw support bundle to the public repository.

A successful package verification proves Windows build and CLI launch behavior,
not OBS scene control or contribution media.
