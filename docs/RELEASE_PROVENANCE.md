# Release Provenance

OpenIRL's Windows portable workflow creates a clean, source-pinned package and
has a second Windows runner verify it before provenance or release publication.
This document defines what that pipeline proves and what remains outside its
claim boundary.

## Artifact set

The `windows-package` workflow produces four public evidence files:

| File | Purpose |
| --- | --- |
| `openirl-windows-portable-alpha.zip` | Portable Windows package rooted at `OpenIRL/`. |
| `openirl-windows-portable-alpha.zip.sha256` | SHA-256 digest in standard checksum-file form. |
| `openirl-windows-portable-alpha.manifest.json` | Source revision, build inputs, evidence limits, and per-file size/digest inventory. |
| `openirl-windows-portable-alpha.verification.json` | Independent clean-runner verification result. |

The package manifest excludes its own digest to avoid a circular value. The
independent report records the manifest digest, and signed provenance covers all
four final evidence files.

## Build controls

`scripts/windows/build-alpha-portable.ps1`:

1. Requires a clean worktree, verifies the Git object database, and optionally
   requires an exact workflow revision.
2. Runs `cargo xtask ci` and confirms validation did not dirty the checkout.
3. Rejects caller-supplied compiler flags, disables incremental compilation,
   uses locked dependencies, remaps the local source path, and takes archive
   timestamps from the source commit.
4. Copies only an explicit set of tracked config, dashboard, documentation, and
   smoke files plus the two newly built executables.
5. Records each payload path, size, and SHA-256 digest in a machine-readable
   manifest, then builds the zip in sorted order with a fixed timestamp.
6. Runs the bounded public-artifact scan before any workflow upload.

These controls make the build repeatable and its inputs inspectable. They do not
promise byte-for-byte equality across different Windows runner images, Rust
toolchains, or linker versions. The manifest records those build inputs so a
digest difference can be investigated rather than ignored.

## Independent verification

The build and verification jobs never share a workspace. The verification job
downloads the immutable workflow artifact on a new Windows runner and runs
`scripts/windows/verify-alpha-portable.ps1`. It checks:

- archive SHA-256 and source revision
- embedded versus external manifest identity
- exact payload inventory, file sizes, and file hashes
- Windows target and mandatory source gate
- explicit `not-run` status for live dependencies
- bounded path and credential scan
- packaged `openirl-agent.exe readiness` output
- packaged `openirl-desktop.exe plan` output

The executable checks establish Windows CLI launch behavior. They do not launch
OBS, MediaMTX, a mobile encoder, BELABOX, SRTLA, a relay host, or contribution
media.

After the independent report is written, the verifier scans the archive,
manifest, checksum, and report together. The release job repeats that bounded
scan and includes the release notes before any file is published.

## Signed provenance

After verification, trusted pushes to `main` and version tags use
`actions/attest-build-provenance` at a reviewed immutable commit. GitHub obtains
a short-lived Sigstore certificate through OIDC, emits an in-toto SLSA
provenance statement, and associates it with this public repository. No signing
key or long-lived credential is stored in the repository. Pull requests and
manual workflow dispatches can build and verify artifacts, but they cannot
request provenance credentials or publish a release.

For a tagged release, authenticate the release first, then verify each downloaded
file against the expected signer workflow and source tag:

```powershell
$repo = 'halveknots-llc/openirl'
$tag = 'v0.1.0-alpha.0'
$workflow = "$repo/.github/workflows/windows-package.yml"

gh release verify $tag --repo $repo
if ($LASTEXITCODE -ne 0) { throw 'Release authentication failed' }

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
```

The attestation signs artifact provenance. It is not an Authenticode publisher
signature and does not establish Windows publisher identity or SmartScreen
reputation. Portable alpha release notes must keep that distinction explicit.

## Release policy

Pull requests build and independently verify the package but cannot request an
OIDC signing certificate. The repository protects `v*` tags against update and
deletion and enables immutable releases. A tag matching
`v<workspace-version>` may publish a prerelease only when the tag commit is
reachable from `origin/main`, package verification passes, and provenance
succeeds. Immediately before publication, the release job fetches the tag again
and requires its peeled commit to equal the exact workflow revision used to
build, verify, and attest the package. It creates a draft with the complete file
set and publishes that draft only after every upload succeeds.

Release notes name every live integration exercised by the workflow. The
current automated release path marks OBS, MediaMTX, mobile encoders, BELABOX,
SRTLA, relays, and production networks `not-run`; separately reviewed results
belong in the [compatibility matrix](COMPATIBILITY.md).

## Local verification

On Windows, check out the manifest's exact `source_revision`, place the three
package evidence files in `dist/windows-alpha`, then run:

```powershell
$revision = (Get-Content -Raw .\dist\windows-alpha\openirl-windows-portable-alpha.manifest.json | ConvertFrom-Json).source_revision
.\scripts\windows\verify-alpha-portable.ps1 `
  -ArtifactDir dist\windows-alpha `
  -ExpectedRevision $revision
```

The verifier writes the fourth file, the independent verification report. Keep
all credentials and raw production evidence outside the checkout and artifact
directory.
