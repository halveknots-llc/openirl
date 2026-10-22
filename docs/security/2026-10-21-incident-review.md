# Dependency and control-plane security review

Reviewed October 21, 2026 (America/Los_Angeles). Scope: the published OpenIRL
workspace plus the pending maintenance changes, its resolved Cargo dependencies,
dashboard request boundary, relay/log input boundaries, and build/release controls.
This is an incident applicability review and targeted hardening pass, not a claim
that the application or its external infrastructure is free of vulnerabilities.

## Incident applicability

| Report | Repository exposure and response |
| --- | --- |
| [eMudhra intermediate compromise, discussed on Hacker News](https://news.ycombinator.com/item?id=50166579), CVE-2026-111943 | **Affected dependency found.** A fresh `cargo deny --locked check` rejected `rustls-webpki 0.103.15` through rustls and the OBS WebSocket adapter. Updated to 0.103.16, whose upstream fix rejects certificate paths containing the distrusted keys. This repairs the TLS-capable dependency; it does not encrypt the agent's configured plaintext OBS connection. |
| [RustSec RUSTSEC-2026-0382](https://rustsec.org/advisories/RUSTSEC-2026-0382.html) and [upstream fix](https://github.com/rustls/webpki/pull/546) | The advisory was issued October 20. The stable fixed range starts at 0.103.16. Related [webpki-roots](https://rustsec.org/advisories/RUSTSEC-2026-0381.html) and [webpki-root-certs](https://rustsec.org/advisories/RUSTSEC-2026-0383.html) packages are absent from this lockfile. Native certificate-store updates remain a separate host responsibility. |
| [PHP security release, October 21](https://www.php.net/) | CVE-2026-109618 and CVE-2026-109619 affect PHP request processing. No PHP source, package manifest, interpreter integration, or PHP deployment image was found in this repository. No PHP workaround was applied to the Rust service. |
| [Node.js security release, October 21](https://nodejs.org/en/blog/vulnerability/october-2026-security-releases) | Includes TLS/DNS checks, Brotli memory corruption and dependency fixes. OpenIRL has no Node application runtime or npm dependency graph. JavaScript GitHub Actions do use runner-managed Node: their embedded runtime and the runner image must be patched by the runner provider. A Cargo update does not establish that patch status. |
| [GitLab critical patch release, October 21](https://docs.gitlab.com/ja-jp/releases/patches/patch-release-gitlab-19-5-1-released/) | CVE-2026-110264 concerns GitLab's regular-expression compiler; OpenIRL is hosted on GitHub and has no GitLab server or Onigmo dependency. The Rust regex crate is a different implementation. Its stale Dependabot exclusion was removed independently. |
| [OVHcloud CLI release-asset compromise, October 19](https://github.com/ovh/ovhcloud-cli/security/advisories/GHSA-862f-55f5-vmvw) | No OVHcloud CLI dependency or download step was found. The supply-chain lesson applies to release integrity: retain immutable action/container pins, hashed Python requirements, source-bound Windows verification and trusted-push provenance; prevent dependency re-resolution and retained checkout credentials. No machine-wide compromise assessment or credential rotation is implied. |
| [CISA KEV feed](https://www.cisa.gov/sites/default/files/feeds/known_exploited_vulnerabilities.json), catalog 2026.10.21 | Recent additions since October 19 cover SiteGuard, Plex Media Server and Siemens Software Center. None is a repository dependency or declared integration. The feed was retrieved directly; the catalog web page returned HTTP 403. |

Hacker News and The Hacker News were used for discovery. Primary advisories,
upstream source, the lockfile and executable checks determined applicability.
The English GitLab page and Mozilla blog were unavailable through web retrieval;
the official GitLab locale page and RustSec/upstream patch supplied the relevant
evidence. Absence from this repository does not establish absence on an operator's
machine, relay host, browser, or CI runner.

## Dependency changes

The candidate retains Rust 1.88 and updates these versions relative to the pending
maintenance branch:

| Package | Before | After | Dependabot PR |
| --- | --- | --- | --- |
| rustls-webpki | 0.103.15 | 0.103.16 | Emergency advisory fix |
| thiserror / thiserror-impl | 2.0.20 | 2.0.21 | #40 |
| futures-util and its core/macro/sink/task crates | 0.3.32 | 0.3.34 | #41 |
| tokio | 1.53.1 | 1.54.1 | #42, superseding its earlier target |
| tower-http | 0.6.11 | 0.7.1 | #43 |
| uuid | 1.24.1 | 1.26.1 | #44 |

All five PR patches were inspected. Cargo selected Rust-1.88-compatible versions;
uuid 1.28.0 requires Rust 1.89 and was not selected. The lockfile has 196 registry
package entries. Existing rustls 0.23.46 and chacha20 0.10.2 fixes from the pending
maintenance branch are preserved. No advisory is ignored. Duplicate transitive
version warnings remain visible; they are not vulnerability exceptions.

`cargo update -p regex` succeeded without changing its current 1.13.0 resolution.
That bounded probe supports removing the historical ignore, but does not prove
the next hosted Dependabot update will succeed.

## Controls and verification

- Added framing denial, MIME-sniffing prevention and referrer suppression on all
  dashboard responses, plus `no-store` on API responses. The live HTTP regression
  probe covers normal/static responses, missing routes and early origin rejection.
- Kept existing loopback Host/Origin and token enforcement, public-bind rejection,
  bounded metrics/log ingestion, argument-based process execution and shared secret
  redaction. The security smoke starts the real local agent and exercises these
  browser/control and support-export boundaries.
- Added locked dependency resolution to CI, security/demo smokes and build entry
  points, rejected yanked dependencies, disabled credential persistence for CI
  and CodeQL checkouts, and grouped Dependabot minor/patch updates.
- `cargo --locked xtask ci` passed: 16 Python policy tests, static validation,
  handoff audit, security smoke, release scanner self-test, demo smoke, dependency
  policy, formatting, Clippy, 103 Rust tests and doc-test execution. Rust counts
  come from the original successful test output, not artifact manifests.
- All-feature Clippy and workspace tests also passed with the optional OBS
  WebSocket adapter enabled (103 passing Rust tests). This is compiled/tested
  adapter code, not a live OBS connection.

Context7 was consulted for cargo-deny, tower-http and Axum. Its Axum versioned
coverage was 0.8.4, while this project uses 0.8.9; the installed 0.8.9 routing source
confirmed middleware wrapping of routes and fallbacks. Installed tower-http 0.7.1
source/changelog and the actual Rust 1.88 build resolved its compatibility contract.
The new CSP addresses framing/base/object controls, not complete script isolation.

## Integration and remaining evidence

The candidate starts from maintenance commit
`2975188f6948366ad96242e08f516f0cbc0854de`. Public `main` remained
`6eb7489340385bf810ad89aeb37bb19cb9cc8d67` when inspected. The existing instruction
to complete signed Creator Presence publication before merging maintenance still
applies; no public-main fast-forward or release is established by this review.

No live OBS, MediaMTX, SRTLA, mobile encoder, Docker image or Windows installer was
tested locally. Those tools were not available on PATH. Local Rust/HTTP validation
does not substitute for hosted Windows packaging, release signing/provenance,
physical streaming, or patching separately installed tools and operating systems.
Container digest pins ensure identity, not current vulnerability-free contents;
no container OS CVE scan was performed without a container runtime.
