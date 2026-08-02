# OpenIRL Windows Portable Alpha

This prerelease contains the local-first OpenIRL agent, desktop command shell,
dashboard assets, example localhost configuration, and Windows smoke scripts.

## Verified by the release workflow

- The source checkout is clean, its Git objects pass verification, and its full
  commit matches the release workflow revision.
- `cargo xtask ci` passes before release compilation.
- Release binaries are built on a Windows runner with locked dependencies,
  incremental compilation disabled, source paths remapped, and the commit time
  used as the archive timestamp.
- An explicit allowlist is packaged. Every payload file is recorded with its
  size and SHA-256 digest in the machine-readable manifest.
- A separate clean Windows runner verifies the archive checksum, source
  revision, manifest, full file inventory, public-artifact scan, agent readiness
  command, and desktop plan command.
- GitHub creates signed SLSA provenance for the archive, checksum, manifest, and
  independent verification report using a short-lived Sigstore certificate.

## Live integration boundary

The release workflow does **not** run OBS Studio, MediaMTX, Moblin, IRL Pro,
Larix, BELABOX, SRTLA tooling, a relay host, or a production network. The package
manifest and verification report keep each of those integrations marked
`not-run`. Consult the versioned compatibility matrix for separately reviewed
integration and field evidence.

The portable binaries do not carry an Authenticode publisher signature in this
alpha workflow. The GitHub artifact attestation signs provenance for the exact
downloaded files; it is not a substitute for Windows publisher identity or
SmartScreen reputation.

## Verify the download

```powershell
Get-FileHash -Algorithm SHA256 .\openirl-windows-portable-alpha.zip
Get-Content .\openirl-windows-portable-alpha.zip.sha256
gh attestation verify .\openirl-windows-portable-alpha.zip `
  --repo halveknots-llc/openirl
```

For full package verification, check out the `source_revision` from the attached
manifest and run `scripts/windows/verify-alpha-portable.ps1` against the four
downloaded release evidence files.
