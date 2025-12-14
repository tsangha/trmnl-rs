//! Optional authentication for BYOS endpoints.
//!
//! By default, BYOS endpoints are public - anyone who knows the URL can access them.
//! This module provides optional token-based authentication.
//!
//! # Usage
//!
//! Configure your device URL with a token query parameter:
//! `https://yourserver.com/api/display?token=your-secret-token`
//!
//! Then validate it in your handler:
//!
//! ```rust,ignore
//! use trmnl::{DeviceInfo, DisplayResponse, TokenAuth};
//!
//! async fn display(
//!     device: DeviceInfo,
//!     auth: TokenAuth,
//! ) -> Result<Json<DisplayResponse>, (StatusCode, &'static str)> {
//!     // Validate token against environment variable
//!     auth.validate_env("TRMNL_TOKEN")?;
//!
//!     // Or validate against a specific value
//!     // auth.validate("my-secret-token")?;
//!
//!     Ok(Json(DisplayResponse::new(...)))
//! }
//! ```

use std::collections::HashMap;

/// Authentication error returned when token validation fails.
#[derive(Debug, Clone)]
pub struct AuthError {
    /// Error message
    pub message: &'static str,
}

impl AuthError {
    /// Create a new auth error.
    pub fn new(message: &'static str) -> Self {
        Self { message }
    }
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AuthError {}

/// Token authentication extractor.
///
/// Extracts the `token` query parameter from the request URL.
/// Use with `validate()` or `validate_env()` to check the token.
#[derive(Debug, Clone, Default)]
pub struct TokenAuth {
    /// The token from the query string (if present)
    pub token: Option<String>,
}

impl TokenAuth {
    /// Create a new TokenAuth with the given token.
    pub fn new(token: Option<String>) -> Self {
        Self { token }
    }

    /// Validate the token against an expected value.
    ///
    /// Returns `Ok(())` if tokens match, or `Err(AuthError)` if not.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// auth.validate("my-secret-token")?;
    /// ```
    pub fn validate(&self, expected: &str) -> Result<(), AuthError> {
        match &self.token {
            Some(token) if token == expected => Ok(()),
            Some(_) => Err(AuthError::new("Invalid token")),
            None => Err(AuthError::new("Missing token")),
        }
    }

    /// Validate the token against an environment variable.
    ///
    /// If the environment variable is not set, authentication is skipped (allows open access).
    /// This lets you easily enable/disable auth via environment.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // If TRMNL_TOKEN is set, validates against it
    /// // If TRMNL_TOKEN is not set, allows all requests
    /// auth.validate_env("TRMNL_TOKEN")?;
    /// ```
    pub fn validate_env(&self, env_var: &str) -> Result<(), AuthError> {
        match std::env::var(env_var) {
            Ok(expected) => self.validate(&expected),
            Err(_) => Ok(()), // No token configured = open access
        }
    }

    /// Check if a token was provided (without validating it).
    pub fn has_token(&self) -> bool {
        self.token.is_some()
    }

    /// Extract token from a query string.
    ///
    /// Useful for manual extraction outside of axum.
    pub fn from_query_string(query: &str) -> Self {
        let params: HashMap<_, _> = form_urlencoded::parse(query.as_bytes()).collect();
        Self {
            token: params.get("token").map(|s| s.to_string()),
        }
    }
}

#[cfg(feature = "axum")]
mod axum_impl {
    use super::*;
    use axum::extract::FromRequestParts;
    use axum::http::request::Parts;
    use axum::http::StatusCode;

    /// Axum extractor for TokenAuth.
    ///
    /// Extracts the `token` query parameter from the request.
    ///
    /// Note: TRMNL firmware has a quirk where if you configure the base URL with
    /// a query parameter like `?token=xxx`, it appends `/api/display` to the token
    /// value, resulting in `token=xxx/api/display`. We strip any `/api/...` suffix.
    impl<S> FromRequestParts<S> for TokenAuth
    where
        S: Send + Sync,
    {
        type Rejection = (StatusCode, &'static str);

        async fn from_request_parts(
            parts: &mut Parts,
            _state: &S,
        ) -> Result<Self, Self::Rejection> {
            let token = parts.uri.query().and_then(|q| {
                form_urlencoded::parse(q.as_bytes())
                    .find(|(k, _)| k == "token")
                    .map(|(_, v)| {
                        // Strip firmware's malformed /api/... suffix if present
                        let s = v.to_string();
                        if let Some(idx) = s.find("/api/") {
                            s[..idx].to_string()
                        } else {
                            s
                        }
                    })
            });

            Ok(TokenAuth { token })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_auth_validate() {
        let auth = TokenAuth::new(Some("secret123".to_string()));
        assert!(auth.validate("secret123").is_ok());
        assert!(auth.validate("wrong").is_err());
    }

    #[test]
    fn test_token_auth_missing() {
        let auth = TokenAuth::new(None);
        assert!(auth.validate("anything").is_err());
    }

    #[test]
    fn test_from_query_string() {
        let auth = TokenAuth::from_query_string("token=mysecret&other=value");
        assert_eq!(auth.token, Some("mysecret".to_string()));

        let auth_empty = TokenAuth::from_query_string("other=value");
        assert_eq!(auth_empty.token, None);
    }

    #[test]
    fn test_validate_env_not_set() {
        // When env var is not set, should allow access
        let auth = TokenAuth::new(None);
        assert!(auth.validate_env("NONEXISTENT_VAR_12345").is_ok());
    }

    #[test]
    fn test_from_query_string_with_firmware_quirk() {
        // TRMNL firmware appends /api/... to tokens when URL is misconfigured
        let auth = TokenAuth::from_query_string("token=mysecret/api/display");
        // from_query_string doesn't strip (that's in axum extractor)
        // but the token should validate if we manually strip
        let token = auth.token.unwrap();
        let clean = if let Some(idx) = token.find("/api/") {
            token[..idx].to_string()
        } else {
            token
        };
        assert_eq!(clean, "mysecret");
    }
}
