# Windows OBS Alpha Runbook

## Verify before launch

For a workflow artifact or tagged prerelease, download the zip, checksum,
manifest, and verification report. Compare the archive SHA-256, then verify its
signed provenance:

```powershell
Get-FileHash -Algorithm SHA256 .\openirl-windows-portable-alpha.zip
Get-Content .\openirl-windows-portable-alpha.zip.sha256
gh attestation verify .\openirl-windows-portable-alpha.zip `
  --repo halveknots-llc/openirl
```

The provenance attestation does not provide an Authenticode publisher signature.
Review [Release Provenance](../RELEASE_PROVENANCE.md) before bypassing any
Windows reputation warning. The portable workflow is the supported alpha
package path; the MSI script remains an operator-reviewed experimental path.

## Local package verification

From a clean checkout at the manifest's `source_revision`:

```powershell
$revision = (Get-Content -Raw .\dist\windows-alpha\openirl-windows-portable-alpha.manifest.json | ConvertFrom-Json).source_revision
.\scripts\windows\verify-alpha-portable.ps1 `
  -ArtifactDir dist\windows-alpha `
  -ExpectedRevision $revision
```

This reruns the archive, manifest, inventory, secret-scan, and packaged CLI
checks. It does not contact OBS or MediaMTX.

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
