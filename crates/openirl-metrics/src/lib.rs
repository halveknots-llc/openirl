//! Relay/router metric ingestion for OpenIRL.
//!
//! feature areas keeps media routing process-bound and reads exported metrics from
//! MediaMTX-compatible Prometheus text, relay process logs, or deterministic
//! demo samples. The reducer converts those observations into the shared
//! `StreamMetrics` model consumed by the brownout health engine.

use openirl_core::StreamMetrics;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, str::FromStr};
use thiserror::Error;
use time::OffsetDateTime;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::{Duration, timeout},
};

const DEFAULT_ACTIVE_UNKNOWN_BITRATE_KBPS: u32 = 3_500;
const DEFAULT_OUTPUT_UNKNOWN_BITRATE_KBPS: u32 = 5_000;
const DEFAULT_RTT_MS: u32 = 80;
const DEFAULT_ENCODER_FPS: f32 = 30.0;
const MAX_HTTP_RESPONSE_BYTES: usize = 2 * 1024 * 1024;

/// Metric ingestion errors.
#[derive(Debug, Error)]
pub enum MetricsError {
    /// Unsupported or invalid endpoint URL.
    #[error("invalid metrics endpoint: {0}")]
    InvalidEndpoint(String),
    /// Network error while polling an HTTP endpoint.
    #[error("metrics HTTP error: {0}")]
    Http(#[from] std::io::Error),
    /// HTTP response status was not successful.
    #[error("metrics endpoint returned non-success status: {0}")]
    HttpStatus(String),
    /// Request timed out.
    #[error("metrics request timed out after {0}ms")]
    Timeout(u64),
    /// Prometheus text could not be parsed.
    #[error("prometheus parse error at line {line}: {message}")]
    PrometheusParse {
        /// One-based line number.
        line: usize,
        /// Human readable parse message.
        message: String,
    },
}

/// One parsed Prometheus text sample.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrometheusSample {
    /// Metric name.
    pub name: String,
    /// Metric labels.
    pub labels: BTreeMap<String, String>,
    /// Numeric sample value.
    pub value: f64,
    /// Optional unix timestamp from exposition text.
    pub timestamp: Option<i64>,
}

/// Parsed Prometheus text document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PrometheusDocument {
    /// Parsed samples.
    pub samples: Vec<PrometheusSample>,
}

impl PrometheusDocument {
    /// Returns samples matching a metric name.
    pub fn named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a PrometheusSample> + 'a {
        self.samples
            .iter()
            .filter(move |sample| sample.name == name)
    }
}

/// Summary reduced from router/exporter metrics before conversion to health metrics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouterMetricsSnapshot {
    /// Wall-clock sample timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Active path count.
    pub active_paths: u32,
    /// Active SRT connection count.
    pub srt_connections: u32,
    /// Active RTMP connection count.
    pub rtmp_connections: u32,
    /// Active WebRTC session count.
    pub webrtc_sessions: u32,
    /// Total bytes received by router paths or inbound connections.
    pub bytes_received_total: u64,
    /// Total bytes sent by router paths or outbound connections.
    pub bytes_sent_total: u64,
    /// Names of active paths when available.
    pub active_path_names: Vec<String>,
}

/// Metric conversion output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricsIngestResult {
    /// Source label.
    pub source: String,
    /// Router snapshot that fed the reducer.
    pub router: Option<RouterMetricsSnapshot>,
    /// Health-engine input metrics.
    pub stream_metrics: StreamMetrics,
    /// Non-fatal warnings about missing or inferred values.
    pub warnings: Vec<String>,
}

/// Dashboard-safe snapshot of the metrics accumulator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricsAccumulatorSnapshot {
    /// Previous router snapshot used for bitrate deltas.
    pub previous_router: Option<RouterMetricsSnapshot>,
    /// Most recent stream metrics.
    pub last_stream_metrics: Option<StreamMetrics>,
    /// Most recent source label.
    pub last_source: Option<String>,
    /// Most recent warnings.
    pub last_warnings: Vec<String>,
    /// Number of samples ingested since process start.
    pub ingested_samples: u64,
}

/// Stateful reducer for deriving bitrate deltas from cumulative router metrics.
#[derive(Debug, Clone, Default)]
pub struct MetricsAccumulator {
    previous_router: Option<RouterMetricsSnapshot>,
    last_stream_metrics: Option<StreamMetrics>,
    last_source: Option<String>,
    last_warnings: Vec<String>,
    ingested_samples: u64,
}

