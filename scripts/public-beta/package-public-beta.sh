#!/usr/bin/env bash
set -euo pipefail
mkdir -p artifacts/v1-public-beta
cp -R docs presets issue_templates plugin artifacts/v1-public-beta/
echo public beta package refreshed
