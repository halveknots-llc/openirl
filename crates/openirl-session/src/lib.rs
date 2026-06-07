//! Stateful local session history for the OpenIRL agent.
//!
//! This crate intentionally stays pure Rust and media-tool agnostic. It records
//! metrics, health decisions, and operator actions so the agent can expose a
//! useful dashboard and diagnostics report before real SRT/SRTLA/OBS adapters
//! are wired in.

use openirl_core::{HealthDecision, SceneRole, StreamMetrics};
use openirl_diagnostics::{ReportBuilder, SessionSample, StreamReport};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

/// Default maximum number of retained samples/events in memory.
pub const DEFAULT_HISTORY_LIMIT: usize = 512;

/// A timeline event produced by the local control plane.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionEvent {
    /// Event ID.
    pub id: Uuid,
    /// Event creation timestamp.
    pub created_at: OffsetDateTime,
    /// Event kind.
    pub kind: SessionEventKind,
    /// Human-readable message.
    pub message: String,
}

impl SessionEvent {
    /// Creates a new session event.
    #[must_use]
    pub fn new(kind: SessionEventKind, message: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            created_at: OffsetDateTime::now_utc(),
            kind,
            message: message.into(),
        }
    }
}

/// Event classes shown in the dashboard timeline.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SessionEventKind {
    /// Agent started.
    AgentStarted,
    /// Metrics sample evaluated.
    MetricsEvaluated,
    /// Health state changed.
    HealthStateChanged,
    /// OBS scene switch requested.
    SceneSwitch,
    /// Start streaming requested.
    StartStreaming,
    /// Stop streaming requested.
    StopStreaming,
    /// Manual privacy/BRB/back-to-live operation.
    OperatorControl,
    /// Diagnostics report generated.
    Diagnostics,
}

/// In-memory session store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStore {
    id: Uuid,
    started_at: OffsetDateTime,
    history_limit: usize,
    samples: Vec<SessionSample>,
    events: Vec<SessionEvent>,
    current_decision: HealthDecision,
    last_scene: SceneRole,
}

impl SessionStore {
    /// Creates a new store with the default history limit.
    #[must_use]
    pub fn new() -> Self {
        Self::with_limit(DEFAULT_HISTORY_LIMIT)
    }

    /// Creates a new store with a bounded history length.
    #[must_use]
    pub fn with_limit(history_limit: usize) -> Self {
        let limit = history_limit.max(1);
        let mut store = Self {
            id: Uuid::new_v4(),
            started_at: OffsetDateTime::now_utc(),
            history_limit: limit,
            samples: Vec::new(),
            events: Vec::new(),
            current_decision: HealthDecision::healthy(),
            last_scene: SceneRole::Live,
        };
        store.push_event(SessionEvent::new(
            SessionEventKind::AgentStarted,
            "OpenIRL agent session initialized",
        ));
        store
    }

    /// Records an evaluated sample.
    pub fn push_sample(&mut self, metrics: StreamMetrics, decision: HealthDecision) {
        let previous_state = self.current_decision.state;
        self.last_scene = decision.recommended_scene;
        self.current_decision = decision.clone();
        self.push_bounded_sample(SessionSample { metrics, decision });
        self.push_event(SessionEvent::new(
            SessionEventKind::MetricsEvaluated,
            format!(
                "metrics evaluated: state={}, score={}",
                self.current_decision.state, self.current_decision.score
            ),
        ));
        if previous_state != self.current_decision.state {
            self.push_event(SessionEvent::new(
                SessionEventKind::HealthStateChanged,
                format!(
                    "health changed: {previous_state} -> {}",
                    self.current_decision.state
                ),
            ));
        }
    }

    /// Records a requested scene switch.
    pub fn record_scene_switch(&mut self, role: SceneRole) {
        self.last_scene = role;
        self.push_event(SessionEvent::new(
            SessionEventKind::SceneSwitch,
            format!("requested scene role: {role}"),
        ));
    }

    /// Records a control-plane operation.
    pub fn record_control(&mut self, kind: SessionEventKind, message: impl Into<String>) {
        self.push_event(SessionEvent::new(kind, message));
    }

    /// Returns a serializable snapshot for the dashboard.
    #[must_use]
    pub fn snapshot(&self) -> SessionSnapshot {
        SessionSnapshot {
            id: self.id,
            started_at: self.started_at,
            sample_count: self.samples.len(),
            event_count: self.events.len(),
            current_decision: self.current_decision.clone(),
            last_scene: self.last_scene,
            recent_samples: self.samples.clone(),
            recent_events: self.events.clone(),
        }
    }

    /// Builds the current diagnostics report.
    #[must_use]
    pub fn report(&self) -> StreamReport {
        let mut builder = ReportBuilder::new();
        for sample in &self.samples {
            builder.push(sample.clone());
        }
        builder.build()
    }

    /// Clears metrics samples but keeps the current session ID and an audit event.
    pub fn clear_samples(&mut self) {
        self.samples.clear();
        self.current_decision = HealthDecision::healthy();
        self.last_scene = SceneRole::Live;
        self.push_event(SessionEvent::new(
            SessionEventKind::Diagnostics,
            "metrics history cleared",
        ));
    }

    fn push_bounded_sample(&mut self, sample: SessionSample) {
        self.samples.push(sample);
        trim_to_limit(&mut self.samples, self.history_limit);
    }

    fn push_event(&mut self, event: SessionEvent) {
        self.events.push(event);
        trim_to_limit(&mut self.events, self.history_limit);
    }
}

impl Default for SessionStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Dashboard snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionSnapshot {
    /// Session ID.
    pub id: Uuid,
    /// Session start timestamp.
    pub started_at: OffsetDateTime,
    /// Number of retained samples.
    pub sample_count: usize,
    /// Number of retained events.
    pub event_count: usize,
    /// Most recent health decision.
    pub current_decision: HealthDecision,
    /// Last requested scene role.
    pub last_scene: SceneRole,
    /// Recent sample history.
    pub recent_samples: Vec<SessionSample>,
    /// Recent event history.
    pub recent_events: Vec<SessionEvent>,
}

fn trim_to_limit<T>(items: &mut Vec<T>, limit: usize) {
    if items.len() > limit {
        let excess = items.len() - limit;
        items.drain(0..excess);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openirl_core::HealthState;

    #[test]
    fn bounded_history_keeps_latest_sample() {
        let mut store = SessionStore::with_limit(1);
        store.push_sample(StreamMetrics::default(), HealthDecision::healthy());
        store.push_sample(
            StreamMetrics {
                input_bitrate_kbps: 1000,
                ..StreamMetrics::default()
            },
            HealthDecision {
                state: HealthState::Brownout,
                score: 35,
                reasons: vec!["test".to_string()],
                recommended_scene: SceneRole::LowSignal,
            },
        );
        let snapshot = store.snapshot();
        assert_eq!(snapshot.recent_samples.len(), 1);
        assert_eq!(snapshot.current_decision.state, HealthState::Brownout);
    }

    #[test]
    fn report_uses_recorded_samples() {
        let mut store = SessionStore::new();
        store.push_sample(StreamMetrics::default(), HealthDecision::healthy());
        let report = store.report();
        assert_eq!(report.sample_count, 1);
    }
}
