//! # trmnl
//!
//! A Rust SDK for [TRMNL](https://usetrmnl.com) e-ink displays.
//!
//! TRMNL is a customizable e-ink dashboard that displays information from various sources.
//! This crate provides a type-safe client for pushing data to TRMNL displays via their
//! webhook API.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use trmnl::{Client, Error};
//! use serde_json::json;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Error> {
//!     // Create a client with your plugin UUID
//!     let client = Client::new("your-plugin-uuid");
//!
//!     // Push any JSON-serializable data
//!     client.push(json!({
//!         "temperature": 72,
//!         "humidity": 45,
//!         "status": "Online"
//!     })).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Features
//!
//! - **Type-safe**: Push any `Serialize` type to your display
//! - **Merge strategies**: Replace, deep merge, or stream data
//! - **Rate limit handling**: Automatic detection of TRMNL's 12 req/hour limit
//! - **Async**: Built on `reqwest` for efficient async I/O
//!
//! ## Merge Strategies
//!
//! TRMNL supports three merge strategies for webhook updates:
//!
//! - [`MergeStrategy::Replace`] - Completely replace existing data (default)
//! - [`MergeStrategy::DeepMerge`] - Recursively merge with existing data
//! - [`MergeStrategy::Stream`] - Append to arrays with optional size limit
//!
//! ```rust,no_run
//! use trmnl::{Client, MergeStrategy};
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), trmnl::Error> {
//! let client = Client::new("your-plugin-uuid");
//!
//! // Stream new items to an array, keeping last 10
//! client.push_with_options(
//!     json!({ "events": [{ "time": "10:30", "name": "Meeting" }] }),
//!     Some(MergeStrategy::Stream),
//!     Some(10),
//! ).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## E-ink Display Constraints
//!
//! When designing content for TRMNL displays:
//!
//! - **Resolution**: 800x480 pixels (landscape) or 480x800 (portrait)
//! - **Color depth**: 1-bit (black/white) or 4-bit (16 grayscales)
//! - **No animations** - E-ink refresh is slow
//! - **High contrast** is essential for readability
//! - **Simple layouts** work best
//!
//! ## Template Design Tips
//!
//! **What works in TRMNL Private Plugin Markup:**
//!
//! - Start with `<div class="layout">` as your root container
//! - Use simple `columns` and `column` for two-column layouts
//! - Basic typography: `title`, `title--small`, `description`, `label`, `label--small`
//! - Plain HTML elements: `<div>`, `<strong>`, `<small>`, `<br>`
//! - Inline styles for spacing: `style="margin-top:12px"`
//!
//! **What DOESN'T work:**
//!
//! - Don't wrap in `<div class="screen">` - TRMNL adds this automatically
//! - Don't use `<div class="view view--full">` wrapper - Also added by TRMNL
//! - Complex nested layouts like `layout--row layout--stretch-x` often break
//! - The `item`, `meta`, `content` components have unpredictable positioning
//! - Don't use `title_bar` - It doesn't position correctly in private plugins
//!
//! See the [README](https://github.com/tsangha/trmnl-rs) for complete template examples.

mod client;
mod error;

pub use client::{Client, MergeStrategy};
pub use error::Error;

/// Default timeout for HTTP requests (10 seconds)
pub const DEFAULT_TIMEOUT_SECS: u64 = 10;

/// TRMNL API base URL
pub const API_BASE_URL: &str = "https://usetrmnl.com/api/custom_plugins";

/// Maximum requests per hour allowed by TRMNL
pub const RATE_LIMIT_PER_HOUR: u32 = 12;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants() {
        assert_eq!(DEFAULT_TIMEOUT_SECS, 10);
        assert_eq!(RATE_LIMIT_PER_HOUR, 12);
        assert!(API_BASE_URL.starts_with("https://"));
    }
}
