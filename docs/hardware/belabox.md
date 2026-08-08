# BELABOX

BELABOX workflows are useful for backpack encoder and bonding-oriented field paths. Treat them as live dependency validation: source checks alone do not prove BELABOX compatibility.

## Setup Path

1. Prove local SRT ingest with a simpler publisher.
2. Review the BELABOX endpoint, stream ID, and passphrase behavior.
3. Configure the relay or SRTLA receiver deliberately.
4. Publish from BELABOX into the OpenIRL-monitored path.
5. Confirm metrics, OBS source behavior, brownout response, and support-bundle export.

## Validation Focus

- SRTLA receiver compatibility
- passphrase handling
- reconnect behavior after network loss
- whether OpenIRL classifies degradation and recovery clearly
- whether support bundles redact endpoint and credential details

## Evidence

Follow the [public evidence policy](../PUBLIC_EVIDENCE.md): before publishing,
remove stream credentials, authentication credentials, credential-bearing URLs,
private network details, local paths, device identifiers, location-sensitive
media, private-production stream IDs, and raw support bundles.

Record BELABOX software or hardware version, receiver tool, protocol, relay host class, and the observed recovery behavior. Do not publish passphrases, private relay credentials, device identifiers, or location-adjacent notes.

Submit a reviewed result against the `belabox-profile` row using the
[compatibility evidence process](../COMPATIBILITY.md). Keep raw production logs
and unreviewed support bundles out of the public issue.
