#!/usr/bin/env bash
set -euo pipefail
curl -fsS -X POST http://127.0.0.1:7707/api/session/support-bundle/export
