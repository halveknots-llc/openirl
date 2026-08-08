# Moblin

Moblin is the preferred first iOS validation target because it can exercise a local SRT contribution path before more complex relay or bonding work.

## Setup Path

1. Validate OpenIRL source checks.
2. Start MediaMTX or the configured local ingest path.
3. Generate a Moblin profile from OpenIRL.
4. Import or scan the profile on the iOS device.
5. Publish into the local ingest path.
6. Confirm metrics and OBS source behavior.

## Profile Command

```bash
cargo run --package openirl-agent -- profile \
  --encoder moblin \
  --protocol srt \
  --host 127.0.0.1 \
  --port 9000 \
  --stream-id openirl-main
```

Replace `127.0.0.1` with the OBS or relay host address reachable from the phone. Keep passphrases and stream keys out of committed config.

## Evidence

Follow the [public evidence policy](../PUBLIC_EVIDENCE.md): before publishing,
remove stream credentials, authentication credentials, credential-bearing URLs,
private network details, local paths, device identifiers, location-sensitive
media, private-production stream IDs, and raw support bundles.

Record the Moblin version, iOS version, protocol, endpoint host class, and whether the generated profile was accepted. A public field report should remove passphrases, stream IDs that identify a private production setup, and location-adjacent notes.

Submit a reviewed result against the `moblin-profile` row using the
[compatibility evidence process](../COMPATIBILITY.md). Profile generation alone
remains source evidence until a real Moblin version and iOS host are recorded.