impl MetricsAccumulator {
    /// Creates an empty accumulator.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Ingests MediaMTX/Prometheus text and returns health-engine metrics.
    pub fn ingest_prometheus_text(
        &mut self,
        text: &str,
        timestamp_ms: u64,
    ) -> Result<MetricsIngestResult, MetricsError> {
        let document = parse_prometheus_text(text)?;
        let router = reduce_mediamtx_document(&document, timestamp_ms);
        let (stream_metrics, warnings) = router_snapshot_to_stream_metrics(
            &router,
            self.previous_router.as_ref(),
            self.last_stream_metrics.as_ref(),
        );
        self.previous_router = Some(router.clone());
        self.remember(
            "prometheus-mediamtx",
            stream_metrics.clone(),
            warnings.clone(),
        );
        Ok(MetricsIngestResult {
            source: "prometheus-mediamtx".to_string(),
            router: Some(router),
            stream_metrics,
            warnings,
        })
    }

    /// Ingests one SRT/SRTLA-style status line.
    pub fn ingest_srtla_log_line(&mut self, line: &str, timestamp_ms: u64) -> MetricsIngestResult {
        let mut metrics = self.last_stream_metrics.clone().unwrap_or_default();
        metrics.timestamp_ms = timestamp_ms;

        let parsed = parse_key_value_status_line(line);
        let mut warnings = Vec::new();
        apply_status_pairs_to_metrics(&parsed, &mut metrics, &mut warnings);
        if parsed.is_empty() {
            warnings.push("no key=value SRT/SRTLA stats were detected in the line".to_string());
        }

        self.remember("srtla-log", metrics.clone(), warnings.clone());
        MetricsIngestResult {
            source: "srtla-log".to_string(),
            router: None,
            stream_metrics: metrics,
            warnings,
        }
    }

    /// Generates a deterministic demo sample for dashboards and tests.
    pub fn ingest_demo_sample(&mut self, timestamp_ms: u64) -> MetricsIngestResult {
        let phase = self.ingested_samples % 6;
        let mut metrics = StreamMetrics {
            timestamp_ms,
            ..StreamMetrics::default()
        };
        let source = match phase {
            0 | 1 => "demo-healthy",
            2 => {
                metrics.input_bitrate_kbps = 2_000;
                metrics.packet_loss_percent = 3.5;
                metrics.rtt_ms = 360;
                "demo-degraded"
            }
            3 => {
                metrics.input_bitrate_kbps = 850;
                metrics.packet_loss_percent = 9.0;
                metrics.rtt_ms = 740;
                metrics.retransmits_per_sec = 70;
                "demo-brownout"
            }
            4 => {
                metrics.input_bitrate_kbps = 0;
                metrics.connected_links = 0;
                metrics.clean_frame_age_ms = 15_000;
                "demo-offline"
            }
            _ => {
                metrics.input_bitrate_kbps = 4_800;
                metrics.packet_loss_percent = 0.2;
                metrics.rtt_ms = 95;
                "demo-recovery"
            }
        };
        let warnings =
            vec!["demo sample generated locally; no media router was queried".to_string()];
        self.remember(source, metrics.clone(), warnings.clone());
        MetricsIngestResult {
            source: source.to_string(),
            router: None,
            stream_metrics: metrics,
            warnings,
        }
    }

    /// Returns a dashboard snapshot.
    #[must_use]
    pub fn snapshot(&self) -> MetricsAccumulatorSnapshot {
        MetricsAccumulatorSnapshot {
            previous_router: self.previous_router.clone(),
            last_stream_metrics: self.last_stream_metrics.clone(),
            last_source: self.last_source.clone(),
            last_warnings: self.last_warnings.clone(),
            ingested_samples: self.ingested_samples,
        }
    }

    fn remember(&mut self, source: &str, stream_metrics: StreamMetrics, warnings: Vec<String>) {
        self.last_source = Some(source.to_string());
        self.last_stream_metrics = Some(stream_metrics);
        self.last_warnings = warnings;
        self.ingested_samples = self.ingested_samples.saturating_add(1);
    }
}

/// Parses Prometheus exposition text into samples.
pub fn parse_prometheus_text(text: &str) -> Result<PrometheusDocument, MetricsError> {
    let mut samples = Vec::new();
    for (index, raw_line) in text.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        samples.push(parse_prometheus_line(line, line_number)?);
    }
    Ok(PrometheusDocument { samples })
}

