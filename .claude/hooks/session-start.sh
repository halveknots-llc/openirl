#!/bin/bash
# Installs this repo's dependencies for Claude Code cloud sessions.
# Registered as a SessionStart hook in .claude/settings.json, which runs in
# single-repo cloud sessions. Project threads don't run repo hooks, so there
# run it by hand once: bash .claude/hooks/session-start.sh
# Each step re-runs only when its inputs change. Always exits 0 so a failed
# install never blocks a session; failures are reported on stderr.
set -uo pipefail

if [ "${CLAUDE_CODE_REMOTE:-}" != "true" ] && [ "${1:-}" != "--force" ]; then
  exit 0 # local machine: does nothing unless run with --force
fi

ROOT="${CLAUDE_PROJECT_DIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)}"
cd "$ROOT" || exit 0
SELF="$ROOT/.claude/hooks/session-start.sh"
STAMPS="$(git rev-parse --absolute-git-dir 2>/dev/null || echo "$ROOT/.git")/claude-session-start"
mkdir -p "$STAMPS"
failed=0

# step NAME DIR OUTPUT CMD INPUT...
# Runs CMD in DIR unless the inputs and this script are unchanged since the
# last success and OUTPUT (or "-") still exists. Logs go to $STAMPS/NAME.log.
step() {
  local name="$1" dir="$2" output="$3" cmd="$4" stamp want
  shift 4
  stamp="$STAMPS/$name.sha256"
  want="$(cat "$@" "$SELF" 2>/dev/null | sha256sum | cut -d' ' -f1)"
  if [ -f "$stamp" ] && [ "$(cat "$stamp")" = "$want" ] && { [ "$output" = "-" ] || [ -e "$output" ]; }; then
    echo "session-start: $name up to date"
    return 0
  fi
  if (cd "$dir" && bash -c "$cmd") >"$STAMPS/$name.log" 2>&1; then
    echo "$want" >"$stamp"
    echo "session-start: $name installed"
  else
    rm -f "$stamp"
    failed=1
    echo "session-start: $name FAILED ($cmd); last lines of $STAMPS/$name.log:" >&2
    tail -n 15 "$STAMPS/$name.log" >&2
  fi
}

step rust '.' '-' \
  'cargo fetch --locked' \
  'Cargo.toml' 'Cargo.lock'

[ "$failed" -eq 0 ] || echo "session-start: some installs failed; tests that need them may not run" >&2
exit 0
