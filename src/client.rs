//! TRMNL API client for pushing content to e-ink displays.

use std::time::Duration;

use serde::Serialize;

use crate::error::Error;
use crate::{API_BASE_URL, DEFAULT_TIMEOUT_SECS};

/// Merge strategy for webhook updates.
///
/// Controls how new data is combined with existing data on the TRMNL display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeStrategy {
    /// Replace all existing data with new data (default behavior)
    Replace,

    /// Deep merge new data with existing data
    ///
    /// Nested objects are recursively merged. New keys are added,
    /// existing keys are updated with new values.
    DeepMerge,

    /// Append new items to arrays
    ///
    /// Use with `stream_limit` to cap array size. New items are appended
    /// and oldest items are removed when limit is exceeded.
    Stream,
}

/// Internal webhook request payload
#[derive(Debug, Serialize)]
struct WebhookPayload<T: Serialize> {
    merge_variables: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    merge_strategy: Option<MergeStrategy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream_limit: Option<usize>,
}

/// TRMNL API client for pushing content to displays.
///
/// # Example
///
/// ```rust,no_run
/// use trmnl::Client;
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct Dashboard {
///     temperature: i32,
///     humidity: i32,
///     status: String,
/// }
///
/// # async fn example() -> Result<(), trmnl::Error> {
/// let client = Client::new("your-plugin-uuid");
///
/// client.push(Dashboard {
///     temperature: 72,
///     humidity: 45,
///     status: "Online".to_string(),
/// }).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    plugin_uuid: String,
    base_url: String,
}

impl Client {
    /// Create a new TRMNL client with the given plugin UUID.
    ///
    /// # Arguments
    ///
    /// * `plugin_uuid` - Your TRMNL private plugin UUID (found in plugin settings)
    ///
    /// # Example
    ///
    /// ```rust
    /// use trmnl::Client;
    ///
    /// let client = Client::new("abc123-your-plugin-uuid");
    /// ```
    pub fn new(plugin_uuid: impl Into<String>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            http,
            plugin_uuid: plugin_uuid.into(),
            base_url: API_BASE_URL.to_string(),
        }
    }

    /// Create a client from the `TRMNL_PLUGIN_UUID` environment variable.
    ///
    /// Returns `None` if the environment variable is not set.
    ///
    /// # Example
    ///
    /// ```rust
    /// use trmnl::Client;
    ///
    /// if let Some(client) = Client::from_env() {
    ///     // Client is ready to use
    /// }
    /// ```
    pub fn from_env() -> Option<Self> {
        let plugin_uuid = std::env::var("TRMNL_PLUGIN_UUID").ok()?;
        Some(Self::new(plugin_uuid))
    }

    /// Set a custom base URL (useful for testing).
    #[must_use]
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    /// Set a custom HTTP client.
    #[must_use]
    pub fn with_http_client(mut self, http: reqwest::Client) -> Self {
        self.http = http;
        self
    }

    /// Get the plugin UUID.
    pub fn plugin_uuid(&self) -> &str {
        &self.plugin_uuid
    }

    /// Push data to the TRMNL display.
    ///
    /// The data will replace any existing data (uses [`MergeStrategy::Replace`]).
    ///
    /// # Arguments
    ///
    /// * `data` - Any serializable data to display
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The HTTP request fails
    /// - Rate limit is exceeded (12 requests/hour)
    /// - The API returns an error status
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use trmnl::Client;
    /// use serde_json::json;
    ///
    /// # async fn example() -> Result<(), trmnl::Error> {
    /// let client = Client::new("your-plugin-uuid");
    ///
    /// client.push(json!({
    ///     "title": "Dashboard",
    ///     "value": 42
    /// })).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn push<T: Serialize>(&self, data: T) -> Result<(), Error> {
        self.push_with_options(data, None, None).await
    }

    /// Push data with custom merge strategy and stream limit.
    ///
    /// # Arguments
    ///
    /// * `data` - Any serializable data to display
    /// * `strategy` - How to merge with existing data (None = Replace)
    /// * `stream_limit` - Max items to keep when using Stream strategy
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use trmnl::{Client, MergeStrategy};
    /// use serde_json::json;
    ///
    /// # async fn example() -> Result<(), trmnl::Error> {
    /// let client = Client::new("your-plugin-uuid");
    ///
    /// // Deep merge new data with existing
    /// client.push_with_options(
    ///     json!({ "weather": { "temp": 72 } }),
    ///     Some(MergeStrategy::DeepMerge),
    ///     None,
    /// ).await?;
    ///
    /// // Stream events, keeping last 5
    /// client.push_with_options(
    ///     json!({ "events": [{ "name": "New Event" }] }),
    ///     Some(MergeStrategy::Stream),
    ///     Some(5),
    /// ).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn push_with_options<T: Serialize>(
        &self,
        data: T,
        strategy: Option<MergeStrategy>,
        stream_limit: Option<usize>,
    ) -> Result<(), Error> {
        let url = format!("{}/{}", self.base_url, self.plugin_uuid);

        let payload = WebhookPayload {
            merge_variables: data,
            merge_strategy: strategy,
            stream_limit,
        };

        let response = self
            .http
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await?;

        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(Error::RateLimited);
        }

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Api { status, body });
        }

        tracing::info!("Successfully pushed data to TRMNL");
        Ok(())
    }

    /// Get the current merge variables from TRMNL.
    ///
    /// Useful for debugging or checking what data is currently stored.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use trmnl::Client;
    ///
    /// # async fn example() -> Result<(), trmnl::Error> {
    /// let client = Client::new("your-plugin-uuid");
    ///
    /// let current = client.get_current().await?;
    /// println!("Current data: {}", current);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_current(&self) -> Result<serde_json::Value, Error> {
        let url = format!("{}/{}", self.base_url, self.plugin_uuid);

        let response = self.http.get(&url).send().await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Api { status, body });
        }

        let data: serde_json::Value = response.json().await?;
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = Client::new("test-uuid-123");
        assert_eq!(client.plugin_uuid(), "test-uuid-123");
    }

    #[test]
    fn test_merge_strategy_serialization() {
        assert_eq!(
            serde_json::to_string(&MergeStrategy::Replace).unwrap(),
            "\"replace\""
        );
        assert_eq!(
            serde_json::to_string(&MergeStrategy::DeepMerge).unwrap(),
            "\"deep_merge\""
        );
        assert_eq!(
            serde_json::to_string(&MergeStrategy::Stream).unwrap(),
            "\"stream\""
        );
    }

    #[test]
    fn test_webhook_payload_serialization() {
        let payload = WebhookPayload {
            merge_variables: serde_json::json!({"test": 123}),
            merge_strategy: Some(MergeStrategy::DeepMerge),
            stream_limit: None,
        };

        let json = serde_json::to_string(&payload).unwrap();
        assert!(json.contains("\"merge_variables\""));
        assert!(json.contains("\"merge_strategy\":\"deep_merge\""));
        assert!(!json.contains("stream_limit"));
    }

    #[test]
    fn test_builder_pattern() {
        let client = Client::new("uuid")
            .with_base_url("http://localhost:8080")
            .with_http_client(reqwest::Client::new());

        assert_eq!(client.plugin_uuid(), "uuid");
    }
}
