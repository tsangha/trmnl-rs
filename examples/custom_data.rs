//! Example using typed structs for TRMNL display data.
//!
//! Run with:
//! ```sh
//! TRMNL_PLUGIN_UUID=your-uuid cargo run --example custom_data
//! ```

use serde::Serialize;
use trmnl::{Client, MergeStrategy};

/// Dashboard data structure
#[derive(Serialize)]
struct Dashboard {
    date: String,
    time: String,
    weather: Weather,
    tasks: Vec<Task>,
    quote: Quote,
}

#[derive(Serialize)]
struct Weather {
    temperature: i32,
    condition: String,
}

#[derive(Serialize)]
struct Task {
    name: String,
    completed: bool,
}

#[derive(Serialize)]
struct Quote {
    text: String,
    author: String,
}

#[tokio::main]
async fn main() -> Result<(), trmnl::Error> {
    let client = Client::from_env().expect("TRMNL_PLUGIN_UUID must be set");

    // Build dashboard data
    let dashboard = Dashboard {
        date: "Monday, January 15".into(),
        time: "09:30".into(),
        weather: Weather {
            temperature: 72,
            condition: "Sunny".into(),
        },
        tasks: vec![
            Task {
                name: "Review PRs".into(),
                completed: true,
            },
            Task {
                name: "Deploy to production".into(),
                completed: false,
            },
            Task {
                name: "Write documentation".into(),
                completed: false,
            },
        ],
        quote: Quote {
            text: "The best time to plant a tree was 20 years ago. The second best time is now."
                .into(),
            author: "Chinese Proverb".into(),
        },
    };

    // Push with deep merge to preserve any other data
    client
        .push_with_options(dashboard, Some(MergeStrategy::DeepMerge), None)
        .await?;

    println!("Dashboard pushed successfully!");
    Ok(())
}
