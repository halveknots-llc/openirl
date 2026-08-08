# Public Evidence Safety

OpenIRL accepts only minimal, synthetic, or deliberately reviewed evidence in
public issues, pull requests, compatibility rows, fixtures, and release
packages. Raw production evidence stays local unless the operator creates a
narrow redacted excerpt and reviews the final bytes before submission.

Before publishing, remove stream credentials, authentication credentials,
credential-bearing URLs, private network details, local paths, device
identifiers, location-sensitive media, private-production stream IDs, and raw
support bundles.

The machine-readable policy in
[`public-evidence-policy.json`](public-evidence-policy.json) lists every guarded
submission surface. `python3 scripts/static_validate.py` fails when a listed
surface omits any required review class. This validates guidance coverage; it
does not certify an attachment or replace final human review.
