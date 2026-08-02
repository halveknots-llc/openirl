#!/usr/bin/env python3
"""Bounded public-release archive and manifest secret scan."""
from __future__ import annotations

import argparse
import json
import re
import stat
import tempfile
import zipfile
from pathlib import Path, PurePosixPath

MAX_FILES = 2_048
MAX_FILE_BYTES = 256 * 1024 * 1024
MAX_TOTAL_BYTES = 1024 * 1024 * 1024
MAX_MANIFEST_BYTES = 4 * 1024 * 1024
MAX_COMPRESSION_RATIO = 64
CHUNK_BYTES = 1024 * 1024
OVERLAP_BYTES = 512

DENIED_PARTS = {
    ".git",
    ".idea",
    ".vscode",
    "target",
    "openirl-support-bundles",
    "support-bundles",
    "__pycache__",
}
DENIED_SUFFIXES = {".key", ".pem", ".p12", ".pfx", ".kdbx"}
SENSITIVE_PATTERNS = (
    ("private key", re.compile(rb"-----BEGIN [A-Z ]*PRIVATE KEY-----")),
    ("AWS access key", re.compile(rb"(?:AKIA|ASIA)[0-9A-Z]{16}")),
    ("GitHub token", re.compile(rb"(?:gh[pousr]_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,})")),
    ("OpenAI key", re.compile(rb"sk-(?:proj-)?[A-Za-z0-9_-]{20,}")),
    ("Slack token", re.compile(rb"xox[baprs]-[A-Za-z0-9-]{20,}")),
    ("Google API key", re.compile(rb"AIza[0-9A-Za-z_-]{30,}")),
    ("bearer credential", re.compile(rb"Bearer\s+[A-Za-z0-9._~+/=-]{20,}", re.IGNORECASE)),
    (
        "credential-bearing URL",
        re.compile(rb"[a-z][a-z0-9+.-]*://[^\s/:@]+:[^@\s/]+@", re.IGNORECASE),
    ),
    (
        "assigned credential",
        re.compile(
            rb"\b(?:password|passphrase|stream[_-]?key|dashboard[_-]?token|obs[_-]?password|private[_-]?key)\b\s*[:=]\s*[\"']?[^\s\"'<>]{12,}",
            re.IGNORECASE,
        ),
    ),
)


class ArtifactScanError(ValueError):
    """Public artifact failed a bounded safety check."""


def validate_member_path(name: str) -> PurePosixPath:
    if (
        not name
        or "\\" in name
        or name.startswith("/")
        or any(ord(character) < 32 for character in name)
    ):
        raise ArtifactScanError("archive member uses a non-canonical path")
    raw_parts = name.split("/")
    if any(part in ("", ".", "..") or ":" in part for part in raw_parts):
        raise ArtifactScanError("archive member uses an unsafe platform path")
    path = PurePosixPath(name)
    lowered = [part.lower() for part in path.parts]
    if any(part in DENIED_PARTS for part in lowered):
        raise ArtifactScanError("archive contains a denied private or generated path")
    if any(part.startswith("._") for part in path.parts):
        raise ArtifactScanError("archive contains AppleDouble metadata")
    if any(part == ".env" or part.startswith(".env.") for part in lowered):
        raise ArtifactScanError("archive contains an environment file")
    if path.suffix.lower() in DENIED_SUFFIXES:
        raise ArtifactScanError("archive contains a credential-container file type")
    return path


def encoded_markers(values: list[str]) -> list[bytes]:
    markers: list[bytes] = []
    for value in values:
        if not value:
            continue
        for variant in {value, value.replace("\\", "/"), value.replace("/", "\\")}:
            markers.append(variant.encode("utf-8"))
            markers.append(variant.encode("utf-16-le"))
    return markers


def scan_stream(stream, forbidden: list[bytes]) -> None:
    previous = b""
    while True:
        chunk = stream.read(CHUNK_BYTES)
        if not chunk:
            break
        window = previous + chunk
        lowered = window.lower()
        for marker in forbidden:
            if marker.lower() in lowered:
                raise ArtifactScanError("artifact contains a forbidden local build path")
        for label, pattern in SENSITIVE_PATTERNS:
            if pattern.search(window):
                raise ArtifactScanError(f"artifact contains a high-confidence {label} pattern")
        previous = window[-OVERLAP_BYTES:]


