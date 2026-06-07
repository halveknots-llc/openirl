//! Post-stream diagnostics report model.

use openirl_core::{HealthDecision, HealthState, StreamMetrics};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// One session sample.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionSample {
    /// Metrics.
    pub metrics: StreamMetrics,
    /// Health decision for those metrics.
    pub decision: HealthDecision,
}

/// Summarized session report.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StreamReport {
    /// Report creation time.
    pub generated_at: OffsetDateTime,
    /// Total samples.
    pub sample_count: usize,
    /// Lowest score observed.
    pub lowest_score: u8,
    /// Highest packet loss observed.
    pub worst_packet_loss_percent: f32,
    /// Highest RTT observed.
    pub highest_rtt_ms: u32,
    /// BRB activations observed.
    pub brb_activations: usize,
    /// Human-readable summary.
    pub summary: String,
    /// Recommended actions.
    pub recommendations: Vec<String>,
}

/// Lightweight support bundle manifest returned before file-export support lands.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SupportBundleManifest {
    /// Bundle creation time.
    pub generated_at: OffsetDateTime,
    /// Whether sensitive values should be redacted.
    pub redacted: bool,
    /// Files or sections expected in a future exported bundle.
    pub sections: Vec<String>,
    /// Diagnostic report summary included in the bundle.
    pub report: StreamReport,
}

impl SupportBundleManifest {
    /// Creates a support bundle manifest from a report.
    #[must_use]
    pub fn from_report(report: StreamReport, redacted: bool) -> Self {
        Self {
            generated_at: OffsetDateTime::now_utc(),
            redacted,
            sections: vec![
                "redacted-config".to_string(),
                "session-health-report".to_string(),
                "recent-events".to_string(),
                "obs-dry-run-actions".to_string(),
                "profile-support-matrix".to_string(),
            ],
            report,
        }
    }
}

/// Builds reports from samples.
#[derive(Debug, Default)]
pub struct ReportBuilder {
    samples: Vec<SessionSample>,
}

impl ReportBuilder {
    /// Creates an empty report builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a sample.
    pub fn push(&mut self, sample: SessionSample) {
        self.samples.push(sample);
    }

    /// Builds a report.
    #[must_use]
    pub fn build(&self) -> StreamReport {
        let sample_count = self.samples.len();
        let lowest_score = self
            .samples
            .iter()
            .map(|sample| sample.decision.score)
            .min()
            .unwrap_or(100);
        let worst_packet_loss_percent = self
            .samples
            .iter()
            .map(|sample| sample.metrics.packet_loss_percent)
            .fold(0.0_f32, f32::max);
        let highest_rtt_ms = self
            .samples
            .iter()
            .map(|sample| sample.metrics.rtt_ms)
            .max()
            .unwrap_or_default();
        let brb_activations = self
            .samples
            .iter()
            .filter(|sample| {
                matches!(
                    sample.decision.state,
                    HealthState::Brb | HealthState::Offline
                )
            })
            .count();

        let mut recommendations = Vec::new();
        if worst_packet_loss_percent >= 8.0 {
            recommendations.push(
                "Increase SRT latency or reduce contribution bitrate before next stream."
                    .to_string(),
            );
        }
        if highest_rtt_ms >= 700 {
            recommendations.push(
                "Review carrier/modem placement; RTT repeatedly entered brownout range."
                    .to_string(),
            );
        }
        if brb_activations > 0 {
            recommendations.push(
                "Verify fallback video/BRB scene timing and recovery hold duration.".to_string(),
            );
        }
        if recommendations.is_empty() {
            recommendations.push("No major action required from sampled metrics.".to_string());
        }

        StreamReport {
            generated_at: OffsetDateTime::now_utc(),
            sample_count,
            lowest_score,
            worst_packet_loss_percent,
            highest_rtt_ms,
            brb_activations,
            summary: format!(
                "Analyzed {sample_count} samples. Lowest score: {lowest_score}. BRB/offline events: {brb_activations}."
            ),
            recommendations,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openirl_core::{HealthDecision, SceneRole};

    #[test]
    fn report_mentions_brb_activation() {
        let mut builder = ReportBuilder::new();
        builder.push(SessionSample {
            metrics: StreamMetrics {
                packet_loss_percent: 10.0,
                rtt_ms: 900,
                ..StreamMetrics::default()
            },
            decision: HealthDecision {
                state: HealthState::Brb,
                score: 10,
                reasons: vec!["test".to_string()],
                recommended_scene: SceneRole::Brb,
            },
        });
        let report = builder.build();
        assert_eq!(report.brb_activations, 1);
        assert!(!report.recommendations.is_empty());
    }
}