/// Reduces MediaMTX-compatible Prometheus samples into a router snapshot.
#[must_use]
pub fn reduce_mediamtx_document(
    document: &PrometheusDocument,
    timestamp_ms: u64,
) -> RouterMetricsSnapshot {
    let path_rx = sum_metric(document, "paths_bytes_received");
    let path_tx = sum_metric(document, "paths_bytes_sent");
    let srt_rx = sum_metric(document, "srt_conns_bytes_received");
    let srt_tx = sum_metric(document, "srt_conns_bytes_sent");
    let rtmp_rx = sum_metric(document, "rtmp_conns_bytes_received")
        .saturating_add(sum_metric(document, "rtmps_conns_bytes_received"));
    let rtmp_tx = sum_metric(document, "rtmp_conns_bytes_sent")
        .saturating_add(sum_metric(document, "rtmps_conns_bytes_sent"));
    let webrtc_rx = sum_metric(document, "webrtc_sessions_bytes_received");
    let webrtc_tx = sum_metric(document, "webrtc_sessions_bytes_sent");

    let active_path_names = active_paths(document);
    let active_paths = u32::try_from(active_path_names.len()).unwrap_or(u32::MAX);
    let srt_connections = active_count(document, "srt_conns");
    let rtmp_connections =
        active_count(document, "rtmp_conns").saturating_add(active_count(document, "rtmps_conns"));
    let webrtc_sessions = active_count(document, "webrtc_sessions");

    RouterMetricsSnapshot {
        timestamp_ms,
        active_paths,
        srt_connections,
        rtmp_connections,
        webrtc_sessions,
        bytes_received_total: choose_nonzero(
            path_rx,
            srt_rx.saturating_add(rtmp_rx).saturating_add(webrtc_rx),
        ),
        bytes_sent_total: choose_nonzero(
            path_tx,
            srt_tx.saturating_add(rtmp_tx).saturating_add(webrtc_tx),
        ),
        active_path_names,
    }
}

/// Converts a router snapshot to health-engine stream metrics.
#[must_use]
pub fn router_snapshot_to_stream_metrics(
    current: &RouterMetricsSnapshot,
    previous: Option<&RouterMetricsSnapshot>,
    fallback: Option<&StreamMetrics>,
) -> (StreamMetrics, Vec<String>) {
    let mut warnings = Vec::new();
    let connected_links = connected_links_from_snapshot(current);
    let (input_bitrate_kbps, output_bitrate_kbps) = match previous {
        Some(previous) => {
            let elapsed_ms = current
                .timestamp_ms
                .saturating_sub(previous.timestamp_ms)
                .max(1);
            (
                bitrate_kbps_from_bytes(
                    current
                        .bytes_received_total
                        .saturating_sub(previous.bytes_received_total),
                    elapsed_ms,
                ),
                bitrate_kbps_from_bytes(
                    current
                        .bytes_sent_total
                        .saturating_sub(previous.bytes_sent_total),
                    elapsed_ms,
                ),
            )
        }
        None => {
            warnings.push(
                "first router sample has no previous byte baseline; bitrate is inferred"
                    .to_string(),
            );
            if connected_links == 0 && current.active_paths == 0 {
                (0, 0)
            } else {
                (
                    fallback
                        .map(|metrics| metrics.input_bitrate_kbps)
                        .filter(|value| *value > 0)
                        .unwrap_or(DEFAULT_ACTIVE_UNKNOWN_BITRATE_KBPS),
                    fallback
                        .map(|metrics| metrics.output_bitrate_kbps)
                        .filter(|value| *value > 0)
                        .unwrap_or(DEFAULT_OUTPUT_UNKNOWN_BITRATE_KBPS),
                )
            }
        }
    };

    let clean_frame_age_ms = if input_bitrate_kbps == 0 { 15_000 } else { 0 };
    let metrics = StreamMetrics {
        input_bitrate_kbps,
        output_bitrate_kbps,
        packet_loss_percent: fallback.map_or(0.0, |metrics| metrics.packet_loss_percent),
        retransmits_per_sec: fallback.map_or(0, |metrics| metrics.retransmits_per_sec),
        rtt_ms: fallback.map_or(DEFAULT_RTT_MS, |metrics| metrics.rtt_ms),
        jitter_ms: fallback.map_or(10, |metrics| metrics.jitter_ms),
        connected_links,
        obs_dropped_frames_per_min: fallback
            .map_or(0, |metrics| metrics.obs_dropped_frames_per_min),
        encoder_fps: fallback.map_or(DEFAULT_ENCODER_FPS, |metrics| metrics.encoder_fps),
        audio_silence_ms: fallback.map_or(0, |metrics| metrics.audio_silence_ms),
        frozen_frame_ms: 0,
        reconnect_count: fallback.map_or(0, |metrics| metrics.reconnect_count),
        clean_frame_age_ms,
        timestamp_ms: current.timestamp_ms,
    };

    (metrics, warnings)
}

