//! QR rendering for encoder profile handoff.

use qrcode::{QrCode, render::svg};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// QR render request.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct QrRenderRequest {
    /// Payload encoded in the QR symbol.
    pub payload: String,
    /// Human-readable label for UI display.
    pub label: String,
    /// Minimum SVG dimension in pixels.
    pub min_dimension_px: u32,
}

impl QrRenderRequest {
    /// Creates a QR render request using a safe default size.
    #[must_use]
    pub fn new(payload: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            payload: payload.into(),
            label: label.into(),
            min_dimension_px: 256,
        }
    }
}

/// Rendered QR payload.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct QrRender {
    /// Original payload. Treat as sensitive when profile URLs contain passphrases.
    pub payload: String,
    /// Payload byte length.
    pub payload_len: usize,
    /// UI label.
    pub label: String,
    /// Standalone SVG document fragment.
    pub svg: String,
    /// Screen-reader fallback.
    pub alt_text: String,
}

/// QR render error.
#[derive(Debug, Error, Eq, PartialEq)]
pub enum QrRenderError {
    /// Payload is empty.
    #[error("QR payload is empty")]
    EmptyPayload,
    /// QR generation failed.
    #[error("QR generation failed: {0}")]
    Qr(String),
}

/// Renders a QR symbol as SVG.
///
/// # Errors
///
/// Returns an error when the payload is empty or too large for QR encoding.
pub fn render_qr_svg(request: &QrRenderRequest) -> Result<QrRender, QrRenderError> {
    if request.payload.trim().is_empty() {
        return Err(QrRenderError::EmptyPayload);
    }
    let code = QrCode::new(request.payload.as_bytes())
        .map_err(|error| QrRenderError::Qr(error.to_string()))?;
    let size = request.min_dimension_px.max(128);
    let svg = code
        .render::<svg::Color>()
        .min_dimensions(size, size)
        .dark_color(svg::Color("#111111"))
        .light_color(svg::Color("#ffffff"))
        .build();
    Ok(QrRender {
        payload: request.payload.clone(),
        payload_len: request.payload.len(),
        label: request.label.clone(),
        svg,
        alt_text: format!("QR code for {}", request.label),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_svg() -> Result<(), QrRenderError> {
        let rendered = render_qr_svg(&QrRenderRequest::new("srt://127.0.0.1:9000", "test"))?;
        assert!(rendered.svg.contains("svg"));
        assert_eq!(rendered.payload_len, "srt://127.0.0.1:9000".len());
        Ok(())
    }
}
