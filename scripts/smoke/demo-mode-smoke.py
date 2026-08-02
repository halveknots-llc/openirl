#!/usr/bin/env python3
"""Exercise deterministic first-run demo and scoped readiness behavior."""
from __future__ import annotations

import http.client
import json
import socket
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CARGO = ["cargo", "run", "--quiet", "--package", "openirl-agent", "--"]


def parse_cli_readiness() -> None:
    result = subprocess.run(
        [*CARGO, "readiness"],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=True,
    )
    report = json.loads(result.stdout)
    if report["mode"] != "standard":
        raise AssertionError("CLI readiness report did not use standard mode")
    if report["summary"]["source"]["satisfied"] != 0:
        raise AssertionError("CLI readiness report inferred source evidence")
    if report["summary"]["live_environment"]["satisfied"] != 0:
        raise AssertionError("CLI readiness report inferred live evidence")


def get(port: int, path: str) -> tuple[int, bytes]:
    connection = http.client.HTTPConnection("127.0.0.1", port, timeout=2)
    try:
        connection.request("GET", path)
        response = connection.getresponse()
        return response.status, response.read()
    finally:
        connection.close()


def wait_for_demo(process: subprocess.Popen[str], port: int) -> None:
    deadline = time.monotonic() + 45
    while time.monotonic() < deadline:
        if process.poll() is not None:
            stdout, stderr = process.communicate()
            raise AssertionError(f"demo exited before readiness check: {stdout}{stderr}")
        try:
            status, _body = get(port, "/health")
            if status == 200:
                return
        except OSError:
            time.sleep(0.1)
    raise AssertionError("demo did not become ready")


def assert_demo_server() -> None:
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", 0))
        port = probe.getsockname()[1]

    process = subprocess.Popen(
        [*CARGO, "demo", "--bind", f"127.0.0.1:{port}"],
        cwd=ROOT,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    try:
        wait_for_demo(process, port)
        status, body = get(port, "/api/readiness")
        if status != 200:
            raise AssertionError(f"demo readiness returned HTTP {status}")
        report = json.loads(body)
        demo = report.get("demo")
        if report.get("mode") != "demo" or not demo:
            raise AssertionError("demo readiness payload is missing demo evidence")
        if not demo.get("deterministic") or len(demo.get("steps", [])) != 5:
            raise AssertionError("demo evidence is not the versioned deterministic sequence")
        if demo.get("outbound_network_requests_made"):
            raise AssertionError("demo report claims an outbound network request")
        if demo.get("external_processes_started"):
            raise AssertionError("demo report claims an external process start")
        if demo.get("credentials_required"):
            raise AssertionError("demo report claims a credential requirement")
        serialized = json.dumps(report)
        if "synthetic-demo-passphrase" in serialized:
            raise AssertionError("demo readiness exposed the synthetic passphrase canary")
        if report["summary"]["live_environment"]["satisfied"] != 0:
            raise AssertionError("demo readiness inferred live evidence")

        status, body = get(port, "/api/state")
        state = json.loads(body)
        if status != 200 or state.get("mode") != "demo":
            raise AssertionError("demo API state did not identify demo mode")
        if state["session"]["sample_count"] != 5:
            raise AssertionError("demo API state did not contain the fixed sample sequence")

        status, body = get(port, "/")
        if status != 200 or b"OpenIRL Agent" not in body:
            raise AssertionError("demo dashboard did not load")
    finally:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=5)


def main() -> int:
    parse_cli_readiness()
    assert_demo_server()
    print("deterministic demo mode smoke passed")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:
        print(f"demo mode smoke failed: {exc}", file=sys.stderr)
        raise SystemExit(1)