/// Polls a plain HTTP metrics endpoint and returns the response body.
pub async fn poll_http_text(endpoint: &str, timeout_ms: u64) -> Result<String, MetricsError> {
    let endpoint = HttpEndpoint::parse(endpoint)?;
    let request_timeout = Duration::from_millis(timeout_ms.max(1));
    match timeout(request_timeout, poll_http_text_inner(&endpoint)).await {
        Ok(result) => result,
        Err(_) => Err(MetricsError::Timeout(timeout_ms)),
    }
}

fn parse_prometheus_line(line: &str, line_number: usize) -> Result<PrometheusSample, MetricsError> {
    let (metric_expr, rest) =
        split_once_whitespace(line).ok_or_else(|| MetricsError::PrometheusParse {
            line: line_number,
            message: "sample is missing a value".to_string(),
        })?;
    let mut rest_parts = rest.split_whitespace();
    let value_text = rest_parts
        .next()
        .ok_or_else(|| MetricsError::PrometheusParse {
            line: line_number,
            message: "sample is missing a value".to_string(),
        })?;
    let value = f64::from_str(value_text).map_err(|error| MetricsError::PrometheusParse {
        line: line_number,
        message: format!("invalid sample value: {error}"),
    })?;
    let timestamp = rest_parts
        .next()
        .map(i64::from_str)
        .transpose()
        .map_err(|error| MetricsError::PrometheusParse {
            line: line_number,
            message: format!("invalid sample timestamp: {error}"),
        })?;
    let (name, labels) = parse_metric_expr(metric_expr, line_number)?;
    Ok(PrometheusSample {
        name,
        labels,
        value,
        timestamp,
    })
}

fn parse_metric_expr(
    metric_expr: &str,
    line_number: usize,
) -> Result<(String, BTreeMap<String, String>), MetricsError> {
    if let Some(open_index) = metric_expr.find('{') {
        let close_index = metric_expr
            .rfind('}')
            .ok_or_else(|| MetricsError::PrometheusParse {
                line: line_number,
                message: "metric labels are missing closing brace".to_string(),
            })?;
        if close_index <= open_index {
            return Err(MetricsError::PrometheusParse {
                line: line_number,
                message: "metric label braces are malformed".to_string(),
            });
        }
        let name = metric_expr[..open_index].to_string();
        let labels_text = &metric_expr[(open_index + 1)..close_index];
        Ok((name, parse_labels(labels_text, line_number)?))
    } else {
        Ok((metric_expr.to_string(), BTreeMap::new()))
    }
}

fn parse_labels(
    labels_text: &str,
    line_number: usize,
) -> Result<BTreeMap<String, String>, MetricsError> {
    let mut labels = BTreeMap::new();
    let mut key = String::new();
    let mut value = String::new();
    let mut reading_key = true;
    let mut in_quotes = false;
    let mut escape_next = false;

    for character in labels_text.chars().chain(std::iter::once(',')) {
        if escape_next {
            value.push(character);
            escape_next = false;
            continue;
        }
        match character {
            '\\' if in_quotes => escape_next = true,
            '"' => in_quotes = !in_quotes,
            '=' if reading_key && !in_quotes => reading_key = false,
            ',' if !in_quotes => {
                if !key.trim().is_empty() {
                    if reading_key {
                        return Err(MetricsError::PrometheusParse {
                            line: line_number,
                            message: "label is missing a value".to_string(),
                        });
                    }
                    labels.insert(
                        key.trim().to_string(),
                        value.trim().trim_matches('"').to_string(),
                    );
                }
                key.clear();
                value.clear();
                reading_key = true;
            }
            _ if reading_key => key.push(character),
            _ => value.push(character),
        }
    }

    if in_quotes {
        return Err(MetricsError::PrometheusParse {
            line: line_number,
            message: "unterminated quoted label value".to_string(),
        });
    }

    Ok(labels)
}

fn split_once_whitespace(line: &str) -> Option<(&str, &str)> {
    for (index, character) in line.char_indices() {
        if character.is_whitespace() {
            let rest = line[index..].trim_start();
            return Some((&line[..index], rest));
        }
    }
    None
}

fn sum_metric(document: &PrometheusDocument, name: &str) -> u64 {
    document
        .named(name)
        .filter_map(|sample| finite_nonnegative_u64(sample.value))
        .fold(0_u64, u64::saturating_add)
}

