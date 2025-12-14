//! BYOS (Bring Your Own Server) protocol types.
//!
//! These types match what the TRMNL firmware expects.
//! See: <https://github.com/usetrmnl/trmnl-firmware>

use serde::{Deserialize, Serialize};

use crate::battery_percentage;

/// Device information extracted from HTTP headers.
///
/// The TRMNL firmware sends device info in request headers:
/// - `ID`: MAC address
/// - `Battery-Voltage`: Battery voltage as float (e.g., "4.2")
/// - `FW-Version`: Firmware version string
/// - `RSSI`: WiFi signal strength in dBm
/// - `Refresh-Rate`: Current refresh rate in seconds
///
/// # Example (manual extraction)
///
/// ```
/// use trmnl::DeviceInfo;
///
/// // From raw header values
/// let device = DeviceInfo::new("AA:BB:CC:DD:EE:FF")
///     .with_battery_voltage(4.2)
///     .with_firmware_version("1.2.3")
///     .with_rssi(-50);
///
/// assert_eq!(device.battery_percentage(), Some(100));
/// ```
#[derive(Debug, Clone, Default)]
pub struct DeviceInfo {
    /// Device MAC address (from `ID` header)
    pub mac_address: String,

    /// Battery voltage in volts (from `Battery-Voltage` header)
    pub battery_voltage: Option<f32>,

    /// Firmware version (from `FW-Version` header)
    pub firmware_version: Option<String>,

    /// WiFi signal strength in dBm (from `RSSI` header)
    pub rssi: Option<i32>,

    /// Current refresh rate in seconds (from `Refresh-Rate` header)
    pub refresh_rate: Option<u32>,
}

impl DeviceInfo {
    /// Create new device info with MAC address.
    pub fn new(mac_address: impl Into<String>) -> Self {
        Self {
            mac_address: mac_address.into(),
            ..Default::default()
        }
    }

    /// Set battery voltage.
    #[must_use]
    pub fn with_battery_voltage(mut self, voltage: f32) -> Self {
        self.battery_voltage = Some(voltage);
        self
    }

    /// Set firmware version.
    #[must_use]
    pub fn with_firmware_version(mut self, version: impl Into<String>) -> Self {
        self.firmware_version = Some(version.into());
        self
    }

    /// Set WiFi RSSI.
    #[must_use]
    pub fn with_rssi(mut self, rssi: i32) -> Self {
        self.rssi = Some(rssi);
        self
    }

    /// Set refresh rate.
    #[must_use]
    pub fn with_refresh_rate(mut self, rate: u32) -> Self {
        self.refresh_rate = Some(rate);
        self
    }

    /// Get battery voltage in millivolts.
    pub fn battery_voltage_mv(&self) -> Option<u32> {
        self.battery_voltage.map(|v| (v * 1000.0) as u32)
    }

    /// Get battery percentage (0-100).
    ///
    /// Uses standard LiPo voltage curve: 3.0V (0%) to 4.2V (100%).
    pub fn battery_percentage(&self) -> Option<u8> {
        self.battery_voltage_mv().map(battery_percentage)
    }

    /// Get short device ID (last 4 chars of MAC).
    pub fn short_id(&self) -> &str {
        let len = self.mac_address.len();
        if len >= 4 {
            &self.mac_address[len - 4..]
        } else {
            &self.mac_address
        }
    }
}

/// Response for GET /api/display endpoint.
///
/// This is the critical response type - the firmware uses these fields to:
/// - Fetch the image from `image_url`
/// - Detect new images by comparing `filename`
/// - Control refresh rate and firmware updates
///
/// # Example
///
/// ```
/// use trmnl::DisplayResponse;
///
/// let response = DisplayResponse::new(
///     "https://example.com/screen.png",
///     "screen.png",
/// );
///
/// // Serialize to JSON for the response body
/// let json = serde_json::to_string(&response).unwrap();
/// assert!(json.contains("\"status\":0"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayResponse {
    /// Status code (0 = success)
    pub status: u32,

    /// Full URL to the display image
    pub image_url: String,

    /// Filename for change detection.
    ///
    /// **CRITICAL**: The firmware compares this to the previous filename.
    /// If they match, it skips the display refresh!
    /// Use timestamps in filenames to ensure updates are detected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filename: Option<String>,

    /// Whether to trigger firmware update
    pub update_firmware: bool,

    /// URL to firmware binary (if update_firmware is true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub firmware_url: Option<String>,

    /// Refresh rate in seconds.
    ///
    /// **Note**: Must be a string (firmware expects string).
    pub refresh_rate: String,

    /// Whether to reset the device
    pub reset_firmware: bool,
}

