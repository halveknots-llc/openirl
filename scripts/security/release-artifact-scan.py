#!/usr/bin/env python3
"""Bounded public-release archive and manifest secret scan."""
from __future__ import annotations

import argparse
import json
import re
import stat
import struct
import tempfile
import zipfile
from pathlib import Path, PurePosixPath

MAX_FILES = 2_048
MAX_FILE_BYTES = 256 * 1024 * 1024
MAX_TOTAL_BYTES = 1024 * 1024 * 1024
MAX_MANIFEST_BYTES = 4 * 1024 * 1024
MAX_ARCHIVE_BYTES = MAX_TOTAL_BYTES + (16 * 1024 * 1024)
MAX_COMPRESSION_RATIO = 64
CHUNK_BYTES = 1024 * 1024
OVERLAP_BYTES = 512
ZIP_EOCD_BYTES = 22
MAX_ZIP_COMMENT_BYTES = 65_535
ZIP_LOCAL_HEADER = struct.Struct("<4s5H3L2H")
ZIP_LOCAL_SIGNATURE = b"PK\x03\x04"

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
SENSITIVE_NAME = (
    rb"(?:access[_-]?token|api[_-]?key|auth(?:entication)?[_-]?token|"
    rb"authorization(?:[_-]?header)?|dashboard[_-]?token|obs[_-]?password|"
    rb"password|passphrase|private[_-]?key|secret|signature|srt[_-]?passphrase|"
    rb"stream[_-]?key|token)"
)
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
            rb"(?<![A-Za-z0-9])(?:[\"']\s*)?"
            + SENSITIVE_NAME
            + rb"(?:\s*[\"'])?\s*[:=]\s*"
            + rb"(?:"
            + rb"(?P<quote>[\"'])(?P<quoted>[^\s\"'<>]{12,})(?P=quote)"
            + rb"|"
            + rb"(?=[A-Za-z0-9._~+/%=-]{12,}(?:$|[\s,;})\]]))"
            + rb"(?=[A-Za-z0-9._~+/%=-]*[0-9])"
            + rb"[A-Za-z0-9._~+/%=-]{12,}(?=$|[\s,;})\]])"
            + rb")",
            re.IGNORECASE,
        ),
    ),
    (
        "credential query or fragment",
        re.compile(
            rb"[?&#]" + SENSITIVE_NAME + rb"=[^\s&#\"'<>]{12,}",
            re.IGNORECASE,
        ),
    ),
    (
        "credential command option",
        re.compile(
            rb"(?<![A-Za-z0-9])(?:--|/)"
            + SENSITIVE_NAME
            + rb"(?:\s+|[:=])(?:Bearer\s+)?[^\s\"'<>]{12,}",
            re.IGNORECASE,
        ),
    ),
    (
        "RTMP path credential",
        re.compile(
            rb"\brtmps?://[^\s/?#]+(?:/[^\s/?#]+)*/[A-Za-z0-9._~+%=-]{12,}(?=$|[\s?#\"'<>)])",
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


def scan_bytes(payload: bytes, forbidden: list[bytes]) -> None:
    lowered = payload.lower()
    for marker in forbidden:
        if marker.lower() in lowered:
            raise ArtifactScanError("artifact contains a forbidden local build path")
    views = [payload]
    null_count = payload.count(b"\0")
    if null_count >= 2 and null_count * 8 >= len(payload):
        views.append(payload.replace(b"\0", b""))
    for view in views:
        for label, pattern in SENSITIVE_PATTERNS:
            if pattern.search(view):
                raise ArtifactScanError(f"artifact contains a high-confidence {label} pattern")


def scan_stream(stream, forbidden: list[bytes]) -> None:
    previous = b""
    while True:
        chunk = stream.read(CHUNK_BYTES)
        if not chunk:
            break
        window = previous + chunk
        scan_bytes(window, forbidden)
        previous = window[-OVERLAP_BYTES:]


def scan_manifest(path: Path, forbidden: list[bytes]) -> dict:
    if path.is_symlink() or not path.is_file():
        raise ArtifactScanError("manifest must be a regular file")
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


def scan_public_file(path: Path, forbidden: list[bytes]) -> None:
    if path.is_symlink() or not path.is_file():
        raise ArtifactScanError("additional public evidence must be a regular file")
    if path.stat().st_size > MAX_MANIFEST_BYTES:
        raise ArtifactScanError("additional public evidence exceeds the release size limit")
    with path.open("rb") as stream:
        scan_stream(stream, forbidden)


def inspect_zip_envelope(path: Path) -> tuple[int, bytes]:
    if path.is_symlink() or not path.is_file():
        raise ArtifactScanError("archive must be a regular file")
    size = path.stat().st_size
    if size < ZIP_EOCD_BYTES or size > MAX_ARCHIVE_BYTES:
        raise ArtifactScanError("archive size is outside the public release limit")

    tail_size = min(size, ZIP_EOCD_BYTES + MAX_ZIP_COMMENT_BYTES)
    with path.open("rb") as stream:
        stream.seek(size - tail_size)
        tail = stream.read(tail_size)
    signature = b"PK\x05\x06"
    for offset in range(len(tail) - ZIP_EOCD_BYTES, -1, -1):
        if tail[offset : offset + 4] != signature:
            continue
        fields = struct.unpack_from("<4s4H2LH", tail, offset)
        disk_number, central_disk, disk_entries, total_entries = fields[1:5]
        comment_size = fields[-1]
        if offset + ZIP_EOCD_BYTES + comment_size != len(tail):
            continue
        if disk_number != 0 or central_disk != 0 or disk_entries != total_entries:
            raise ArtifactScanError("multi-disk archives are not supported")
        if total_entries == 0 or total_entries > MAX_FILES:
            raise ArtifactScanError("archive member count is outside the public release limit")
        return total_entries, tail[offset + ZIP_EOCD_BYTES :]
    raise ArtifactScanError("archive has an invalid envelope or trailing data")


def read_local_metadata(stream, offset: int) -> tuple[bytes, bytes]:
    if offset < 0:
        raise ArtifactScanError("archive member has an invalid local header offset")
    stream.seek(offset)
    header = stream.read(ZIP_LOCAL_HEADER.size)
    if len(header) != ZIP_LOCAL_HEADER.size:
        raise ArtifactScanError("archive member has a truncated local header")
    fields = ZIP_LOCAL_HEADER.unpack(header)
    if fields[0] != ZIP_LOCAL_SIGNATURE:
        raise ArtifactScanError("archive member has an invalid local header")
    name_size, extra_size = fields[-2:]
    name = stream.read(name_size)
    extra = stream.read(extra_size)
    if len(name) != name_size or len(extra) != extra_size:
        raise ArtifactScanError("archive member has truncated local metadata")
    return name, extra


def scan_archive(path: Path, forbidden: list[bytes]) -> tuple[int, int]:
    declared_members, archive_comment = inspect_zip_envelope(path)
    scan_bytes(archive_comment, forbidden)

    total = 0
    seen: set[str] = set()
    with path.open("rb") as raw_archive, zipfile.ZipFile(path) as archive:
        members = archive.infolist()
        if len(members) != declared_members:
            raise ArtifactScanError("archive member count is outside the public release limit")
        for info in members:
            member_path = validate_member_path(info.filename)
            scan_bytes(info.filename.encode("utf-8"), forbidden)
            scan_bytes(info.comment, forbidden)
            scan_bytes(info.extra, forbidden)
            local_name, local_extra = read_local_metadata(raw_archive, info.header_offset)
            scan_bytes(local_name, forbidden)
            scan_bytes(local_extra, forbidden)
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
            with archive.open(info) as stream:
                if not info.is_dir():
                    scan_stream(stream, forbidden)
    return len(members), total


def self_test() -> None:
    def require_archive_rejection(root: Path, name: str, member: str, payload: bytes) -> None:
        archive_path = root / f"{name}.zip"
        with zipfile.ZipFile(archive_path, "w") as archive:
            archive.writestr(member, payload)
        try:
            scan_archive(archive_path, [])
        except ArtifactScanError:
            return
        raise ArtifactScanError(f"self-test accepted unsafe {name} evidence")

    def require_public_file_rejection(root: Path, name: str, payload: bytes) -> None:
        path = root / name
        path.write_bytes(payload)
        try:
            scan_public_file(path, [])
        except ArtifactScanError:
            return
        raise ArtifactScanError(f"self-test accepted unsafe {name} evidence")

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

        container_boundary = root / "container-boundary.zip"
        info = zipfile.ZipInfo("OpenIRL/https")
        info.compress_type = zipfile.ZIP_STORED
        with zipfile.ZipFile(container_boundary, "w") as archive:
            archive.writestr(info, b"://operator:synthetic-value@relay.invalid")
        scan_archive(container_boundary, [])

        local_extra = root / "local-extra.zip"
        synthetic_canary = b"synthetic-release-canary-12345"
        unsafe_extra_value = b"dashboard_token=" + synthetic_canary
        safe_extra = struct.pack("<HH", 0xCAFE, len(unsafe_extra_value)) + (
            b"x" * len(unsafe_extra_value)
        )
        unsafe_extra = struct.pack("<HH", 0xCAFE, len(unsafe_extra_value)) + unsafe_extra_value
        info = zipfile.ZipInfo("OpenIRL/README.md")
        info.extra = safe_extra
        with zipfile.ZipFile(local_extra, "w") as archive:
            archive.writestr(info, "synthetic public artifact")
        with local_extra.open("r+b") as stream:
            header = ZIP_LOCAL_HEADER.unpack(stream.read(ZIP_LOCAL_HEADER.size))
            stream.seek(ZIP_LOCAL_HEADER.size + header[-2])
            stream.write(unsafe_extra)
        try:
            scan_archive(local_extra, [])
        except ArtifactScanError:
            pass
        else:
            raise ArtifactScanError("self-test accepted unsafe local extra metadata")

        private_key = ("-----BEGIN " + "PRIVATE KEY-----").encode("ascii")
        unsafe_payloads = {
            "private-key": private_key,
            "fragment-credential": b"https://relay.invalid/live#access_token="
            + b"synthetic-release-canary-12345",
            "camel-query-credential": b"https://relay.invalid/api?apiKey="
            + b"synthetic-release-canary-12345",
            "command-credential": b"--authorization-header "
            + b"synthetic-release-canary-12345",
            "rtmp-path-credential": b"rtmp://relay.invalid/live/"
            + b"synthetic-release-canary-12345",
        }
        for name, payload in unsafe_payloads.items():
            require_archive_rejection(root, name, "OpenIRL/review.txt", payload)

        require_public_file_rejection(
            root,
            "quoted-structured-key.json",
            b'{"dashboard_token": "' + synthetic_canary + b'"}',
        )
        require_public_file_rejection(
            root,
            "utf16-assignment.txt",
            ("dashboard_token=" + synthetic_canary.decode()).encode("utf-16-le"),
        )

        archive_comment = root / "archive-comment.zip"
        with zipfile.ZipFile(archive_comment, "w") as archive:
            archive.writestr("OpenIRL/README.md", "synthetic public artifact")
            archive.comment = b"dashboard_token=" + synthetic_canary
        try:
            scan_archive(archive_comment, [])
        except ArtifactScanError:
            pass
        else:
            raise ArtifactScanError("self-test accepted unsafe archive comment evidence")

        member_comment = root / "member-comment.zip"
        info = zipfile.ZipInfo("OpenIRL/README.md")
        info.comment = b"dashboard_token=" + synthetic_canary
        with zipfile.ZipFile(member_comment, "w") as archive:
            archive.writestr(info, "synthetic public artifact")
        try:
            scan_archive(member_comment, [])
        except ArtifactScanError:
            pass
        else:
            raise ArtifactScanError("self-test accepted unsafe member comment evidence")

        member_extra = root / "member-extra.zip"
        info = zipfile.ZipInfo("OpenIRL/README.md")
        extra_value = b"dashboard_token=" + synthetic_canary
        info.extra = struct.pack("<HH", 0xCAFE, len(extra_value)) + extra_value
        with zipfile.ZipFile(member_extra, "w") as archive:
            archive.writestr(info, "synthetic public artifact")
        try:
            scan_archive(member_extra, [])
        except ArtifactScanError:
            pass
        else:
            raise ArtifactScanError("self-test accepted unsafe member extra evidence")

        trailing_data = root / "trailing-data.zip"
        with zipfile.ZipFile(trailing_data, "w") as archive:
            archive.writestr("OpenIRL/README.md", "synthetic public artifact")
        with trailing_data.open("ab") as stream:
            stream.write(b"synthetic trailing bytes")
        try:
            scan_archive(trailing_data, [])
        except ArtifactScanError:
            pass
        else:
            raise ArtifactScanError("self-test accepted trailing archive data")

        require_archive_rejection(root, "unsafe-path", "OpenIRL/.env", b"unsafe")
        require_archive_rejection(
            root,
            "credential-name",
            "OpenIRL/accessToken=synthetic-release-canary-12345.txt",
            b"unsafe",
        )

        local_root = str(root / "private-checkout")
        local_archive = root / "local-root.zip"
        with zipfile.ZipFile(local_archive, "w") as archive:
            archive.writestr("OpenIRL/review.txt", local_root)
        try:
            scan_archive(local_archive, encoded_markers([local_root]))
        except ArtifactScanError:
            pass
        else:
            raise ArtifactScanError("self-test accepted a forbidden local build path")

        safe_controls = root / "safe-controls.zip"
        with zipfile.ZipFile(safe_controls, "w") as archive:
            archive.writestr(
                "OpenIRL/review.txt",
                "https://relay.invalid/api?view=summary#operations "
                "rtmp://relay.invalid/live --authorization-header=<redacted> "
                "authToken = tokenInput.value.trim(); "
                "$Password = $env:OPENIRL_OBS_PASSWORD, "
                "$secret = [Convert]::ToBase64String($secretBytes)",
            )
        scan_archive(safe_controls, [])

        unsafe_metadata = root / "unsafe-metadata.json"
        unsafe_metadata.write_bytes(
            b'{"relay":"https://relay.invalid/api?apiKey='
            + b'synthetic-release-canary-12345"}'
        )
        try:
            scan_public_file(unsafe_metadata, [])
        except ArtifactScanError:
            pass
        else:
            raise ArtifactScanError("self-test accepted unsafe public metadata")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path)
    parser.add_argument("--manifest", type=Path)
    parser.add_argument("--scan-file", type=Path, action="append", default=[])
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
    for path in args.scan_file:
        scan_public_file(path, forbidden)
    print(
        "release artifact scan passed: "
        f"{members} members, {total} uncompressed bytes, "
        f"{len(args.scan_file)} additional files"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
