//! Handoff feature plan for OpenIRL.

use serde::Serialize;

/// Feature area represented in the handoff package.
#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct FeatureArea {
    /// Stable feature key used by APIs and docs.
    pub key: &'static str,
    /// Human-readable name.
    pub name: &'static str,
    /// Product goal.
    pub goal: &'static str,
    /// Evidence that the area has a concrete implementation contract.
    pub evidence: &'static [&'static str],
}

/// Number of pre-handoff feature areas captured in the initial planning contract.
pub const HANDOFF_PHASE_COUNT: u8 = 8;

/// Returns the initial feature plan without numbered development labels.
#[must_use]
pub fn handoff_phases() -> Vec<FeatureArea> {
    vec![
        FeatureArea {
            key: "product-contract",
            name: "Product contract and Rust constraints",
            goal: "Freeze local-first OSS scope and Rust-first implementation boundaries.",
            evidence: &[
                "OpenIRL is defined as a local OBS control plane, not a managed Cloud OBS product.",
                "Rust is accepted as the core implementation language.",
            ],
        },
        FeatureArea {
            key: "workspace-quality",
            name: "Rust workspace and quality gates",
            goal: "Create a multi-crate workspace that can be validated and released consistently.",
            evidence: &[
                "Cargo workspace exists with apps, services, crates, docs, deploy assets, and xtask.",
                "Workspace lint inheritance and Rust 2024 are configured.",
            ],
        },
        FeatureArea {
            key: "health-engine",
            name: "Core domain model and health engine",
            goal: "Represent metrics, states, scenes, protocols, encoders, and health decisions.",
            evidence: &[
                "Health engine classifies healthy, degraded, brownout, BRB, backup, and offline states.",
                "Scene decisions are deterministic and unit-testable.",
            ],
        },
        FeatureArea {
            key: "agent-dashboard",
            name: "Local agent API and mobile dashboard",
            goal: "Expose local-first control without Discord or cloud dependency.",
            evidence: &[
                "Agent serves local APIs and a phone-oriented dashboard shell.",
                "Dashboard exposes OBS, profile, metrics, relay, and support-bundle controls.",
            ],
        },
        FeatureArea {
            key: "obs-profiles",
            name: "OBS automation and profile generation",
            goal: "Make setup simple through typed OBS adapters and encoder profile generation.",
            evidence: &[
                "OBS automation trait, review controller, and WebSocket controller boundaries exist.",
                "Moblin, IRL Pro, Larix, and BELABOX profiles are generated through typed requests.",
            ],
        },
        FeatureArea {
            key: "relay-protocols",
            name: "Relay and protocol integration",
            goal: "Support SRT/SRTLA/RTMP/RIST/WebRTC paths without binding Rust directly to all media engines.",
            evidence: &[
                "Process-bound relay supervision exists for MediaMTX, SRTLA helpers, go-irl style bridges, and custom tools.",
                "Relay plans, readiness, credentials, metrics endpoints, and process state are exposed through APIs.",
            ],
        },
        FeatureArea {
            key: "diagnostics-security",
            name: "Diagnostics, redaction, and safety model",
            goal: "Make stream failures explainable and secrets safe by default.",
            evidence: &[
                "Session reports, support-bundle exports, and timeline diagnostics exist.",
                "Secret redaction, config validation, auth checks, and LAN exposure warnings are modeled.",
            ],
        },
        FeatureArea {
            key: "handoff-readiness",
            name: "Handoff readiness",
            goal: "Provide a clean, feature-oriented package for further implementation and live validation.",
            evidence: &[
                "Feature docs, validation scripts, source package layout, presets, and handoff tasks are included.",
                "Repository audit checks for missing docs, legacy pass labels, sample markers, and parse failures.",
            ],
        },
    ]
}

/// Returns true when all initial handoff feature areas are represented.
#[must_use]
pub fn handoff_contract_complete() -> bool {
    handoff_phases().len() == usize::from(HANDOFF_PHASE_COUNT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handoff_contract_has_expected_count() {
        assert_eq!(handoff_phases().len(), usize::from(HANDOFF_PHASE_COUNT));
    }
}
