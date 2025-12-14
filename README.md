# trmnl-rs

A Rust framework for building [TRMNL](https://usetrmnl.com) BYOS (Bring Your Own Server) applications.

[![Crates.io](https://img.shields.io/crates/v/trmnl.svg)](https://crates.io/crates/trmnl)
[![Documentation](https://docs.rs/trmnl/badge.svg)](https://docs.rs/trmnl)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## What is TRMNL?

[TRMNL](https://usetrmnl.com) is an e-ink display device with an ESP32-C3 microcontroller and 7.5" screen. It connects to WiFi and periodically polls a server for content to display.

**Terminology:**
- **BYOS** (Bring Your Own Server) - You have a TRMNL device and point it at your own server instead of TRMNL's cloud
- **BYOD** (Bring Your Own Device) - You have your own e-ink hardware (not TRMNL) running TRMNL firmware

## When to Use This Crate

**Use this crate if you want to:**
- Run your own server instead of using TRMNL's cloud
- Have complete control over what your display shows
- Integrate private data sources (home automation, internal APIs, databases)
- Build in Rust (there are also [Ruby](https://github.com/usetrmnl/byos_hanami) and [PHP](https://github.com/usetrmnl/byos_laravel) implementations)

**Don't use this crate if you:**
- Want to use TRMNL's cloud with webhooks and Liquid templates (just use their cloud)
- Don't have a server to host your BYOS endpoint

## Getting Started

### 1. Get Your Device Ready

**If you have a TRMNL device:**
1. Purchase from [usetrmnl.com](https://usetrmnl.com)
2. During WiFi setup, configure it to point to your server URL instead of TRMNL's cloud
3. See [TRMNL's BYOS guide](https://docs.usetrmnl.com/go/diy/byos) for device configuration

**If you're bringing your own device (BYOD):**
1. Flash your ESP32-based e-ink display with [TRMNL firmware](https://github.com/usetrmnl/trmnl-firmware)
2. Configure it to point to your server
3. See [TRMNL's BYOD guide](https://docs.usetrmnl.com/go/diy/byod)

### 2. Set Up Your Server

Add to your `Cargo.toml`:
```toml
[dependencies]
trmnl = { version = "0.1", features = ["axum", "render"] }
axum = "0.8"
tokio = { version = "1", features = ["full"] }
```

Create a minimal server (see full examples below):
```rust
use axum::{routing::get, Json, Router};
use trmnl::{DeviceInfo, DisplayResponse};

async fn display(device: DeviceInfo) -> Json<DisplayResponse> {
    // Your display logic here
    Json(DisplayResponse::new("https://yourserver.com/image.png", "image.png"))
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/api/display", get(display));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

### 3. Configure Your Device

Point your device to `https://yourserver.com/api/display`. The device will poll this endpoint and display whatever image URL you return.

## Official TRMNL Resources

- [TRMNL API Documentation](https://docs.usetrmnl.com/go/) - Complete API reference
- [BYOS Setup Guide](https://docs.usetrmnl.com/go/diy/byos) - Official BYOS documentation
- [BYOD Setup Guide](https://docs.usetrmnl.com/go/diy/byod) - Bring your own device
- [How It Works](https://docs.usetrmnl.com/go/how-it-works) - Device architecture
- [Firmware Source](https://github.com/usetrmnl/trmnl-firmware) - Open source firmware (MIT)

## How BYOS Works

```
┌─────────────┐         ┌─────────────────┐         ┌─────────────┐
│   TRMNL     │  GET    │   Your Server   │  fetch  │  Your Data  │
│   Device    │ ──────► │  (built with    │ ◄─────► │  Sources    │
│             │ ◄────── │   this crate)   │         │             │
└─────────────┘  JSON   └─────────────────┘         └─────────────┘
                 + PNG
```

Your device polls your server every N seconds. Your server returns a JSON response pointing to a PNG image. The device downloads and displays it.

### Where Can You Run Your Server?

This crate helps you **build** a BYOS server - a Rust binary you can run anywhere:

| Deployment | Notes |
|------------|-------|
| **Home server** | Raspberry Pi, NAS, old laptop - works great |
| **VPS** | DigitalOcean, Linode, Hetzner, etc. |
| **Cloud** | AWS, GCP, Azure, Fly.io, Railway |
| **Local machine** | For development/testing |

**Requirements:**
- Your server must be reachable from your TRMNL device (same network or public internet)
- If using HTML rendering (`render` feature), Chrome/Chromium must be installed
- HTTPS recommended for public deployments (device supports both HTTP and HTTPS)

**Home server tips:**
- Use a static local IP or hostname (e.g., `http://192.168.1.100:3000` or `http://myserver.local:3000`)
- For access outside your home, set up port forwarding or use a tunnel (Cloudflare Tunnel, Tailscale, ngrok)
- Raspberry Pi 4 handles HTML rendering fine; Pi Zero may struggle with Chrome

## Choose Your Setup

### Option A: Dynamic HTML Rendering (Most Common)

Best for: Dashboards, data displays, anything that changes frequently.

```toml
[dependencies]
trmnl = { version = "0.1", features = ["axum", "render"] }
axum = "0.8"
tokio = { version = "1", features = ["full"] }
```

```rust
use axum::{routing::get, Json, Router};
use trmnl::{DeviceInfo, DisplayResponse};
use trmnl::render::{render_html_to_png, RenderConfig};

async fn display(device: DeviceInfo) -> Json<DisplayResponse> {
    // 1. Generate HTML (fetch your data, build your layout)
    let html = format!(r#"
        <html>
        <body style="width:800px; height:480px; background:white; padding:20px;">
            <h1>Hello from {}</h1>
            <p>Battery: {}%</p>
        </body>
        </html>
    "#, device.short_id(), device.battery_percentage().unwrap_or(0));

    // 2. Render HTML to PNG
    let png = render_html_to_png(&html, &RenderConfig::default()).await.unwrap();

    // 3. Save to disk (your web server serves static files)
    let filename = format!("{}.png", std::time::UNIX_EPOCH.elapsed().unwrap().as_secs());
    std::fs::write(format!("/var/www/trmnl/{}", filename), &png).unwrap();

    // 4. Return URL to the image
    Json(DisplayResponse::new(
        format!("https://myserver.com/trmnl/{}", filename),
        filename,
    ))
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/api/display", get(display));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

**Requirements:** Chrome or Chromium installed on your server.

### Option B: Static/Pre-generated Images

Best for: Simple displays, images generated elsewhere, or when you can't install Chrome.

```toml
[dependencies]
trmnl = { version = "0.1", features = ["axum"] }
axum = "0.8"
tokio = { version = "1", features = ["full"] }
```

```rust
use axum::{routing::get, Json, Router};
use trmnl::{DeviceInfo, DisplayResponse};

async fn display(_device: DeviceInfo) -> Json<DisplayResponse> {
    // Just point to an existing image
    // (generated by a cron job, external service, etc.)
    Json(DisplayResponse::new(
        "https://myserver.com/current-display.png",
        "current-display.png",
    ).with_refresh_rate(300)) // Check every 5 minutes
}
```

## API Reference

### DeviceInfo

Automatically extracted from request headers when using axum:

```rust
async fn display(device: DeviceInfo) -> Json<DisplayResponse> {
    device.mac_address        // "AA:BB:CC:DD:EE:FF"
    device.battery_voltage    // Some(4.2)
    device.battery_percentage() // Some(100)
    device.firmware_version   // Some("1.2.3")
    device.rssi               // Some(-45) (WiFi signal in dBm)
    device.short_id()         // "E:FF" (last 4 chars of MAC)
}
```

### DisplayResponse

```rust
// Minimal
DisplayResponse::new("https://url/image.png", "image.png")

// With options
DisplayResponse::new("https://url/image.png", "image.png")
    .with_refresh_rate(60)    // Seconds until next poll (default: 60)
    .with_firmware_update("https://url/firmware.bin")  // Trigger OTA
    .with_reset()             // Reset device
```

**Important:** The `filename` must change when your image changes. The device compares filenames to detect updates. Use timestamps:

```rust
let filename = format!("{}.png", SystemTime::now()
    .duration_since(UNIX_EPOCH).unwrap().as_secs());
```

### Render Config

```rust
use trmnl::render::RenderConfig;

let config = RenderConfig {
    chrome_path: None,        // Auto-detect, or Some("/path/to/chrome")
    temp_dir: None,           // System temp, or Some(PathBuf::from("/tmp"))
    optimize: true,           // Run through ImageMagick for smaller files
    color_depth: 8,           // Bits per channel
};
```

## BYOS Protocol

Your server implements:

| Endpoint | Method | Required | Purpose |
|----------|--------|----------|---------|
| `/api/display` | GET | Yes | Returns image URL |
| `/api/setup` | GET | No | Device registration |
| `/api/log` | POST | No | Receive device logs |

The device sends these headers:
- `ID`: MAC address
- `Battery-Voltage`: e.g., "4.2"
- `FW-Version`: Firmware version
- `RSSI`: WiFi signal strength
- `Refresh-Rate`: Current refresh rate

## Authentication (Optional)

By default, BYOS endpoints are public—anyone who knows your URL can access them. The device's MAC address (in the `ID` header) identifies the device but doesn't authenticate it.

This crate provides optional token-based authentication via query parameters:

### Setup

1. Configure your device URL with a token:
   ```
   https://yourserver.com/api/display?token=your-secret-token
   ```

2. Set the token on your server (environment variable):
   ```bash
   export TRMNL_TOKEN=your-secret-token
   ```

3. Validate it in your handler:
   ```rust
   use axum::{Json, http::StatusCode};
   use trmnl::{DeviceInfo, DisplayResponse, TokenAuth};

   async fn display(
       device: DeviceInfo,
       auth: TokenAuth,
   ) -> Result<Json<DisplayResponse>, (StatusCode, &'static str)> {
       // Validate against environment variable
       // If TRMNL_TOKEN is not set, allows all requests (open access)
       auth.validate_env("TRMNL_TOKEN")
           .map_err(|e| (StatusCode::UNAUTHORIZED, e.message))?;

       Ok(Json(DisplayResponse::new("https://...", "image.png")))
   }
   ```

### TokenAuth Methods

```rust
// Validate against a specific value
auth.validate("my-secret-token")?;

// Validate against environment variable (if not set, allows all requests)
auth.validate_env("TRMNL_TOKEN")?;

// Check if a token was provided (without validating)
if auth.has_token() { ... }

// Manual extraction (for non-axum use)
let auth = TokenAuth::from_query_string("token=secret&other=value");
```

### Changing the Device URL

The BYOS URL (including any `?token=` parameter) is configured on the device during WiFi setup. To change it:

1. **Factory reset the device** (hold button for 10+ seconds until LED flashes)
2. **Re-run WiFi setup** through the TRMNL app
3. **Enter the new BYOS URL** with your token when prompted

There's no way to change the BYOS URL without re-running WiFi setup—it's baked into the device's firmware configuration.

**Important:** If you add token authentication to an existing BYOS setup, your device will start getting 401 errors until you update the URL on the device.

### Device URL Format

When configuring your device, use this URL format:

```
https://yourserver.com?token=your-secret-token
```

The device will automatically append `/api/display`, `/api/log`, etc. to this base URL.

## Display Constraints

- **Resolution**: 800×480 pixels (fixed)
- **Max file size**: 90KB (device rejects larger)
- **Format**: PNG
- **Colors**: 16 or fewer for best e-ink rendering
- **Orientation**: Landscape only

## Battery Life

The TRMNL device uses a LiPo battery (3.0V-4.2V range). Battery drain depends primarily on refresh rate:

| Refresh Rate | Polls/Day | Expected Battery Life |
|--------------|-----------|----------------------|
| 60s (1 min)  | 1,440     | ~3-5 days            |
| 300s (5 min) | 288       | ~2-3 weeks           |
| 900s (15 min)| 96        | ~1-2 months          |
| 1800s (30 min)| 48       | ~2-3 months          |
| 3600s (1 hr) | 24        | ~3-4 months          |

**Tips for extending battery life:**
- Use longer refresh rates for static content (weather, quotes)
- Use shorter rates only for time-sensitive data (transit, meetings)
- The device reports battery voltage in the `Battery-Voltage` header
- Use `device.battery_percentage()` to display remaining charge

## Building Text Dashboards

For text-heavy dashboards (tasks, calendars, briefings), use HTML with Chrome headless rendering. The key is fixed pixel positioning—Chrome headless doesn't handle flexbox reliably.

### Dashboard Design Principles

1. **Fixed dimensions**: Always set `width: 800px; height: 480px` on body
2. **Absolute positioning**: Use `position: absolute` for major sections
3. **High contrast**: Black text on white background only
4. **No images**: Text renders sharper on e-ink than images
5. **Generous spacing**: E-ink needs more whitespace than LCD

### Example Layout

```
┌────────────────────────────────────────────────────────────┐
│  Header: Date/Time (left)              Weather (right)     │
├────────────────────────────┬───────────────────────────────┤
│                            │                               │
│  Left Column               │  Right Column                 │
│  - Status/metrics          │  - Briefing text              │
│  - Task list               │  - Quotes/highlights          │
│  - Calendar/meetings       │  - News/updates               │
│                            │                               │
├────────────────────────────┴───────────────────────────────┤
│  Footer: Battery (left)    Message (center)                │
└────────────────────────────────────────────────────────────┘
```

### Example Template

See [`templates/dashboard.html`](templates/dashboard.html) for a complete example with:
- Two-column layout with header and footer
- Task lists with due dates
- Meeting schedules
- Body text sections
- Quote/highlight sections

### Font Size Guidelines

| Element | Size | Use For |
|---------|------|---------|
| 20px | Headers | Date, main titles |
| 18px | Subheaders | Time, weather |
| 16px | Emphasis | Key values, footer message |
| 14-15px | Body | Section titles, quotes |
| 12-13px | Details | Task items, body text |
| 11px | Meta | Timestamps, sources |

### CSS Template

```css
* { margin: 0; padding: 0; box-sizing: border-box; }
html, body {
    width: 800px;
    height: 480px;
    overflow: hidden;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    background: white;
    color: black;
}
.header {
    position: absolute;
    top: 10px;
    left: 16px;
    right: 16px;
    height: 40px;
}
.columns {
    position: absolute;
    top: 55px;
    left: 16px;
    right: 16px;
    bottom: 55px;
    display: flex;
    gap: 20px;
}
.column { flex: 1; overflow: hidden; }
.footer {
    position: absolute;
    bottom: 15px;
    left: 16px;
    right: 16px;
    height: 35px;
    border-top: 1px solid #ddd;
}
```

## Refresh Rate Scheduling

The `schedule` feature lets you configure different refresh rates based on time of day and day of week. This helps optimize battery life while keeping displays fresh when needed.

```toml
[dependencies]
trmnl = { version = "0.1", features = ["axum", "schedule"] }
```

### Schedule Configuration (YAML)

Create a schedule config file:

```yaml
# config/schedule.yaml
timezone: "America/New_York"
default_refresh_rate: 300  # 5 minutes (fallback if no rule matches)

schedule:
  # Sleep hours - infrequent updates to save battery
  - days: all
    start: "23:00"
    end: "06:00"
    refresh_rate: 1800  # 30 minutes

  # Morning routine - frequent updates
  - days: weekdays
    start: "06:00"
    end: "09:00"
    refresh_rate: 60  # 1 minute

  # Work hours - moderate updates
  - days: weekdays
    start: "09:00"
    end: "18:00"
    refresh_rate: 120  # 2 minutes

  # Weekend - relaxed
  - days: weekends
    start: "06:00"
    end: "23:00"
    refresh_rate: 600  # 10 minutes
```

### Day Selectors

- `all` - Every day
- `weekdays` - Monday through Friday
- `weekends` - Saturday and Sunday
- `["mon", "wed", "fri"]` - Specific days (list format)
- `monday` / `mon` - Single day

### Usage

**Option 1: Global schedule (recommended for most apps)**

```rust
use trmnl::{init_global_schedule, get_global_refresh_rate};

#[tokio::main]
async fn main() {
    // Load once at startup
    init_global_schedule("config/schedule.yaml");

    // ... start your server
}

async fn display(device: DeviceInfo) -> Json<DisplayResponse> {
    // Returns rate based on current time, or 60s if no schedule loaded
    let refresh_rate = get_global_refresh_rate();

    Json(DisplayResponse::new(url, filename)
        .with_refresh_rate(refresh_rate))
}
```

**Option 2: Manual schedule management**

```rust
use trmnl::schedule::RefreshSchedule;

// Load schedule at startup
let schedule = RefreshSchedule::load("config/schedule.yaml")?;

// In your display handler
async fn display(device: DeviceInfo) -> Json<DisplayResponse> {
    let refresh_rate = schedule.get_refresh_rate(); // Returns rate based on current time

    Json(DisplayResponse::new(url, filename)
        .with_refresh_rate(refresh_rate))
}
```

### Time Ranges

- Normal ranges: `09:00` to `17:00` matches 9am-5pm
- Overnight ranges: `23:00` to `06:00` matches 11pm-6am (spans midnight)
- End time is exclusive: `09:00` to `17:00` does not include exactly 17:00

## Feature Flags

| Feature | Dependencies Added | Use When |
|---------|-------------------|----------|
| `axum` | axum, http | Building a web server (most users) |
| `render` | tokio | Generating images from HTML (requires Chrome) |
| `schedule` | chrono, chrono-tz, serde_yaml | Time-based refresh rate scheduling |
| `full` | All of the above | You want everything |

## Examples

See the [`examples/`](examples/) directory:
- `basic_byos.rs` - Minimal BYOS server
- `with_render.rs` - HTML rendering example

Run with:
```bash
cargo run --example basic_byos --features axum
cargo run --example with_render --features "axum render"
```

## License

MIT
