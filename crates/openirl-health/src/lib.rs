//! Brownout-aware stream health engine.

use openirl_core::{HealthDecision, HealthState, HealthThresholds, SceneRole, StreamMetrics};
use thiserror::Error;

/// Health engine errors.
#[derive(Debug, Error, Eq, PartialEq)]
pub enum HealthError {
    /// Invalid metric input.
    #[error("invalid stream metrics: {0}")]
    InvalidMetrics(&'static str),
}

/// Stateful health evaluator.
#[derive(Debug, Clone)]
pub struct HealthEngine {
    thresholds: HealthThresholds,
    previous_state: HealthState,
    last_bad_timestamp_ms: Option<u64>,
}

impl HealthEngine {
    /// Creates an engine with default thresholds.
    #[must_use]
    pub fn new() -> Self {
        Self::with_thresholds(HealthThresholds::default())
    }

    /// Creates an engine with custom thresholds.
    #[must_use]
    pub fn with_thresholds(thresholds: HealthThresholds) -> Self {
        Self {
            thresholds,
            previous_state: HealthState::Healthy,
            last_bad_timestamp_ms: None,
        }
    }

    /// Returns thresholds.
    #[must_use]
    pub const fn thresholds(&self) -> &HealthThresholds {
        &self.thresholds
    }

    /// Evaluates one sample and updates recovery state.
    ///
    /// # Errors
    ///
    /// Returns an error if the metrics contain invalid values.
    pub fn evaluate(&mut self, metrics: &StreamMetrics) -> Result<HealthDecision, HealthError> {
        validate_metrics(metrics)?;

        let mut score: i32 = 100;
        let mut reasons = Vec::new();

        if metrics.input_bitrate_kbps == 0 {
            reasons.push("input bitrate is zero".to_string());
            return Ok(self.commit(HealthState::Offline, 0, reasons));
        }

        if metrics.clean_frame_age_ms >= self.thresholds.brb_clean_frame_age_ms {
            reasons.push(format!(
                "no clean frame for {}ms",
                metrics.clean_frame_age_ms
            ));
            return Ok(self.commit(HealthState::Brb, 5, reasons));
        }

        if metrics.frozen_frame_ms >= self.thresholds.frozen_frame_brb_ms {
            reasons.push(format!("frozen frame for {}ms", metrics.frozen_frame_ms));
            return Ok(self.commit(HealthState::Brb, 10, reasons));
        }

        if metrics.input_bitrate_kbps < self.thresholds.min_brownout_bitrate_kbps {
            score -= 65;
            reasons.push(format!(
                "input bitrate {}kbps is below brownout threshold {}kbps",
                metrics.input_bitrate_kbps, self.thresholds.min_brownout_bitrate_kbps
            ));
        } else if metrics.input_bitrate_kbps < self.thresholds.min_healthy_bitrate_kbps {
            score -= 30;
            reasons.push(format!(
                "input bitrate {}kbps is below healthy threshold {}kbps",
                metrics.input_bitrate_kbps, self.thresholds.min_healthy_bitrate_kbps
            ));
        }

        if metrics.packet_loss_percent >= self.thresholds.brownout_packet_loss_percent {
            score -= 55;
            reasons.push(format!(
                "packet loss {:.1}% exceeds brownout threshold {:.1}%",
                metrics.packet_loss_percent, self.thresholds.brownout_packet_loss_percent
            ));
        } else if metrics.packet_loss_percent >= self.thresholds.degraded_packet_loss_percent {
            score -= 25;
            reasons.push(format!(
                "packet loss {:.1}% exceeds degraded threshold {:.1}%",
                metrics.packet_loss_percent, self.thresholds.degraded_packet_loss_percent
            ));
        }

        if metrics.rtt_ms >= self.thresholds.brownout_rtt_ms {
            score -= 35;
            reasons.push(format!("RTT {}ms is in brownout range", metrics.rtt_ms));
        } else if metrics.rtt_ms >= self.thresholds.degraded_rtt_ms {
            score -= 15;
            reasons.push(format!("RTT {}ms is degraded", metrics.rtt_ms));
        }

        if metrics.retransmits_per_sec > 50 {
            score -= 25;
            reasons.push(format!(
                "high retransmit pressure: {}/sec",
                metrics.retransmits_per_sec
            ));
        }

        if metrics.connected_links == 0 {
            score -= 45;
            reasons.push("no connected contribution links".to_string());
        }

        if metrics.obs_dropped_frames_per_min > 120 {
            score -= 25;
            reasons.push(format!(
                "OBS dropped {} frames/min",
                metrics.obs_dropped_frames_per_min
            ));
        }

        if metrics.audio_silence_ms >= self.thresholds.audio_silence_warn_ms {
            score -= 15;
            reasons.push(format!("audio silence for {}ms", metrics.audio_silence_ms));
        }

        let score = score.clamp(0, 100) as u8;
        let raw_state = if score <= 20 {
            HealthState::Brb
        } else if score <= 45 {
            HealthState::Brownout
        } else if score <= 75 {
            HealthState::Degraded
        } else {
            HealthState::Healthy
        };

        let state = self.apply_recovery_hysteresis(raw_state, metrics.timestamp_ms);
        if reasons.is_empty() {
            reasons.push("all metrics are within thresholds".to_string());
        }

        Ok(self.commit(state, score, reasons))
    }

