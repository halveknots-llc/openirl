# Support

OpenIRL is an alpha project for operators who are comfortable validating local tooling and live streaming dependencies in their own environment.

## Where to Ask

- Use GitHub issues for reproducible bugs, documentation fixes, feature requests, and field reports.
- Use the field report issue template when a mobile encoder, network route, brownout, relay, or OBS recovery behavior is involved.
- Use [SECURITY.md](SECURITY.md) for vulnerabilities or sensitive production reports.

## Before Opening an Issue

Run the smallest relevant local check and include the command output that does not contain secrets. Useful commands include:

```bash
python3 scripts/static_validate.py
python3 scripts/audit/handoff_audit.py
cargo xtask ci
cargo run --package openirl-agent -- check-config --config config/openirl.example.toml
```

For live dependency issues, include which real dependency was used: OBS Studio version, MediaMTX version, encoder app/device, relay host, tunnel tool, or Windows host. Do not claim a live check passed unless it ran against that dependency.

## Sharing Support Bundles

Review support bundles before sharing them. They are designed to redact sensitive values, but operators remain responsible for checking that exports do not contain stream keys, passphrases, dashboard tokens, OBS passwords, relay credentials, or private network details.
