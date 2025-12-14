//! # trmnl
//!
//! A BYOS (Bring Your Own Server) framework for [TRMNL](https://usetrmnl.com) e-ink displays.
//!
//! TRMNL devices can operate in two modes:
//! - **Cloud mode**: Device polls TRMNL's servers, which poll your webhook
//! - **BYOS mode**: Device polls your server directly (this crate's focus)
//!
//! This crate provides everything you need to build a BYOS server:
//! - Protocol types that match firmware expectations
//! - Device info extraction from HTTP headers
//! - Optional axum integration for quick setup
//! - Optional HTML-to-PNG rendering via Chrome headless
//!
//! ## Quick Start (axum)
//!
//! ```rust,ignore
//! use axum::{Router, routing::get};
//! use trmnl::{DisplayResponse, DeviceInfo};
//!
//! async fn display(device: DeviceInfo) -> axum::Json<DisplayResponse> {
//!     println!("Request from device: {}", device.mac_address);
//!
//!     axum::Json(DisplayResponse::new(
//!         "https://example.com/screen.png",
//!         "screen.png",
//!     ))
//! }
//!
//! let app = Router::new()
//!     .route("/api/display", get(display));
//! ```
//!
//! ## Display Dimensions
//!
//! TRMNL displays are 800x480 pixels. Images must be:
//! - Exactly 800x480 PNG
//! - Under 90KB (firmware rejects larger files)
//! - 16 colors or less for optimal e-ink rendering
//!
//! ## BYOS Protocol
//!
//! Your server must implement these endpoints:
//!
//! | Endpoint | Method | Purpose |
//! |----------|--------|---------|
//! | `/api/setup` | GET | Device registration (optional) |
//! | `/api/display` | GET | Returns image URL and metadata |
//! | `/api/log` | POST | Receives device telemetry (optional) |
//!
//! The device sends these headers:
//! - `ID`: Device MAC address
//! - `Battery-Voltage`: Battery voltage (e.g., "4.2")
//! - `FW-Version`: Firmware version
//! - `RSSI`: WiFi signal strength
//! - `Refresh-Rate`: Current refresh rate
//!
//! ## Feature Flags
//!
//! - `axum` - Axum extractors and handlers
//! - `render` - HTML to PNG rendering via Chrome headless
//! - `schedule` - Time-based refresh rate scheduling (YAML config)
//! - `full` - All features

pub mod auth;
mod byos;
mod error;

pub use auth::TokenAuth;
pub use byos::{
    DeviceInfo, DeviceStatusStamp, DisplayResponse, LogEntry, LogResponse, SetupResponse,
};
pub use error::Error;

/// TRMNL display width in pixels
pub const DISPLAY_WIDTH: u32 = 800;

/// TRMNL display height in pixels
pub const DISPLAY_HEIGHT: u32 = 480;

/// Maximum image size in bytes (firmware rejects larger)
pub const MAX_IMAGE_SIZE: usize = 90 * 1024; // 90KB

/// LiPo battery minimum voltage (0%)
pub const BATTERY_MIN_MV: u32 = 3000;

/// LiPo battery maximum voltage (100%)
pub const BATTERY_MAX_MV: u32 = 4200;

// Optional modules
#[cfg(feature = "render")]
pub mod render;
#[cfg(feature = "render")]
pub use render::{render_html_to_png, timestamped_filename, RenderConfig};

#[cfg(feature = "schedule")]
pub mod schedule;
#[cfg(feature = "schedule")]
pub use schedule::{
    get_global_refresh_rate, init_global_schedule, DaySelector, RefreshSchedule, ScheduleRule,
};

// Re-export axum integration
#[cfg(feature = "axum")]
pub mod axum_ext;

/// Convert battery voltage (in millivolts) to percentage.
///
/// Uses standard LiPo voltage curve: 3.0V (0%) to 4.2V (100%).
///
/// # Example
///
/// ```
/// use trmnl::battery_percentage;
///
/// assert_eq!(battery_percentage(4200), 100);
/// assert_eq!(battery_percentage(3600), 50);
/// assert_eq!(battery_percentage(3000), 0);
/// ```
pub fn battery_percentage(voltage_mv: u32) -> u8 {
    if voltage_mv <= BATTERY_MIN_MV {
        0
    } else if voltage_mv >= BATTERY_MAX_MV {
        100
    } else {
        ((voltage_mv - BATTERY_MIN_MV) * 100 / (BATTERY_MAX_MV - BATTERY_MIN_MV)) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_battery_percentage() {
        assert_eq!(battery_percentage(4200), 100);
        assert_eq!(battery_percentage(4201), 100); // Clamp high
        assert_eq!(battery_percentage(3000), 0);
        assert_eq!(battery_percentage(2999), 0); // Clamp low
        assert_eq!(battery_percentage(3600), 50);
    }

    #[test]
    fn test_constants() {
        assert_eq!(DISPLAY_WIDTH, 800);
        assert_eq!(DISPLAY_HEIGHT, 480);
        assert_eq!(MAX_IMAGE_SIZE, 90 * 1024);
    }
}
