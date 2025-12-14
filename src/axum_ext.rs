//! Axum integration for TRMNL BYOS servers.
//!
//! Provides extractors to easily get device info from requests.
//!
//! # Example
//!
//! ```rust,ignore
//! use axum::{Router, routing::get};
//! use trmnl::{DisplayResponse, DeviceInfo};
//!
//! async fn display(device: DeviceInfo) -> axum::Json<DisplayResponse> {
//!     tracing::info!("Device {} requesting display", device.mac_address);
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

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::StatusCode;

use crate::DeviceInfo;

/// Extract device info from request headers.
///
/// This extractor reads TRMNL firmware headers:
/// - `ID`: Device MAC address (required)
/// - `Battery-Voltage`: Battery voltage
/// - `FW-Version`: Firmware version
/// - `RSSI`: WiFi signal strength
/// - `Refresh-Rate`: Current refresh rate
///
/// # Example
///
/// ```rust,ignore
/// use trmnl::DeviceInfo;
///
/// async fn handler(device: DeviceInfo) {
///     println!("MAC: {}", device.mac_address);
///     if let Some(pct) = device.battery_percentage() {
///         println!("Battery: {}%", pct);
///     }
/// }
/// ```
impl<S> FromRequestParts<S> for DeviceInfo
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let headers = &parts.headers;

        // MAC address is required
        let mac_address = headers
            .get("ID")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unknown".to_string());

        // Battery voltage
        let battery_voltage = headers
            .get("Battery-Voltage")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok());

        // Firmware version
        let firmware_version = headers
            .get("FW-Version")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // RSSI
        let rssi = headers
            .get("RSSI")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok());

        // Refresh rate
        let refresh_rate = headers
            .get("Refresh-Rate")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse().ok());

        Ok(DeviceInfo {
            mac_address,
            battery_voltage,
            firmware_version,
            rssi,
            refresh_rate,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Request;

    #[tokio::test]
    async fn test_device_info_extractor() {
        let request = Request::builder()
            .header("ID", "AA:BB:CC:DD:EE:FF")
            .header("Battery-Voltage", "4.2")
            .header("FW-Version", "1.0.0")
            .header("RSSI", "-50")
            .header("Refresh-Rate", "60")
            .body(())
            .unwrap();

        let (mut parts, _body) = request.into_parts();
        let device = DeviceInfo::from_request_parts(&mut parts, &())
            .await
            .unwrap();

        assert_eq!(device.mac_address, "AA:BB:CC:DD:EE:FF");
        assert_eq!(device.battery_voltage, Some(4.2));
        assert_eq!(device.firmware_version, Some("1.0.0".to_string()));
        assert_eq!(device.rssi, Some(-50));
        assert_eq!(device.refresh_rate, Some(60));
    }

    #[tokio::test]
    async fn test_device_info_missing_headers() {
        let request = Request::builder().body(()).unwrap();

        let (mut parts, _body) = request.into_parts();
        let device = DeviceInfo::from_request_parts(&mut parts, &())
            .await
            .unwrap();

        assert_eq!(device.mac_address, "unknown");
        assert_eq!(device.battery_voltage, None);
    }
}
