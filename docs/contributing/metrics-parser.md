# Add a Metrics Parser

Use this recipe for a new Prometheus metric family, MediaMTX version shape, or
SRT/SRTLA status-line field. Metric ingestion feeds brownout decisions, so parser
changes must define malformed, missing, oversized, and counter-reset behavior.

## Ownership map

| Surface | Responsibility |
| --- | --- |
| `crates/openirl-metrics/src/lib.rs` | Bounded HTTP polling, text parsing, reduction, stateful deltas, and `StreamMetrics` conversion |
| `fixtures/contributing/metrics-exporter.sample.prom` | Minimal synthetic exposition shape |
| `fixtures/metrics/` | Versioned regression and brownout scenarios |
| `docs/features/brownout.md` | Operator meaning of derived health inputs |
| `docs/features/local-ingest.md` | Router/exporter configuration and live boundary |

## Implementation steps

1. Copy the synthetic metrics fixture and include only the labels and counters
   needed for the new behavior. Avoid stream names, hostnames, IDs, or labels
   copied from production.
2. Reuse `parse_prometheus_text` for standard exposition. Add named reduction
   logic rather than introducing a second text parser for one exporter.
3. For status lines, extend the structured key/value mapping and define accepted
   units. Unknown fields should remain ignorable; malformed values should create
   a bounded error or warning rather than a panic.
4. Keep network collection in `poll_http_text`: HTTP only, localhost by default,
   explicit timeout, response-size cap, and no redirect or credential logging.
5. Use saturating arithmetic for cumulative counters and define behavior for a
   reset, missing previous sample, zero elapsed time, NaN, and impossible ranges.
6. Add a parser test for the fixture and a reducer test for the exact
   `StreamMetrics` fields that change. Add a negative or boundary case whenever
   untrusted text handling changes.

Do not infer healthy media solely from endpoint reachability. A reachable
metrics server with no active publisher is a distinct observation.

## Smallest local gate

```bash
cargo test --package openirl-metrics --package openirl-health
cargo clippy --package openirl-metrics --package openirl-health --all-targets -- -D warnings
```

Run `cargo xtask ci` before review because metric fields can change dashboard,
session, readiness, and brownout behavior.

## Live evidence boundary

A fixture proves parser compatibility with recorded synthetic text. Integration
evidence requires the exact exporter/router version, host platform, endpoint
class, and a real active and inactive path observation. Field maturity further
requires degraded-link and recovery behavior. Never attach a raw scrape if its
labels expose private paths, device IDs, endpoints, or operator names.
