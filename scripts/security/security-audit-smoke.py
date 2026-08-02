#!/usr/bin/env python3
"""Executable security smoke checks for the OpenIRL local agent."""
from __future__ import annotations

import json
import http.client
import shutil
import socket
import subprocess
import sys
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CONFIG = ROOT / "config/openirl.example.toml"
CARGO = ["cargo", "run", "--quiet", "--package", "openirl-agent", "--"]
CANARIES = [
    "super-dashboard-token",
    "obs-password-canary",
    "srt-passphrase-canary",
    "Bearer field-token-canary",
    "10.23.45.67",
]


def run_agent(args: list[str], *, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [*CARGO, *args],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=check,
    )


def parse_json(stdout: str) -> dict:
    return json.loads(stdout.strip())


def assert_default_config_validates() -> None:
    result = run_agent(["check-config", "--config", str(CONFIG)])
    payload = parse_json(result.stdout)
    validation = payload["validation"]
    if not validation["ok"]:
        raise AssertionError(f"default config validation failed: {validation}")
    if payload["config"]["api"]["bind"] != "127.0.0.1:7707":
        raise AssertionError("default API bind is not localhost")


def assert_public_bind_override_is_blocked() -> None:
    result = run_agent(
        ["serve", "--config", str(CONFIG), "--bind", "0.0.0.0:0"],
        check=False,
    )
    combined = result.stdout + result.stderr
    if result.returncode == 0:
        raise AssertionError("unsafe public bind override started successfully")
    if "refusing to start with unsafe config" not in combined:
        raise AssertionError(f"unsafe public bind failed for the wrong reason: {combined}")


def request_status(port: int, host: str, origin: str | None = None) -> int:
    connection = http.client.HTTPConnection("127.0.0.1", port, timeout=2)
    try:
        connection.putrequest("POST", "/api/metrics/simulate/healthy", skip_host=True)
        connection.putheader("Host", host)
        if origin is not None:
            connection.putheader("Origin", origin)
        connection.putheader("Content-Length", "0")
        connection.endheaders()
        response = connection.getresponse()
        response.read()
        return response.status
    finally:
        connection.close()


def assert_tokenless_localhost_rejects_untrusted_authority() -> None:
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", 0))
        port = probe.getsockname()[1]

    process = subprocess.Popen(
        [
            *CARGO,
            "serve",
            "--config",
            str(CONFIG),
            "--bind",
            f"127.0.0.1:{port}",
        ],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    try:
        deadline = time.monotonic() + 45
        while time.monotonic() < deadline:
            if process.poll() is not None:
                stdout, stderr = process.communicate()
                raise AssertionError(f"local agent exited before security probe: {stdout}{stderr}")
            try:
                connection = http.client.HTTPConnection("127.0.0.1", port, timeout=1)
                connection.request("GET", "/health")
                response = connection.getresponse()
                response.read()
                connection.close()
                if response.status == 200:
                    break
            except OSError:
                time.sleep(0.1)
        else:
            raise AssertionError("local agent did not become ready for security probe")

        untrusted = request_status(
            port,
            f"untrusted.example:{port}",
            f"http://untrusted.example:{port}",
        )
        if untrusted not in (401, 403):
            raise AssertionError(
                f"untrusted local authority bypassed token enforcement with HTTP {untrusted}"
            )

        trusted = request_status(
            port,
            f"127.0.0.1:{port}",
            f"http://127.0.0.1:{port}",
        )
        if trusted != 200:
            raise AssertionError(f"trusted loopback demo request returned HTTP {trusted}")
    finally:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)


def support_bundle_config(tmp: Path) -> Path:
    source = CONFIG.read_text(encoding="utf-8")
    bundle_dir = (tmp / "support-bundles").as_posix()
    field_dir = (tmp / "field-reports").as_posix()
    source = source.replace(
        'support_bundles_dir = "artifacts/support-bundles"',
        f'support_bundles_dir = "{bundle_dir}"',
    )
    source = source.replace(
        'field_reports_dir = "artifacts/field-reports"',
        f'field_reports_dir = "{field_dir}"',
    )
    path = tmp / "openirl.security-smoke.toml"
    path.write_text(source, encoding="utf-8")
    return path


def assert_support_bundle_redacts_canaries() -> None:
    with tempfile.TemporaryDirectory(prefix="openirl-security-smoke-") as raw_tmp:
        tmp = Path(raw_tmp)
        config = support_bundle_config(tmp)
        field_report = tmp / "field-report.md"
        field_report.write_text(
            "\n".join(
                [
                    "# Field Report",
                    "dashboard_token: super-dashboard-token",
                    "OBS password = obs-password-canary",
                    "srt://relay.example:9000?streamid=main&passphrase=srt-passphrase-canary",
                    "Authorization: Bearer field-token-canary",
                    "relay_host=10.23.45.67",
                ]
            ),
            encoding="utf-8",
        )
        result = run_agent(
            [
                "artifacts",
                "support-bundle",
                "--config",
                str(config),
                "--field-report",
                str(field_report),
            ]
        )
        export = parse_json(result.stdout)
        root = Path(export["root_dir"])
        payload = (root / "support-bundle.json").read_text(encoding="utf-8")
        report = (root / "field-report.md").read_text(encoding="utf-8")
        combined = f"{payload}\n{report}"
        leaked = [canary for canary in CANARIES if canary in combined]
        if leaked:
            raise AssertionError(f"support bundle leaked canaries: {leaked}")
        if "<redacted>" not in combined or "<redacted-ip>" not in combined:
            raise AssertionError("support bundle did not include expected redaction markers")


def main() -> int:
    if shutil.which("cargo") is None:
        print("cargo is unavailable", file=sys.stderr)
        return 1

    assert_default_config_validates()
    assert_public_bind_override_is_blocked()
    assert_tokenless_localhost_rejects_untrusted_authority()
    assert_support_bundle_redacts_canaries()
    print("security audit smoke passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
