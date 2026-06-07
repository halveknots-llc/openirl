#!/usr/bin/env bash
set -euo pipefail
curl -fsS http://127.0.0.1:7707/api/obs/template >/tmp/openirl-obs-template.json
curl -fsS -X POST http://127.0.0.1:7707/api/obs/template/apply
