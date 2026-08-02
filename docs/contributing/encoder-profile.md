# Add an Encoder Profile

Use this recipe for a mobile app, backpack encoder, hardware family, or a new
protocol combination on an existing encoder. Profile generation creates
credential-bearing contribution URLs, so raw output and display-safe output are
separate security surfaces.

## Ownership map

| Surface | Responsibility |
| --- | --- |
| `crates/openirl-core/src/models.rs` | `EncoderKind`, `Protocol`, serialized names, and display names |
| `crates/openirl-profiles/src/lib.rs` | support matrix, compatibility rules, URL generation, notes, and redacted display URL |
| `apps/openirl-agent/src/main.rs` | CLI `EncoderArg` and protocol conversion |
| `apps/openirl-agent/static/index.html` | Dashboard selector and labels |
| `presets/encoders/` | Public-safe import shape with no credential value |
| `docs/features/encoder-profiles.md` | Operator behavior and supported boundaries |
| `docs/COMPATIBILITY.md` | Source, device-import, contribution, and field evidence |

## Implementation steps

1. Copy `fixtures/contributing/encoder-profile.sample.json` and keep its host,
   stream ID, and passphrase visibly synthetic.
2. Add or update `EncoderKind` and its exhaustive `Display` match. A new shared
   variant may reveal additional exhaustive matches; review each compiler error
   instead of adding broad wildcard behavior.
3. Update `support_matrix`, `supported_protocols`, `supports`, URL construction,
   and operator notes in `openirl-profiles`.
4. Add the CLI enum conversion and dashboard option. Search the repository for
   the neighboring Moblin, IRL Pro, Larix, and BELABOX variants to find
   onboarding, field-plan, package, and documentation surfaces that genuinely
   apply.
5. Add a preset without a real stream key or passphrase. Keep the default host
   on loopback unless the operator explicitly supplies a broader endpoint.
6. Add a focused fixture test beside the generator tests. Assert the raw URL has
   the synthetic canary only when required for runtime use, while `display_url`,
   logs, reports, and serialized public evidence do not.

Do not add an encoder as `Custom` merely to avoid exhaustive updates when it is
intended to be a first-class supported family. Conversely, do not add a shared
enum variant for a one-off operator preset that the existing custom path can
represent safely.

## Smallest local gate

```bash
cargo test --package openirl-profiles
cargo clippy --package openirl-profiles --all-targets -- -D warnings
python3 scripts/static_validate.py
```

Then run `cargo xtask ci` because a shared encoder enum can affect the agent,
dashboard contracts, onboarding, field validation, and package materializers.

## Live evidence boundary

Generation and JSON parsing establish source maturity only. Record these as
separate observations when real hardware is available:

- app or firmware accepted the profile
- encoder established the contribution connection
- MediaMTX or the relay observed the publisher
- OpenIRL ingested metrics
- OBS displayed media
- brownout and recovery behavior worked

Name exact app/device OS versions, OpenIRL commit, protocol, and non-secret
configuration class in the compatibility process. A QR rendering or successful
import does not prove contribution media.
