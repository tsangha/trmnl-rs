//! Basic BYOS server example
//!
//! Run with: cargo run --example basic_byos --features axum
//!
//! Then test with:
//!   curl -H "ID: test-device" http://localhost:3000/api/display

use axum::{routing::get, Json, Router};
use trmnl::{DeviceInfo, DisplayResponse, LogEntry, LogResponse, SetupResponse};

/// GET /api/setup - Device registration
async fn setup(device: DeviceInfo) -> Json<SetupResponse> {
    println!("Device {} requesting setup", device.mac_address);

    Json(SetupResponse::new(
        format!("trmnl-{}", device.short_id()),
        "https://example.com/welcome.png",
        "Welcome to BYOS!",
    ))
}

/// GET /api/display - Main display endpoint
async fn display(device: DeviceInfo) -> Json<DisplayResponse> {
    println!(
        "Device {} requesting display (battery: {:?}%)",
        device.mac_address,
        device.battery_percentage()
    );

    // In a real implementation, you would:
    // 1. Generate HTML content based on your data
    // 2. Render to PNG
    // 3. Save the PNG and return its URL

    // Use timestamp in filename for cache busting
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    Json(
        DisplayResponse::new(
            format!("https://example.com/screen/{}.png", timestamp),
            format!("{}.png", timestamp),
        )
        .with_refresh_rate(60),
    )
}

/// POST /api/log - Device telemetry
async fn log(device: DeviceInfo, Json(entry): Json<LogEntry>) -> Json<LogResponse> {
    println!(
        "Log from {}: {:?} (battery: {:?}V)",
        device.mac_address,
        entry.log_message,
        entry
            .device_status_stamp
            .as_ref()
            .and_then(|s| s.battery_voltage)
    );

    Json(LogResponse::ok())
}

#[tokio::main]
async fn main() {
    println!("Starting TRMNL BYOS server on http://localhost:3000");
    println!();
    println!("Endpoints:");
    println!("  GET  /api/setup   - Device registration");
    println!("  GET  /api/display - Get display image");
    println!("  POST /api/log     - Device telemetry");
    println!();
    println!("Test with:");
    println!("  curl -H 'ID: test-device' http://localhost:3000/api/display");

    let app = Router::new()
        .route("/api/setup", get(setup))
        .route("/api/display", get(display))
        .route("/api/log", axum::routing::post(log));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
