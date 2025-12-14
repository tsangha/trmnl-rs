//! Error types for the TRMNL crate.

use thiserror::Error;

/// Errors that can occur when working with TRMNL.
///
/// This enum is marked `#[non_exhaustive]` to allow adding new error variants
/// in minor versions without breaking changes.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// Image rendering failed
    #[error("Render failed: {0}")]
    Render(String),

    /// File I/O error
    #[error("I/O error: {0}")]
    Io(String),

    /// Chrome/browser not found or failed
    #[error("Chrome error: {0}")]
    Chrome(String),

    /// Image too large for TRMNL (max 90KB)
    #[error("Image too large: {size} bytes (max {max} bytes)")]
    ImageTooLarge {
        /// Actual size in bytes
        size: usize,
        /// Maximum allowed size
        max: usize,
    },

    /// JSON serialization error
    #[error("JSON error: {0}")]
    Json(String),

    /// Configuration error (schedule, settings, etc.)
    #[error("Config error: {0}")]
    Config(String),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Json(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::ImageTooLarge {
            size: 100_000,
            max: 90 * 1024,
        };
        assert!(err.to_string().contains("100000"));
        assert!(err.to_string().contains("92160"));
    }

    #[test]
    fn test_render_error() {
        let err = Error::Render("Chrome crashed".to_string());
        assert!(err.to_string().contains("Chrome crashed"));
    }
}
