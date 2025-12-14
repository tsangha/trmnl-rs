//! BYOS server with HTML rendering
//!
//! Run with: cargo run --example with_render --features "axum render"
//!
//! Requires:
//!   - Google Chrome or Chromium installed
//!   - ImageMagick (optional, for optimization)

use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{header, StatusCode},
    response::Response,
    routing::get,
    Json, Router,
};
use tokio::sync::RwLock;
use trmnl::{
    render::{render_html_to_png, timestamped_filename, RenderConfig},
    DeviceInfo, DisplayResponse,
};

/// Application state
struct AppState {
    /// Base URL for images
    base_url: String,
    /// Directory to store images
    image_dir: PathBuf,
    /// Last generated filename
    last_filename: RwLock<Option<String>>,
    /// Render configuration
    render_config: RenderConfig,
}

/// Generate HTML for the display
fn generate_html(device: &DeviceInfo) -> String {
    let battery = device
        .battery_percentage()
        .map(|p| format!("{}%", p))
        .unwrap_or_else(|| "?".to_string());

    // Use Unix timestamp for simple time display
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let time = format!("T+{}", timestamp % 86400 / 3600); // Hours since midnight-ish
    let date = "TRMNL Demo";

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        html, body {{
            width: 800px;
            height: 480px;
            font-family: -apple-system, BlinkMacSystemFont, sans-serif;
            background: white;
            color: black;
        }}
        .container {{
            padding: 20px;
            height: 100%;
            display: flex;
            flex-direction: column;
        }}
        .header {{
            display: flex;
            justify-content: space-between;
            align-items: center;
            padding-bottom: 20px;
            border-bottom: 2px solid black;
        }}
        .time {{ font-size: 48px; font-weight: bold; }}
        .date {{ font-size: 24px; color: #333; }}
        .content {{
            flex: 1;
            display: flex;
            align-items: center;
            justify-content: center;
            font-size: 36px;
        }}
        .footer {{
            display: flex;
            justify-content: space-between;
            font-size: 14px;
            color: #666;
            padding-top: 10px;
            border-top: 1px solid #ddd;
        }}
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <div class="time">{time}</div>
            <div class="date">{date}</div>
        </div>
        <div class="content">
            Hello, TRMNL! 👋
        </div>
        <div class="footer">
            <span>Device: {device_id}</span>
            <span>Battery: {battery}</span>
        </div>
    </div>
</body>
</html>"#,
        time = time,
        date = date,
        device_id = device.short_id(),
        battery = battery,
    )
}

/// GET /api/display - Generate and return display
async fn display(
    State(state): State<Arc<AppState>>,
    device: DeviceInfo,
) -> Result<Json<DisplayResponse>, (StatusCode, String)> {
    println!(
        "Rendering display for device {} (battery: {:?}%)",
        device.mac_address,
        device.battery_percentage()
    );

    // Generate HTML
    let html = generate_html(&device);

    // Render to PNG
    let png_data = render_html_to_png(&html, &state.render_config)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Save to file
    let filename = timestamped_filename();
    let image_path = state.image_dir.join(&filename);

    tokio::fs::create_dir_all(&state.image_dir)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tokio::fs::write(&image_path, &png_data)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Update last filename
    *state.last_filename.write().await = Some(filename.clone());

    let image_url = format!("{}/images/{}", state.base_url, filename);

    Ok(Json(
        DisplayResponse::new(image_url, filename).with_refresh_rate(60),
    ))
}

/// GET /images/:filename - Serve images
async fn serve_image(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(filename): axum::extract::Path<String>,
) -> Result<Response, (StatusCode, String)> {
    // Security: prevent path traversal
    if filename.contains("..") || filename.contains('/') {
        return Err((StatusCode::BAD_REQUEST, "Invalid filename".to_string()));
    }

    let image_path = state.image_dir.join(&filename);

    let data = tokio::fs::read(&image_path)
        .await
        .map_err(|_| (StatusCode::NOT_FOUND, "Image not found".to_string()))?;

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "image/png")
        .header(header::CACHE_CONTROL, "no-cache")
        .body(Body::from(data))
        .unwrap())
}

#[tokio::main]
async fn main() {
    println!("Starting TRMNL BYOS server with rendering on http://localhost:3000");
    println!();
    println!("Requirements:");
    println!("  - Google Chrome (or set CHROME_PATH)");
    println!("  - ImageMagick (optional, for optimization)");
    println!();
    println!("Test with:");
    println!("  curl -H 'ID: test-device' http://localhost:3000/api/display");

    let state = Arc::new(AppState {
        base_url: "http://localhost:3000".to_string(),
        image_dir: PathBuf::from("/tmp/trmnl-images"),
        last_filename: RwLock::new(None),
        render_config: RenderConfig::default(),
    });

    let app = Router::new()
        .route("/api/display", get(display))
        .route("/images/{filename}", get(serve_image))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
