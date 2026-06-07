# End-to-End Smoke Test

1. Start OBS and enable WebSocket authentication.
2. Start MediaMTX with the local OpenIRL config.
3. Start `openirl-agent serve`.
4. Open the dashboard.
5. Generate an encoder profile.
6. Publish SRT from a device or test publisher.
7. Confirm metrics polling and health evaluation.
8. Trigger a degraded/brownout scenario.
9. Confirm scene switching.
10. Export a support bundle.