fn active_count(document: &PrometheusDocument, name: &str) -> u32 {
    let count = document
        .named(name)
        .filter(|sample| sample.value > 0.0)
        .count();
    u32::try_from(count).unwrap_or(u32::MAX)
}

fn active_paths(document: &PrometheusDocument) -> Vec<String> {
    document
        .named("paths")
        .filter(|sample| sample.value > 0.0)
        .filter(|sample| {
            sample
                .labels
                .get("state")
                .is_none_or(|state| state == "ready" || state == "publish")
        })
        .filter_map(|sample| sample.labels.get("name").cloned())
        .collect()
}

fn finite_nonnegative_u64(value: f64) -> Option<u64> {
    if !value.is_finite() || value < 0.0 {
        return None;
    }
    if value > u64::MAX as f64 {
        Some(u64::MAX)
    } else {
        Some(value as u64)
    }
}

fn choose_nonzero(primary: u64, fallback: u64) -> u64 {
    if primary == 0 { fallback } else { primary }
}

fn connected_links_from_snapshot(snapshot: &RouterMetricsSnapshot) -> u8 {
    let count = snapshot
        .srt_connections
        .saturating_add(snapshot.rtmp_connections)
        .saturating_add(snapshot.webrtc_sessions)
        .max(snapshot.active_paths);
    u8::try_from(count).unwrap_or(u8::MAX)
}

fn bitrate_kbps_from_bytes(delta_bytes: u64, elapsed_ms: u64) -> u32 {
    let kbps = delta_bytes.saturating_mul(8) / elapsed_ms.max(1);
    u32::try_from(kbps).unwrap_or(u32::MAX)
}

fn parse_key_value_status_line(line: &str) -> BTreeMap<String, String> {
    let mut pairs = BTreeMap::new();
    for raw_token in line.split(|character: char| character.is_whitespace() || character == ',') {
        let token = raw_token.trim_matches(|character: char| character == ';' || character == ',');
        if let Some((key, value)) = token.split_once('=') {
            let cleaned_key = key
                .trim()
                .trim_matches(|character: char| character == '[' || character == ']');
            let cleaned_value = value
                .trim()
                .trim_matches(|character: char| character == '[' || character == ']');
            if !cleaned_key.is_empty() && !cleaned_value.is_empty() {
                pairs.insert(cleaned_key.to_ascii_lowercase(), cleaned_value.to_string());
            }
        }
    }
    pairs
}

fn apply_status_pairs_to_metrics(
    pairs: &BTreeMap<String, String>,
    metrics: &mut StreamMetrics,
    warnings: &mut Vec<String>,
) {
    apply_u32_any(
        pairs,
        &["bitrate", "input_bitrate", "input_bitrate_kbps", "kbps"],
        |value| {
            metrics.input_bitrate_kbps = value;
        },
    );
    apply_u32_any(pairs, &["output_bitrate", "output_bitrate_kbps"], |value| {
        metrics.output_bitrate_kbps = value;
    });
    apply_f32_any(
        pairs,
        &["loss", "packet_loss", "packet_loss_percent", "loss_percent"],
        |value| {
            metrics.packet_loss_percent = value;
        },
    );
    apply_u32_any(pairs, &["rtt", "rtt_ms"], |value| {
        metrics.rtt_ms = value;
    });
    apply_u32_any(pairs, &["jitter", "jitter_ms"], |value| {
        metrics.jitter_ms = value;
    });
    apply_u32_any(
        pairs,
        &["retransmits", "retrans", "retransmits_per_sec"],
        |value| {
            metrics.retransmits_per_sec = value;
        },
    );
    apply_u8_any(pairs, &["links", "connected_links", "modems"], |value| {
        metrics.connected_links = value;
    });
    apply_f32_any(pairs, &["fps", "encoder_fps"], |value| {
        metrics.encoder_fps = value;
    });

    if metrics.input_bitrate_kbps == 0 {
        metrics.clean_frame_age_ms = metrics.clean_frame_age_ms.max(15_000);
    }
    if metrics.packet_loss_percent > 100.0 {
        metrics.packet_loss_percent = 100.0;
        warnings.push("packet loss was clamped to 100%".to_string());
    }
}

fn apply_u32_any<F>(pairs: &BTreeMap<String, String>, keys: &[&str], mut apply: F)
where
    F: FnMut(u32),
{
    for key in keys {
        if let Some(value) = pairs
            .get(*key)
            .and_then(|value| parse_u32_with_units(value))
        {
            apply(value);
            return;
        }
    }
}

