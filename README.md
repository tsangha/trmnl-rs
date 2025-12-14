# trmnl-rs

A Rust SDK for [TRMNL](https://usetrmnl.com) e-ink displays.

[![Crates.io](https://img.shields.io/crates/v/trmnl.svg)](https://crates.io/crates/trmnl)
[![Documentation](https://docs.rs/trmnl/badge.svg)](https://docs.rs/trmnl)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Features

- **Type-safe** - Push any `Serialize` type to your display
- **Merge strategies** - Replace, deep merge, or stream data
- **Rate limit handling** - Automatic detection of TRMNL's 12 req/hour limit
- **Async** - Built on `reqwest` for efficient async I/O

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
trmnl = "0.1"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
serde_json = "1.0"
```

Push data to your display:

```rust
use trmnl::Client;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), trmnl::Error> {
    let client = Client::new("your-plugin-uuid");

    client.push(json!({
        "temperature": 72,
        "humidity": 45,
        "status": "Online"
    })).await?;

    Ok(())
}
```

## Merge Strategies

TRMNL supports three strategies for updating display data:

```rust
use trmnl::{Client, MergeStrategy};
use serde_json::json;

let client = Client::new("your-plugin-uuid");

// Replace all data (default)
client.push(json!({"temp": 72})).await?;

// Deep merge with existing data
client.push_with_options(
    json!({"weather": {"temp": 72}}),
    Some(MergeStrategy::DeepMerge),
    None,
).await?;

// Stream new items, keeping last 10
client.push_with_options(
    json!({"events": [{"name": "New Event"}]}),
    Some(MergeStrategy::Stream),
    Some(10),
).await?;
```

## Using with Typed Structs

```rust
use serde::Serialize;
use trmnl::Client;

#[derive(Serialize)]
struct Dashboard {
    date: String,
    weather: String,
    tasks: Vec<Task>,
}

#[derive(Serialize)]
struct Task {
    name: String,
    done: bool,
}

let client = Client::new("your-plugin-uuid");

client.push(Dashboard {
    date: "Monday, January 15".into(),
    weather: "72°F Clear".into(),
    tasks: vec![
        Task { name: "Review PRs".into(), done: true },
        Task { name: "Deploy app".into(), done: false },
    ],
}).await?;
```

## Environment Variable

The client can be created from the `TRMNL_PLUGIN_UUID` environment variable:

```rust
if let Some(client) = Client::from_env() {
    client.push(data).await?;
}
```

---

# TRMNL Framework Guide

This section documents the TRMNL Framework CSS classes and best practices for building
e-ink display UIs. These lessons were learned through extensive trial and error.

## Display Constraints

- **Resolution**: 800x480 pixels (landscape) or 480x800 (portrait)
- **Color depth**: 1-bit (black/white) or 4-bit (16 grayscales)
- **No animations** - E-ink refresh is slow (~1-2 seconds)
- **High contrast** is essential for readability
- **Simple layouts** work best

## Critical Lessons Learned

### What DOESN'T Work in Private Plugin Markup

1. **DO NOT wrap in `<div class="screen">`** - TRMNL adds this automatically. Including it breaks the layout.

2. **DO NOT use `<div class="view view--full">`** wrapper - Also added by TRMNL. Including it causes content to overflow.

3. **DO NOT use complex nested layouts** like `layout--row layout--stretch-x` - They often break and push content off-screen.

4. **DO NOT use `item`, `meta`, `content` components** - They render but positioning is unpredictable.

5. **DO NOT use `title_bar`** - It doesn't position correctly in private plugins.

### What DOES Work

1. **Start with `<div class="layout">`** - This is your root container.

2. **Use simple `columns` and `column`** - Basic two-column layouts work reliably.

3. **Use basic typography**: `title`, `title--small`, `description`, `label`, `label--small`

4. **Use plain HTML for simple layouts** - `<div>`, `<strong>`, `<small>`, `<br>` work fine.

5. **Use inline styles sparingly** - `style="margin-top:12px"` is more reliable than complex CSS classes.

6. **Keep it simple** - The simpler the markup, the better it renders.

## Template Structure

### Basic Template (Recommended)

```html
<div class="layout">
  <div class="title">{{ title }}</div>
  <div class="description">{{ description }}</div>

  <div class="columns">
    <div class="column">
      <div class="title title--small">Left Column</div>
      {% for item in left_items %}
        <div><strong>{{ item.name }}</strong>: {{ item.value }}</div>
      {% endfor %}
    </div>

    <div class="column">
      <div class="title title--small">Right Column</div>
      {% for item in right_items %}
        <div><strong>{{ item.name }}</strong>: {{ item.value }}</div>
      {% endfor %}
    </div>
  </div>

  <small style="margin-top:12px">Updated: {{ time }}</small>
</div>
```

### Typography Classes

| Class | Use For |
|-------|---------|
| `title` | Large heading text |
| `title--small` | Section headers |
| `label` | Medium text for labels |
| `label--small` | Smaller label text |
| `description` | Body/paragraph text |
| `value` | Numeric values (use sparingly, can be finicky) |

## Complete Dashboard Example

This template works reliably for a personal dashboard:

```html
<div class="layout">
  <!-- Header -->
  <div style="display:flex; justify-content:space-between; margin-bottom:12px">
    <span class="title">{{ date }}</span>
    <span class="label">{{ weather }}</span>
  </div>

  <!-- Two Column Layout -->
  <div class="columns">
    <!-- Left: Stats -->
    <div class="column">
      <div class="title title--small">Stats</div>
      {% for station in stations %}
        <div style="display:flex; justify-content:space-between">
          <span class="label">{{ station.name }}</span>
          <span class="title title--small">{{ station.value }}</span>
        </div>
      {% endfor %}

      <div class="title title--small" style="margin-top:12px">Tasks</div>
      {% for task in tasks limit:5 %}
        <div class="label">{{ task.name }}</div>
      {% endfor %}
    </div>

    <!-- Right: Message -->
    <div class="column">
      <div class="title title--small">Today</div>
      <div class="description">{{ briefing | truncate: 200 }}</div>

      <div style="margin-top:12px; padding:8px; border:1px solid black">
        <div class="description">"{{ quote }}"</div>
        <small>- {{ author }}</small>
      </div>
    </div>
  </div>

  <!-- Footer -->
  <div style="display:flex; justify-content:space-between; margin-top:12px">
    <small>My Dashboard</small>
    <small>{{ time }}</small>
  </div>
</div>
```

## Liquid Template Reference

### Variables

```liquid
{{ variable }}
{{ object.property }}
{{ array[0] }}
```

### Loops

```liquid
{% for item in items %}
  {{ forloop.index }}. {{ item.name }}
{% endfor %}

{% for item in items limit:5 %}
  {{ item.name }}
{% endfor %}
```

### Conditionals

```liquid
{% if condition %}
  Content
{% elsif other_condition %}
  Other content
{% else %}
  Fallback
{% endif %}

{% unless empty %}
  Has content
{% endunless %}
```

### Filters

```liquid
{{ text | truncate: 100 }}
{{ text | upcase }}
{{ text | downcase }}
{{ number | plus: 1 }}
{{ array | size }}
{{ array | first }}
{{ array | last }}
{{ date | date: "%B %d, %Y" }}
```

### TRMNL-Specific Variables

```liquid
{{ trmnl.user.locale }}
{{ trmnl.user.utc_offset }}
{{ trmnl.plugin_settings.instance_name }}
```

## Integration Modes

### Polling Mode

TRMNL polls your endpoint periodically. You return JSON that TRMNL renders using your Liquid template.

**Your Rust server:**
```rust
use axum::{Json, extract::Query};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
struct DisplayQuery {
    token: Option<String>,
}

#[derive(Serialize)]
struct DisplayData {
    date: String,
    weather: String,
    tasks: Vec<Task>,
}

async fn get_display(Query(q): Query<DisplayQuery>) -> Json<DisplayData> {
    // Validate token...
    Json(DisplayData {
        date: "Monday, January 15".into(),
        weather: "72°F Clear".into(),
        tasks: vec![/* ... */],
    })
}
```

**Plugin settings:**
```yaml
strategy: polling
polling_url: https://your-server.com/display?token=secret
polling_verb: GET
refresh_interval: 60
```

### Push/Webhook Mode

Your server pushes data to TRMNL when it changes.

```rust
use trmnl::Client;

let client = Client::new("your-plugin-uuid");

// Push whenever data changes
client.push(json!({
    "temperature": 72,
    "updated_at": "10:30 AM"
})).await?;
```

**Plugin settings:**
```yaml
strategy: webhook
# No polling URL needed - you push to TRMNL
```

## E-ink Design Best Practices

1. **Maximize contrast** - Use pure black and white when possible
2. **Avoid thin lines** - Minimum 2px for visibility
3. **Large text** - Text should be readable from arm's length
4. **Simple layouts** - Less is more on a small e-ink display
5. **Test on device** - Emulators may not show dithering accurately
6. **Use semantic hierarchy** - `title` > `label` > `description` for visual weight
7. **Limit content** - Don't try to cram too much information

## Resources

- [TRMNL Design System](https://usetrmnl.com/framework)
- [Private Plugins Docs](https://docs.usetrmnl.com/go/private-plugins)
- [Webhooks API](https://docs.usetrmnl.com/go/private-plugins/webhooks)
- [Template Guide](https://docs.usetrmnl.com/go/private-plugins/templates)
- [Liquid 101](https://help.usetrmnl.com/en/articles/10671186-liquid-101)

## License

MIT
