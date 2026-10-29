# Cloud sessions

A Claude Code cloud session runs this repo on Ubuntu 24.04 x86_64: the Rust workspace builds and tests there, but there is no Windows, OBS, MediaMTX, mobile encoder or BELABOX hardware.

Set up once per session: `bash .claude/hooks/session-start.sh` (runs `cargo fetch --locked`; rustup pulls the 1.88 toolchain from `rust-toolchain.toml` on first use). Single-repo cloud sessions run it as a SessionStart hook; project threads don't run repo hooks, so run it yourself before any check.

All of the AGENTS.md build-and-test commands pass here. `cargo xtask ci` also runs `cargo deny`, which isn't in the stock image: install CI's version once with `cargo install cargo-deny --version 0.19.7 --locked` (a few minutes to build) before running it.

Can't run here: the Windows installer and `scripts/windows` packaging, and smoke scripts that need live OBS, MediaMTX, relays or encoders. Leave those to a machine that has them and say in the PR that you didn't run them.

When `scripts/check` exists, it is the gate to run before pushing.