    fn apply_recovery_hysteresis(
        &mut self,
        raw_state: HealthState,
        timestamp_ms: u64,
    ) -> HealthState {
        let was_bad = matches!(
            self.previous_state,
            HealthState::Brownout
                | HealthState::Brb
                | HealthState::Offline
                | HealthState::BackupIngest
        );

        if matches!(
            raw_state,
            HealthState::Brownout | HealthState::Brb | HealthState::Offline
        ) {
            self.last_bad_timestamp_ms = Some(timestamp_ms);
            return raw_state;
        }

        if raw_state == HealthState::Healthy && was_bad {
            if let Some(last_bad) = self.last_bad_timestamp_ms {
                let stable_for_ms = timestamp_ms.saturating_sub(last_bad);
                if stable_for_ms < u64::from(self.thresholds.recovery_hold_ms) {
                    return HealthState::RecoveryPending;
                }
            }
        }

        raw_state
    }

    fn commit(&mut self, state: HealthState, score: u8, reasons: Vec<String>) -> HealthDecision {
        self.previous_state = state;
        HealthDecision {
            state,
            score,
            reasons,
            recommended_scene: scene_for_state(state),
        }
    }
}

impl Default for HealthEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Maps health state to the default scene role.
#[must_use]
pub const fn scene_for_state(state: HealthState) -> SceneRole {
    match state {
        HealthState::Healthy | HealthState::Degraded => SceneRole::Live,
        HealthState::Brownout | HealthState::RecoveryPending => SceneRole::LowSignal,
        HealthState::Brb | HealthState::Offline => SceneRole::Brb,
        HealthState::BackupIngest => SceneRole::BackupFeed,
    }
}

fn validate_metrics(metrics: &StreamMetrics) -> Result<(), HealthError> {
    if !(0.0..=100.0).contains(&metrics.packet_loss_percent) {
        return Err(HealthError::InvalidMetrics("packet loss must be 0..=100"));
    }
    if metrics.encoder_fps.is_sign_negative() {
        return Err(HealthError::InvalidMetrics(
            "encoder fps cannot be negative",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn healthy_metrics_choose_live() -> Result<(), HealthError> {
        let mut engine = HealthEngine::new();
        let decision = engine.evaluate(&StreamMetrics::default())?;
        assert_eq!(decision.state, HealthState::Healthy);
        assert_eq!(decision.recommended_scene, SceneRole::Live);
        Ok(())
    }

    #[test]
    fn low_bitrate_without_disconnect_is_brownout() -> Result<(), HealthError> {
        let mut engine = HealthEngine::new();
        let metrics = StreamMetrics {
            input_bitrate_kbps: 900,
            timestamp_ms: 1_000,
            ..StreamMetrics::default()
        };
        let decision = engine.evaluate(&metrics)?;
        assert_eq!(decision.state, HealthState::Brownout);
        assert_eq!(decision.recommended_scene, SceneRole::LowSignal);
        Ok(())
    }

    #[test]
    fn missing_clean_frame_goes_brb() -> Result<(), HealthError> {
        let mut engine = HealthEngine::new();
        let metrics = StreamMetrics {
            clean_frame_age_ms: 15_000,
            timestamp_ms: 1_000,
            ..StreamMetrics::default()
        };
        let decision = engine.evaluate(&metrics)?;
        assert_eq!(decision.state, HealthState::Brb);
        assert_eq!(decision.recommended_scene, SceneRole::Brb);
        Ok(())
    }

    #[test]
    fn invalid_packet_loss_is_rejected() {
        let mut engine = HealthEngine::new();
        let metrics = StreamMetrics {
            packet_loss_percent: 101.0,
            ..StreamMetrics::default()
        };
        assert!(engine.evaluate(&metrics).is_err());
    }
}
