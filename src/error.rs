//! Error types for the TRMNL SDK.

use thiserror::Error;

/// Errors that can occur when interacting with the TRMNL API.
#[derive(Debug, Error)]
pub enum Error {
    /// HTTP request failed (network error, timeout, etc.)
    #[error("HTTP request failed: {0}")]
    Request(String),

    /// TRMNL API returned an error status code
    #[error("API returned error status {status}: {body}")]
    Api {
        /// HTTP status code
        status: u16,
        /// Response body
        body: String,
    },

    /// Rate limited - TRMNL allows max 12 requests per hour
    #[error("Rate limited - TRMNL allows max 12 requests per hour")]
    RateLimited,

    /// Client not configured (missing plugin UUID)
    #[error("TRMNL not configured (missing plugin UUID)")]
    NotConfigured,

    /// JSON serialization error
    #[error("JSON serialization error: {0}")]
    Serialization(String),
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::Request(err.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::Serialization(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::RateLimited;
        assert!(err.to_string().contains("12 requests per hour"));

        let err = Error::Api {
            status: 400,
            body: "Bad request".to_string(),
        };
        assert!(err.to_string().contains("400"));
        assert!(err.to_string().contains("Bad request"));
    }
}