fn apply_u8_any<F>(pairs: &BTreeMap<String, String>, keys: &[&str], mut apply: F)
where
    F: FnMut(u8),
{
    for key in keys {
        if let Some(value) = pairs
            .get(*key)
            .and_then(|value| parse_u32_with_units(value))
        {
            apply(u8::try_from(value).unwrap_or(u8::MAX));
            return;
        }
    }
}

fn apply_f32_any<F>(pairs: &BTreeMap<String, String>, keys: &[&str], mut apply: F)
where
    F: FnMut(f32),
{
    for key in keys {
        if let Some(value) = pairs
            .get(*key)
            .and_then(|value| parse_f32_with_units(value))
        {
            apply(value);
            return;
        }
    }
}

fn parse_u32_with_units(value: &str) -> Option<u32> {
    parse_f32_with_units(value).map(|number| {
        if number.is_sign_negative() {
            0
        } else if number > u32::MAX as f32 {
            u32::MAX
        } else {
            number.round() as u32
        }
    })
}

fn parse_f32_with_units(value: &str) -> Option<f32> {
    let mut cleaned = value.trim().to_ascii_lowercase();
    for suffix in ["kbps", "kbit/s", "ms", "%", "fps", "pps", "s"] {
        if let Some(stripped) = cleaned.strip_suffix(suffix) {
            cleaned = stripped.trim().to_string();
            break;
        }
    }
    cleaned
        .parse::<f32>()
        .ok()
        .filter(|value| value.is_finite())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct HttpEndpoint {
    host: String,
    port: u16,
    path: String,
}

impl HttpEndpoint {
    fn parse(endpoint: &str) -> Result<Self, MetricsError> {
        let trimmed = endpoint.trim();
        let without_scheme = trimmed.strip_prefix("http://").ok_or_else(|| {
            MetricsError::InvalidEndpoint(
                "only plain local http:// metrics URLs are supported".to_string(),
            )
        })?;
        let (authority, path) = match without_scheme.split_once('/') {
            Some((authority, path)) => (authority, format!("/{path}")),
            None => (without_scheme, "/".to_string()),
        };
        if authority.is_empty() {
            return Err(MetricsError::InvalidEndpoint("missing host".to_string()));
        }
        let (host, port) = parse_authority(authority)?;
        Ok(Self { host, port, path })
    }
}

fn parse_authority(authority: &str) -> Result<(String, u16), MetricsError> {
    if let Some((host, port_text)) = authority.rsplit_once(':') {
        if host.contains(']') && !host.ends_with(']') {
            return Ok((authority.to_string(), 80));
        }
        let port = port_text.parse::<u16>().map_err(|error| {
            MetricsError::InvalidEndpoint(format!("invalid port in metrics URL: {error}"))
        })?;
        Ok((
            host.trim_matches(|character| character == '[' || character == ']')
                .to_string(),
            port,
        ))
    } else {
        Ok((authority.to_string(), 80))
    }
}

async fn poll_http_text_inner(endpoint: &HttpEndpoint) -> Result<String, MetricsError> {
    let mut stream = TcpStream::connect((endpoint.host.as_str(), endpoint.port)).await?;
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}:{}\r\nUser-Agent: openirl-metrics/0.1\r\nAccept: text/plain,*/*\r\nConnection: close\r\n\r\n",
        endpoint.path, endpoint.host, endpoint.port
    );
    stream.write_all(request.as_bytes()).await?;
    let mut buffer = Vec::new();
    stream.read_to_end(&mut buffer).await?;
    if buffer.len() > MAX_HTTP_RESPONSE_BYTES {
        buffer.truncate(MAX_HTTP_RESPONSE_BYTES);
    }
    let response = String::from_utf8_lossy(&buffer).to_string();
    let (headers, body) = response.split_once("\r\n\r\n").ok_or_else(|| {
        MetricsError::InvalidEndpoint("metrics endpoint response was not valid HTTP".to_string())
    })?;
    let status_line = headers.lines().next().unwrap_or("HTTP/1.1 000 Invalid");
    if !status_line.contains(" 200 ") && !status_line.contains(" 204 ") {
        return Err(MetricsError::HttpStatus(status_line.to_string()));
    }
    Ok(body.to_string())
}

/// Current unix-ish timestamp in milliseconds for sample correlation.
#[must_use]
pub fn now_ms() -> u64 {
    let millis = OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000;
    u64::try_from(millis).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_prometheus_labels() -> Result<(), MetricsError> {
        let document = parse_prometheus_text(
            r#"# HELP paths test
paths{name="main",state="ready"} 1
paths_bytes_received{name="main",state="ready"} 1000
"#,
        )?;
        assert_eq!(document.samples.len(), 2);
        let snapshot = reduce_mediamtx_document(&document, 1_000);
        assert_eq!(snapshot.active_paths, 1);
        assert_eq!(snapshot.bytes_received_total, 1000);
        Ok(())
    }

    #[test]
    fn calculates_kbps_from_byte_delta() -> Result<(), MetricsError> {
        let mut accumulator = MetricsAccumulator::new();
        let _first = accumulator.ingest_prometheus_text(
            "paths{name=\"main\",state=\"ready\"} 1\npaths_bytes_received{name=\"main\",state=\"ready\"} 0\npaths_bytes_sent{name=\"main\",state=\"ready\"} 0\n",
            1_000,
        )?;
        let second = accumulator.ingest_prometheus_text(
            "paths{name=\"main\",state=\"ready\"} 1\npaths_bytes_received{name=\"main\",state=\"ready\"} 500000\npaths_bytes_sent{name=\"main\",state=\"ready\"} 250000\n",
            2_000,
        )?;
        assert_eq!(second.stream_metrics.input_bitrate_kbps, 4_000);
        assert_eq!(second.stream_metrics.output_bitrate_kbps, 2_000);
        Ok(())
    }

    #[test]
    fn parses_srtla_status_line_units() {
        let mut accumulator = MetricsAccumulator::new();
        let result = accumulator.ingest_srtla_log_line(
            "rtt=420ms loss=2.5% retransmits=12 links=3 bitrate=4500kbps fps=30",
            10,
        );
        assert_eq!(result.stream_metrics.rtt_ms, 420);
        assert_eq!(result.stream_metrics.connected_links, 3);
        assert_eq!(result.stream_metrics.input_bitrate_kbps, 4_500);
        assert!((result.stream_metrics.packet_loss_percent - 2.5).abs() < f32::EPSILON);
    }
}

/// Deterministic metric scenarios for CLI/API smoke testing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MetricsScenario {
    /// Healthy contribution path.
    Healthy,
    /// Degraded but still watchable contribution path.
    Degraded,
    /// Brownout: technically connected, bad viewer experience.
    Brownout,
    /// Offline contribution path.
    Offline,
}

impl MetricsScenario {
    /// Parses a scenario label.
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "healthy" => Some(Self::Healthy),
            "degraded" => Some(Self::Degraded),
            "brownout" => Some(Self::Brownout),
            "offline" => Some(Self::Offline),
            _ => None,
        }
    }

    /// All scenario labels accepted by the API.
    #[must_use]
    pub fn labels() -> Vec<&'static str> {
        vec!["healthy", "degraded", "brownout", "offline"]
    }
}

/// Metrics source configuration used by the local agent API.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetricsSourceConfig {
    /// Stable source name.
    pub name: String,
    /// Optional Prometheus metrics URL.
    pub metrics_url: Option<String>,
    /// Optional router API URL.
    pub api_url: Option<String>,
    /// Request timeout in milliseconds.
    pub timeout_ms: u64,
}

impl MetricsSourceConfig {
    /// Returns a disabled sample source.
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            name: "disabled".to_string(),
            metrics_url: None,
            api_url: None,
            timeout_ms: 1_000,
        }
    }
}

/// Snapshot used by feature areas agent endpoints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelayMetricsSnapshot {
    /// Stable source name.
    pub source: String,
    /// Source timestamp in milliseconds.
    pub timestamp_ms: u64,
    /// Optional router snapshot.
    pub router: Option<RouterMetricsSnapshot>,
    /// Converted stream metrics.
    pub stream_metrics: StreamMetrics,
    /// Non-fatal conversion warnings.
    pub warnings: Vec<String>,
}

impl RelayMetricsSnapshot {
    /// Converts this relay snapshot into health-engine metrics.
    #[must_use]
    pub fn to_stream_metrics(&self) -> StreamMetrics {
        self.stream_metrics.clone()
    }
}

/// Stateful relay metrics reducer used by the local agent.
#[derive(Debug, Clone, Default)]
pub struct RelayMetricsState {
    accumulator: MetricsAccumulator,
}

impl RelayMetricsState {
    /// Creates an empty relay metrics state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Updates state from Prometheus text.
    pub fn update_from_prometheus_text(
        &mut self,
        source: String,
        text: &str,
        timestamp_ms: u64,
    ) -> Result<RelayMetricsSnapshot, MetricsError> {
        let result = self
            .accumulator
            .ingest_prometheus_text(text, timestamp_ms)?;
        Ok(RelayMetricsSnapshot {
            source,
            timestamp_ms,
            router: result.router,
            stream_metrics: result.stream_metrics,
            warnings: result.warnings,
        })
    }

    /// Updates state from one SRTLA status/log line.
    pub fn update_from_srtla_log_line(
        &mut self,
        source: String,
        line: &str,
        timestamp_ms: u64,
    ) -> RelayMetricsSnapshot {
        let result = self.accumulator.ingest_srtla_log_line(line, timestamp_ms);
        RelayMetricsSnapshot {
            source,
            timestamp_ms,
            router: result.router,
            stream_metrics: result.stream_metrics,
            warnings: result.warnings,
        }
    }

    /// Returns an accumulator snapshot.
    #[must_use]
    pub fn snapshot(&self) -> MetricsAccumulatorSnapshot {
        self.accumulator.snapshot()
    }
}

/// Async poller for process-bound media routers.
#[derive(Debug, Default)]
pub struct MetricsPoller;

impl MetricsPoller {
    /// Creates a poller.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Polls a Prometheus-compatible metrics source.
    pub async fn poll_prometheus(
        &self,
        state: &mut RelayMetricsState,
        source: &MetricsSourceConfig,
        timestamp_ms: u64,
    ) -> Result<RelayMetricsSnapshot, MetricsError> {
        let metrics_url = source.metrics_url.as_deref().ok_or_else(|| {
            MetricsError::InvalidEndpoint("metrics source has no metrics_url".to_string())
        })?;
        let body = poll_http_text(metrics_url, source.timeout_ms).await?;
        state.update_from_prometheus_text(source.name.clone(), &body, timestamp_ms)
    }

    /// Polls a MediaMTX API endpoint for reachability. Detailed API reduction lands after router schema pinning.
    pub async fn poll_mediamtx_api(
        &self,
        source: &MetricsSourceConfig,
    ) -> Result<RelayMetricsSnapshot, MetricsError> {
        let api_url = source.api_url.as_deref().ok_or_else(|| {
            MetricsError::InvalidEndpoint("metrics source has no api_url".to_string())
        })?;
        let endpoint = format!("{}/v3/paths/list", api_url.trim_end_matches('/'));
        let body = poll_http_text(&endpoint, source.timeout_ms).await?;
        let timestamp_ms = now_ms();
        let mut metrics = StreamMetrics {
            timestamp_ms,
            ..StreamMetrics::default()
        };
        if body.trim().is_empty() {
            metrics.input_bitrate_kbps = 0;
            metrics.connected_links = 0;
            metrics.clean_frame_age_ms = 15_000;
        }
        Ok(RelayMetricsSnapshot {
            source: source.name.clone(),
            timestamp_ms,
            router: None,
            stream_metrics: metrics,
            warnings: vec![
                "MediaMTX API poll verified endpoint reachability; Prometheus metrics remain the authoritative reducer source".to_string(),
            ],
        })
    }
}

/// Builds a deterministic snapshot for a known scenario.
#[must_use]
pub fn simulated_relay_snapshot(
    scenario: MetricsScenario,
    timestamp_ms: u64,
) -> RelayMetricsSnapshot {
    let mut metrics = StreamMetrics {
        timestamp_ms,
        ..StreamMetrics::default()
    };
    let source = match scenario {
        MetricsScenario::Healthy => "demo-healthy",
        MetricsScenario::Degraded => {
            metrics.input_bitrate_kbps = 2_000;
            metrics.packet_loss_percent = 3.5;
            metrics.rtt_ms = 360;
            "demo-degraded"
        }
        MetricsScenario::Brownout => {
            metrics.input_bitrate_kbps = 850;
            metrics.packet_loss_percent = 9.0;
            metrics.rtt_ms = 740;
            metrics.retransmits_per_sec = 70;
            "demo-brownout"
        }
        MetricsScenario::Offline => {
            metrics.input_bitrate_kbps = 0;
            metrics.output_bitrate_kbps = 0;
            metrics.connected_links = 0;
            metrics.clean_frame_age_ms = 15_000;
            "demo-offline"
        }
    };
    RelayMetricsSnapshot {
        source: source.to_string(),
        timestamp_ms,
        router: None,
        stream_metrics: metrics,
        warnings: vec!["deterministic demo snapshot; no router was queried".to_string()],
    }
}
