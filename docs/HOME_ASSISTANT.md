# TRMNL + Home Assistant Integration Design

## Overview

Two-way integration between TRMNL e-ink displays and Home Assistant:

1. **TRMNL displays HA data** - Show sensor states, calendars, etc. on the e-ink display
2. **HA controls TRMNL** - Automations trigger display updates, control refresh rates

## Architecture

```
┌─────────────────┐         ┌─────────────────┐         ┌─────────────┐
│  Home Assistant │ ◄─────► │   trmnl-rs      │ ◄─────► │   TRMNL     │
│                 │  REST   │   BYOS Server   │  HTTP   │   Device    │
│  - Sensors      │         │                 │         │             │
│  - Automations  │         │  - HaClient     │         │  - Display  │
│  - Services     │         │  - Webhooks     │         │  - Battery  │
└─────────────────┘         └─────────────────┘         └─────────────┘
```

## Feature: `homeassistant`

```toml
[dependencies]
trmnl = { version = "0.1", features = ["axum", "render", "homeassistant"] }
```

### HaClient - Fetch HA States

```rust
use trmnl::homeassistant::{HaClient, HaState};

// Initialize with your HA instance
let ha = HaClient::new(
    "http://homeassistant.local:8123",
    "your-long-lived-access-token"
);

// Fetch single entity
let temp = ha.get_state("sensor.living_room_temperature").await?;
println!("Temperature: {}°F", temp.state);

// Fetch multiple entities at once
let states = ha.get_states(&[
    "sensor.living_room_temperature",
    "binary_sensor.front_door",
    "sensor.energy_today",
]).await?;

// Access attributes
let temp = &states["sensor.living_room_temperature"];
println!("Unit: {}", temp.attributes.get("unit_of_measurement").unwrap());
```

### HaState Structure

```rust
pub struct HaState {
    pub entity_id: String,
    pub state: String,
    pub attributes: HashMap<String, serde_json::Value>,
    pub last_changed: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
}

impl HaState {
    /// Parse state as f64 (for sensors)
    pub fn as_f64(&self) -> Option<f64>;

    /// Parse state as bool (for binary sensors)
    pub fn as_bool(&self) -> Option<bool>;

    /// Check if entity is "on", "home", "open", etc.
    pub fn is_on(&self) -> bool;

    /// Get friendly name from attributes
    pub fn friendly_name(&self) -> Option<&str>;
}
```

### Webhook Endpoint - HA Triggers TRMNL

Add a webhook endpoint that HA can call to trigger immediate refresh:

```rust
use trmnl::homeassistant::webhook_handler;

let app = Router::new()
    .route("/api/display", get(display))
    .route("/api/webhook/refresh", post(webhook_handler));  // HA calls this
```

The webhook can:
- Trigger immediate display regeneration
- Change refresh rate temporarily
- Display a specific message/alert

### Schedule from HA Input Helpers

Use HA `input_datetime` and `input_number` helpers to control TRMNL schedule:

```rust
// Fetch schedule parameters from HA
let sleep_start = ha.get_state("input_datetime.trmnl_sleep_start").await?;
let sleep_end = ha.get_state("input_datetime.trmnl_sleep_end").await?;
let refresh_rate = ha.get_state("input_number.trmnl_refresh_rate").await?;
```

## Home Assistant Configuration

### 1. Create Long-Lived Access Token

Settings → Security → Long-lived access tokens → Create Token

### 2. TRMNL as REST Command (for automations)

```yaml
# configuration.yaml
rest_command:
  trmnl_refresh:
    url: "https://yourserver.com/api/webhook/refresh"
    method: POST
    headers:
      Authorization: "Bearer {{ states('input_text.trmnl_token') }}"
    payload: '{"action": "refresh"}'

  trmnl_alert:
    url: "https://yourserver.com/api/webhook/alert"
    method: POST
    headers:
      Authorization: "Bearer {{ states('input_text.trmnl_token') }}"
    payload: '{"message": "{{ message }}"}'
```

### 3. Automations

```yaml
# Refresh display when someone arrives home
automation:
  - alias: "TRMNL - Refresh on arrival"
    trigger:
      - platform: state
        entity_id: person.john
        to: "home"
    action:
      - service: rest_command.trmnl_refresh

  - alias: "TRMNL - Alert on door open too long"
    trigger:
      - platform: state
        entity_id: binary_sensor.front_door
        to: "on"
        for: "00:05:00"
    action:
      - service: rest_command.trmnl_alert
        data:
          message: "Front door has been open for 5 minutes!"
```

### 4. Input Helpers for Schedule Control

```yaml
# configuration.yaml
input_datetime:
  trmnl_sleep_start:
    name: "TRMNL Sleep Start"
    has_time: true
    has_date: false
  trmnl_sleep_end:
    name: "TRMNL Sleep End"
    has_time: true
    has_date: false

input_number:
  trmnl_refresh_rate:
    name: "TRMNL Refresh Rate"
    min: 60
    max: 3600
    step: 60
    unit_of_measurement: "seconds"
```

