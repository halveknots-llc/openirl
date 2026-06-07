#!/usr/bin/env python3
"""Minimal OpenIRL feature areas dry-run API smoke test."""
from __future__ import annotations

import json
import os
import sys
from urllib import request, error

BASE = "http://127.0.0.1:7707"
SCHEMA_REVISION = 38


def call(method: str, path: str, payload: object | None = None) -> object:
    data = None
    headers = {}
    token = os.environ.get("OPENIRL_DASHBOARD_TOKEN", "").strip()
    if token:
        headers["authorization"] = f"Bearer {token}"
    if payload is not None:
        data = json.dumps(payload).encode("utf-8")
        headers["content-type"] = "application/json"
    req = request.Request(BASE + path, data=data, headers=headers, method=method)
    try:
        with request.urlopen(req, timeout=5) as response:
            raw = response.read().decode("utf-8")
    except error.URLError as exc:
        raise RuntimeError(f"{method} {path} failed: {exc}") from exc
    return json.loads(raw)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise RuntimeError(message)


def main() -> int:
    health = call("GET", "/health")
    require(health.get("status") == "ok", "health status was not ok")
    require(health.get("schema_revision") == SCHEMA_REVISION, "agent schema revision is not current")

    validation = call("GET", "/api/config/validation")
    require(validation.get("error_count") == 0, "default config has validation errors")

    quickstart = call("POST", "/api/onboarding/quickstart")
    require("profile" in quickstart, "quickstart missing profile")
    require("qr" in quickstart, "quickstart missing qr field")

    brownout = call("POST", "/api/metrics/simulate/brownout")
    require("decision" in brownout, "brownout simulation missing decision")

    marker = call("POST", "/api/production/marker", {"title": "smoke marker", "note": "api smoke", "tags": ["smoke"]})
    require(marker.get("title") == "smoke marker", "marker was not created")

    replay = call("POST", "/api/production/replay/save")
    require(replay.get("saved") is True, "replay save did not report success")

    manifest = call("GET", "/api/release/manifest")
    require(manifest.get("schema_revision") == SCHEMA_REVISION, "release manifest schema revision is not current")

    alpha = call("GET", "/api/alpha/readiness")
    require("report" in alpha, "alpha readiness missing report")

    field = call("GET", "/api/field/readiness")
    require("report" in field, "field readiness missing report")

    devices = call("GET", "/api/field/device-checklists")
    require("devices" in devices, "field device checklists missing devices")

    bundle = call("GET", "/api/session/support-bundle")
    require("release" in bundle, "support bundle missing release context")
    require("alpha_validation" in bundle, "support bundle missing alpha validation context")
    require("field_validation" in bundle, "support bundle missing field validation context")

    print("OpenIRL feature areas API smoke test passed")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:  # noqa: BLE001 - smoke script should report all failures plainly.
        print(f"smoke test failed: {exc}", file=sys.stderr)
        raise SystemExit(1)
