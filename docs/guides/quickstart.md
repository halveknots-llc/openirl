# Quickstart

1. Start OBS and enable OBS WebSocket authentication.
2. Start `openirl-agent serve --config config/openirl.example.toml --obs-adapter web-socket`.
3. Open `http://127.0.0.1:7707`.
4. Materialize fallback assets and the OBS template.
5. Start MediaMTX with `deploy/mediamtx/openirl.mediamtx.yml`.
6. Generate a Moblin or IRL Pro profile.
7. Verify healthy metrics, then force brownout and recovery.
