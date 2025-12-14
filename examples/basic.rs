//! Basic example of pushing data to a TRMNL display.
//!
//! Run with:
//! ```sh
//! TRMNL_PLUGIN_UUID=your-uuid cargo run --example basic
//! ```

use serde_json::json;
use trmnl::Client;

#[tokio::main]
async fn main() -> Result<(), trmnl::Error> {
    // Create client from environment variable
    let client = Client::from_env().expect("TRMNL_PLUGIN_UUID must be set");

    // Push simple JSON data
    client
        .push(json!({
            "title": "Hello TRMNL!",
            "message": "Pushed from Rust",
            "temperature": 72,
            "humidity": 45
        }))
        .await?;

    println!("Successfully pushed data to TRMNL!");
    Ok(())
}