def scan_manifest(path: Path, forbidden: list[bytes]) -> dict:
    if path.stat().st_size > MAX_MANIFEST_BYTES:
        raise ArtifactScanError("manifest exceeds the public release size limit")
    with path.open("rb") as stream:
        scan_stream(stream, forbidden)
    payload = json.loads(path.read_text(encoding="utf-8"))
    revision = payload.get("source_revision", "")
    if not isinstance(revision, str) or re.fullmatch(r"[0-9a-f]{40}", revision) is None:
        raise ArtifactScanError("manifest source_revision is not a full lowercase Git commit")
    if payload.get("package") != "openirl-windows-portable-alpha":
        raise ArtifactScanError("manifest package identity is invalid")
    return payload


def scan_archive(path: Path, forbidden: list[bytes]) -> tuple[int, int]:
    total = 0
    seen: set[str] = set()
    with zipfile.ZipFile(path) as archive:
        members = archive.infolist()
        if not members or len(members) > MAX_FILES:
            raise ArtifactScanError("archive member count is outside the public release limit")
        for info in members:
            member_path = validate_member_path(info.filename)
            canonical = member_path.as_posix().casefold()
            if canonical in seen:
                raise ArtifactScanError("archive contains a duplicate platform path")
            seen.add(canonical)
            if info.flag_bits & 0x1:
                raise ArtifactScanError("archive contains an encrypted member")
            unix_mode = (info.external_attr >> 16) & 0xFFFF
            if unix_mode and stat.S_ISLNK(unix_mode):
                raise ArtifactScanError("archive contains a symbolic link")
            if info.file_size > MAX_FILE_BYTES:
                raise ArtifactScanError("archive member exceeds the public release size limit")
            if (
                info.file_size > CHUNK_BYTES
                and info.compress_size > 0
                and info.file_size / info.compress_size > MAX_COMPRESSION_RATIO
            ):
                raise ArtifactScanError("archive member exceeds the compression ratio limit")
            total += info.file_size
            if total > MAX_TOTAL_BYTES:
                raise ArtifactScanError("archive exceeds the total public release size limit")
            if not info.is_dir():
                with archive.open(info) as stream:
                    scan_stream(stream, forbidden)
    return len(members), total


def self_test() -> None:
    with tempfile.TemporaryDirectory(prefix="openirl-artifact-scan-") as raw_tmp:
        root = Path(raw_tmp)
        manifest = root / "manifest.json"
        manifest.write_text(
            json.dumps(
                {
                    "package": "openirl-windows-portable-alpha",
                    "source_revision": "a" * 40,
                }
            ),
            encoding="utf-8",
        )
        safe = root / "safe.zip"
        with zipfile.ZipFile(safe, "w") as archive:
            archive.writestr("OpenIRL/README.md", "synthetic public artifact")
        scan_manifest(manifest, [])
        scan_archive(safe, [])

        unsafe = root / "unsafe.zip"
        with zipfile.ZipFile(unsafe, "w") as archive:
            archive.writestr("OpenIRL/.env", "synthetic unsafe path")
        try:
            scan_archive(unsafe, [])
        except ArtifactScanError:
            pass
        else:
            raise ArtifactScanError("self-test accepted an unsafe archive path")

        credential = root / "credential.zip"
        with zipfile.ZipFile(credential, "w") as archive:
            archive.writestr(
                "OpenIRL/review.txt",
                "-----BEGIN " + "PRIVATE KEY-----",
            )
        try:
            scan_archive(credential, [])
        except ArtifactScanError:
            return
        raise ArtifactScanError("self-test accepted a credential pattern")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path)
    parser.add_argument("--manifest", type=Path)
    parser.add_argument("--forbid-local-root", action="append", default=[])
    parser.add_argument("--self-test", action="store_true")
    args = parser.parse_args()

    if args.self_test:
        self_test()
        print("release artifact scan self-test passed")
        return 0
    if args.archive is None or args.manifest is None:
        parser.error("--archive and --manifest are required unless --self-test is used")

    forbidden = encoded_markers(args.forbid_local_root)
    scan_manifest(args.manifest, forbidden)
    members, total = scan_archive(args.archive, forbidden)
    print(f"release artifact scan passed: {members} members, {total} uncompressed bytes")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
