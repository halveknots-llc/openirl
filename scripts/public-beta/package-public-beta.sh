#!/usr/bin/env bash
set -euo pipefail

ROOT="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

printf '%s\n' \
  'public evidence review: remove stream credentials, authentication credentials, credential-bearing URLs, private network details, local paths, device identifiers, location-sensitive media, private-production stream IDs, and raw support bundles' >&2

if ! git diff --quiet HEAD -- || [ -n "$(git status --porcelain --untracked-files=all)" ]; then
  printf '%s\n' 'refusing to package a dirty worktree; commit or clean all changes first' >&2
  exit 1
fi

revision="$(git rev-parse HEAD)"
staging="$(mktemp -d)"
archive="$staging/source.tar"
trap 'rm -rf "$staging"' EXIT

git archive --format=tar --worktree-attributes --output="$archive" HEAD \
  docs presets issue_templates plugin
tar -xf "$archive" -C "$staging"

output="$ROOT/artifacts/v1-public-beta"
rm -rf "$output"
mkdir -p "$output"
cp -R "$staging/docs" "$staging/presets" "$staging/issue_templates" "$staging/plugin" "$output/"
printf '{"schema_version":1,"source_revision":"%s","paths":["docs","presets","issue_templates","plugin"]}\n' \
  "$revision" > "$output/package-manifest.json"
printf 'public beta package refreshed from %s\n' "$revision"