impl DisplayResponse {
    /// Create a new display response.
    ///
    /// # Arguments
    ///
    /// * `image_url` - Full URL to the PNG image
    /// * `filename` - Filename for change detection (use timestamps!)
    pub fn new(image_url: impl Into<String>, filename: impl Into<String>) -> Self {
        Self {
            status: 0,
            image_url: image_url.into(),
            filename: Some(filename.into()),
            update_firmware: false,
            firmware_url: None,
            refresh_rate: "60".to_string(),
            reset_firmware: false,
        }
    }

    /// Set custom refresh rate (in seconds).
    #[must_use]
    pub fn with_refresh_rate(mut self, seconds: u32) -> Self {
        self.refresh_rate = seconds.to_string();
        self
    }

    /// Set firmware update URL.
    #[must_use]
    pub fn with_firmware_update(mut self, firmware_url: impl Into<String>) -> Self {
        self.update_firmware = true;
        self.firmware_url = Some(firmware_url.into());
        self
    }

    /// Trigger device reset.
    #[must_use]
    pub fn with_reset(mut self) -> Self {
        self.reset_firmware = true;
        self
    }

    /// Create an error response.
    ///
    /// Uses status code 1 and empty image URL.
    pub fn error() -> Self {
        Self {
            status: 1,
            image_url: String::new(),
            filename: None,
            update_firmware: false,
            firmware_url: None,
            refresh_rate: "300".to_string(), // Retry in 5 minutes
            reset_firmware: false,
        }
    }
}

impl Default for DisplayResponse {
    fn default() -> Self {
        Self {
            status: 0,
            image_url: String::new(),
            filename: None,
            update_firmware: false,
            firmware_url: None,
            refresh_rate: "60".to_string(),
            reset_firmware: false,
        }
    }
}

/// Response for GET /api/setup endpoint.
///
/// Sent when device first connects. The device stores this configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupResponse {
    /// API key (can be any string for BYOS)
    pub api_key: String,

    /// Friendly device name
    pub friendly_id: String,

    /// Initial image URL
    pub image_url: String,

    /// Welcome message
    pub message: String,
}

impl SetupResponse {
    /// Create a new setup response.
    pub fn new(
        friendly_id: impl Into<String>,
        image_url: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            api_key: "byos".to_string(),
            friendly_id: friendly_id.into(),
            image_url: image_url.into(),
            message: message.into(),
        }
    }
}

/// Log entry from device (POST /api/log).
///
/// The firmware may send device status and debug logs.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    /// Log message text
    #[serde(default)]
    pub log_message: Option<String>,

    /// Device status snapshot
    #[serde(default)]
    pub device_status_stamp: Option<DeviceStatusStamp>,

    /// Any additional fields
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

/// Device status snapshot in log entries.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DeviceStatusStamp {
    /// Battery voltage
    #[serde(default)]
    pub battery_voltage: Option<f32>,

    /// WiFi signal strength
    #[serde(default)]
    pub wifi_rssi_level: Option<i32>,

    /// Current refresh rate
    #[serde(default)]
    pub refresh_rate: Option<u32>,

    /// Firmware version
    #[serde(default)]
    pub current_fw_version: Option<String>,
}

/// Response for POST /api/log endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogResponse {
    /// Status string ("ok")
    pub status: String,
}

impl Default for LogResponse {
    fn default() -> Self {
        Self {
            status: "ok".to_string(),
        }
    }
}

impl LogResponse {
    /// Create a success response.
    pub fn ok() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_info() {
        let device = DeviceInfo::new("AA:BB:CC:DD:EE:FF")
            .with_battery_voltage(4.2)
            .with_firmware_version("1.0.0")
            .with_rssi(-50);

        assert_eq!(device.mac_address, "AA:BB:CC:DD:EE:FF");
        assert_eq!(device.battery_voltage, Some(4.2));
        assert_eq!(device.battery_voltage_mv(), Some(4200));
        assert_eq!(device.battery_percentage(), Some(100));
        assert_eq!(device.short_id(), "E:FF");
    }

    #[test]
    fn test_display_response_serialization() {
        let response = DisplayResponse::new("https://example.com/screen.png", "screen.png")
            .with_refresh_rate(120);

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"status\":0"));
        assert!(json.contains("\"refresh_rate\":\"120\""));
        assert!(json.contains("\"filename\":\"screen.png\""));
        assert!(!json.contains("firmware_url"));
    }

    #[test]
    fn test_setup_response() {
        let response = SetupResponse::new("my-device", "https://example.com/setup.png", "Welcome!");

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"api_key\":\"byos\""));
        assert!(json.contains("\"friendly_id\":\"my-device\""));
    }

    #[test]
    fn test_log_entry_parsing() {
        let json = r#"{"logMessage": "test", "deviceStatusStamp": {"battery_voltage": 4.1}}"#;
        let entry: LogEntry = serde_json::from_str(json).unwrap();

        assert_eq!(entry.log_message, Some("test".to_string()));
        assert_eq!(
            entry.device_status_stamp.as_ref().unwrap().battery_voltage,
            Some(4.1)
        );
    }
}
