# OBS Automation

OpenIRL controls OBS through a trait-based controller with two concrete modes:

- Review controller: records intended actions and validates automation plans without touching OBS.
- OBS WebSocket controller: connects to OBS Studio's built-in WebSocket server and sends request/response commands.

## Scenes

OpenIRL manages these semantic scene roles:

- Live
- Low Signal
- BRB
- Backup Feed
- Privacy
- Starting Soon
- Ending

## Safety

Do not expose OBS WebSocket to the public internet. Use localhost or a private tunnel with authentication.