### 5. TRMNL Device as HA Sensor (optional)

Expose TRMNL battery and status to HA via REST sensor:

```yaml
# configuration.yaml
sensor:
  - platform: rest
    name: "TRMNL Battery"
    resource: "https://yourserver.com/api/status"
    value_template: "{{ value_json.battery_percentage }}"
    unit_of_measurement: "%"
    device_class: battery
    scan_interval: 300

  - platform: rest
    name: "TRMNL Last Update"
    resource: "https://yourserver.com/api/status"
    value_template: "{{ value_json.last_update }}"
    device_class: timestamp
```

## Example: HA Dashboard on TRMNL

```rust
use axum::{routing::get, Json, Router};
use trmnl::{DeviceInfo, DisplayResponse};
use trmnl::homeassistant::HaClient;
use trmnl::render::{render_html_to_png, RenderConfig};

async fn display(device: DeviceInfo) -> Json<DisplayResponse> {
    let ha = HaClient::from_env(); // Uses HA_URL and HA_TOKEN env vars

    // Fetch states
    let states = ha.get_states(&[
        "sensor.living_room_temperature",
        "sensor.outside_temperature",
        "binary_sensor.front_door",
        "binary_sensor.garage_door",
        "sensor.energy_today",
        "weather.home",
    ]).await.unwrap_or_default();

    // Build HTML
    let html = format!(r#"
        <html>
        <body style="width:800px; height:480px; background:white; padding:20px; font-family:sans-serif;">
            <h1>Home Status</h1>
            <div style="display:flex; gap:40px;">
                <div>
                    <h2>Climate</h2>
                    <p>Inside: {inside_temp}°F</p>
                    <p>Outside: {outside_temp}°F</p>
                    <p>Weather: {weather}</p>
                </div>
                <div>
                    <h2>Security</h2>
                    <p>Front Door: {front_door}</p>
                    <p>Garage: {garage}</p>
                </div>
                <div>
                    <h2>Energy</h2>
                    <p>Today: {energy} kWh</p>
                </div>
            </div>
        </body>
        </html>
    "#,
        inside_temp = states.get("sensor.living_room_temperature").map(|s| &s.state).unwrap_or(&"--".into()),
        outside_temp = states.get("sensor.outside_temperature").map(|s| &s.state).unwrap_or(&"--".into()),
        weather = states.get("weather.home").map(|s| &s.state).unwrap_or(&"--".into()),
        front_door = if states.get("binary_sensor.front_door").map(|s| s.is_on()).unwrap_or(false) { "Open" } else { "Closed" },
        garage = if states.get("binary_sensor.garage_door").map(|s| s.is_on()).unwrap_or(false) { "Open" } else { "Closed" },
        energy = states.get("sensor.energy_today").map(|s| &s.state).unwrap_or(&"--".into()),
    );

    // Render and serve
    let png = render_html_to_png(&html, &RenderConfig::default()).await.unwrap();
    let filename = format!("{}.png", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
    std::fs::write(format!("/var/www/trmnl/{}", filename), &png).unwrap();

    Json(DisplayResponse::new(
        format!("https://myserver.com/trmnl/{}", filename),
        filename,
    ))
}
```

## API Endpoints

### GET /api/status

Returns TRMNL device status (for HA sensors):

```json
{
  "battery_percentage": 85,
  "battery_voltage": 3.95,
  "last_update": "2024-01-15T10:30:00Z",
  "refresh_rate": 300,
  "firmware_version": "1.2.3",
  "wifi_rssi": -45
}
```

### POST /api/webhook/refresh

Triggers immediate display refresh:

```json
{
  "action": "refresh"
}
```

### POST /api/webhook/alert

Displays an alert message:

```json
{
  "message": "Front door open!",
  "duration": 300  // seconds to show alert
}
```

### POST /api/webhook/config

Updates configuration:

```json
{
  "refresh_rate": 60,
  "schedule": "active"  // or "sleep"
}
```

## Implementation Plan

### Phase 1: HaClient (read HA states)
- [ ] `HaClient::new(url, token)`
- [ ] `HaClient::from_env()`
- [ ] `get_state(entity_id)`
- [ ] `get_states(&[entity_ids])`
- [ ] `HaState` struct with helpers

### Phase 2: Webhooks (HA triggers TRMNL)
- [ ] `/api/webhook/refresh` endpoint
- [ ] `/api/webhook/alert` endpoint
- [ ] Token authentication for webhooks

### Phase 3: Status API (TRMNL as HA sensor)
- [ ] `/api/status` endpoint
- [ ] Track last device contact
- [ ] Store battery/signal info

### Phase 4: Documentation
- [ ] HA configuration examples
- [ ] Automation recipes
- [ ] Dashboard template
