# Dashboard

The dashboard is the local operator surface for setup, stream state, OBS actions, profile generation, relay planning, and support exports. It is designed to start on localhost and require explicit configuration before broader network exposure.

## Source Validation

```bash
cargo run --package openirl-agent -- check-config --config config/openirl.example.toml
cargo run --package openirl-agent -- serve --config config/openirl.example.toml
python3 scripts/smoke/api_smoke.py
```

Expected evidence:

- `/health` reports `status: ok`
- `/api/config/validation` reports no error findings for the example config
- `/api/auth/status` reflects whether dashboard auth is enabled
- `/api/runtime/readiness` and `/api/state` return redacted operator state

## Auth Boundary

Loopback dashboard access may be tokenless when `allow_loopback_without_token` is enabled. Non-loopback API requests require authorization when `require_auth_outside_localhost` is true. Public bind without auth must be rejected during config validation.

Dashboard bearer tokens are held only for the current page session by the bundled static dashboard. They are not persisted to browser `localStorage`.

CORS is same-origin only by default. If an operator serves a custom dashboard from a different browser origin, list that exact `http://` or `https://` origin in `api.cors_allowed_origins`; wildcard origins are rejected by config validation.

Use this check when changing bind or auth behavior:

```bash
python3 scripts/security/security-audit-smoke.py
```

## Operator Workflow

1. Validate the config.
2. Start the agent on `127.0.0.1`.
3. Open `http://127.0.0.1:7707`.
4. Confirm redacted config, metrics status, OBS status, and support-bundle export.
5. Only then test LAN access with a token and a deliberate bind.

## Current Boundary

API smoke checks prove source-level dashboard behavior. A phone-on-LAN test is still required before claiming mobile dashboard success in a specific network.
