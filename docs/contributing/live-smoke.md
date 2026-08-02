# Add a Live Smoke Check

A live smoke check contacts a named external dependency. It must distinguish a
missing environment from a pass, fail closed on an unsuccessful observation,
and record a narrower claim than a full production certification.

## Choose the evidence level first

- Source check: parses config, fixtures, or command plans without starting the
  dependency.
- Local-runtime check: starts only OpenIRL or a deterministic fake process.
- Integration check: contacts an exact OBS, MediaMTX, relay, tunnel, encoder, or
  Windows environment under controlled conditions.
- Field check: exercises the real device, network path, media, brownout, and
  recovery workflow with an operator.

Do not name a source parser a live smoke check. The existing mobile
`profile-compat-smoke` scripts validate preset JSON only; real import and
contribution evidence follows the mobile field runbook.

## Script contract

1. Put the script under the matching `scripts/` integration directory and use a
   descriptive dependency/workflow name.
2. Require an explicit opt-in variable such as `OPENIRL_LIVE_<AREA>_SMOKE=1`.
   Exit nonzero with a prerequisite message when it is absent.
3. Default control endpoints to loopback. Require explicit configuration for
   broader addresses and never place credentials in command arguments or URLs.
4. Use strict shell or PowerShell error handling, bounded request timeouts, and
   structured response parsing. Do not use `|| true`, empty exception handlers,
   or success output after a failed probe.
5. Check observable behavior, not process existence alone. Examples include an
   active MediaMTX publisher, expected OBS scene state, relay lifecycle and
   metrics, or a packaged CLI result.
6. Write optional evidence only under an ignored artifact directory. Redact it,
   bound its size, and review it manually before sharing a minimal excerpt.
7. Print a concise pass message naming the dependency and check. Never print a
   credential-bearing request, full environment, raw response, or private path.

A Bash guard should follow this shape:

```bash
if [[ "${OPENIRL_LIVE_EXAMPLE_SMOKE:-}" != "1" ]]; then
  echo "Set OPENIRL_LIVE_EXAMPLE_SMOKE=1 after the dependency is ready." >&2
  exit 2
fi
curl --fail --show-error --silent --max-time 5 \
  "${OPENIRL_EXAMPLE_URL:-http://127.0.0.1:9997/health}" >/dev/null
echo "example live smoke reached the configured dependency"
```

Add PowerShell parity when the workflow is supported on Windows. Keep the same
opt-in, timeout, localhost default, structured assertion, and failure semantics.

## Fixture and focused gate

Start from `fixtures/contributing/live-smoke-evidence.sample.json`. It is
deliberately `modeled` and `not-run`; replace versions, revision, result, and a
repository-relative reviewed artifact only after the dependency actually ran.

```bash
bash -n scripts/<area>/<name>-smoke.sh
shellcheck scripts/<area>/<name>-smoke.sh
python3 scripts/static_validate.py
```

Run any source companion test and `cargo xtask ci`. Those local passes do not
change the live result.

## Compatibility evidence

Record the exact dependency version, host OS/version, full OpenIRL commit,
non-secret configuration class, script path, evidence maturity, result, and
reviewed repository-relative artifact. Submit it through the field-report
template. Do not commit raw logs, complete support bundles, private network
topology, device identifiers, or location-sensitive media.
