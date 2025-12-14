//! HTML to PNG rendering for TRMNL displays.
//!
//! Uses Chrome headless to render HTML to PNG images suitable for e-ink displays.
//!
//! # Requirements
//!
//! - Google Chrome or Chromium must be installed
//! - ImageMagick (`convert` command) for image optimization (optional but recommended)
//!
//! # Example
//!
//! ```rust,ignore
//! use trmnl::render::{RenderConfig, render_html_to_png};
//!
//! let html = r#"
//!     <html>
//!     <body style="background: white; color: black;">
//!         <h1>Hello TRMNL!</h1>
//!     </body>
//!     </html>
//! "#;
//!
//! let config = RenderConfig::default();
//! let png_data = render_html_to_png(html, &config).await?;
//! ```

use std::path::PathBuf;

use tokio::process::Command;

use crate::error::Error;
use crate::{DISPLAY_HEIGHT, DISPLAY_WIDTH, MAX_IMAGE_SIZE};

/// Configuration for HTML rendering.
#[derive(Debug, Clone)]
pub struct RenderConfig {
    /// Path to Chrome executable (default: "google-chrome")
    pub chrome_path: String,

    /// Directory for temporary files (default: "/tmp/trmnl")
    pub temp_dir: PathBuf,

    /// Whether to optimize images for e-ink (reduce colors, default: true)
    pub optimize: bool,

    /// Number of colors for optimized images (default: 16)
    pub color_depth: u32,

    /// Display width (default: 800)
    pub width: u32,

    /// Display height (default: 480)
    pub height: u32,
}

impl Default for RenderConfig {
    fn default() -> Self {
        Self {
            chrome_path: std::env::var("CHROME_PATH")
                .unwrap_or_else(|_| "google-chrome".to_string()),
            temp_dir: PathBuf::from("/tmp/trmnl"),
            optimize: true,
            color_depth: 16,
            width: DISPLAY_WIDTH,
            height: DISPLAY_HEIGHT,
        }
    }
}

impl RenderConfig {
    /// Create config with custom Chrome path.
    pub fn with_chrome_path(mut self, path: impl Into<String>) -> Self {
        self.chrome_path = path.into();
        self
    }

    /// Create config with custom temp directory.
    pub fn with_temp_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.temp_dir = path.into();
        self
    }

    /// Disable image optimization.
    pub fn without_optimization(mut self) -> Self {
        self.optimize = false;
        self
    }
}

/// Render HTML to PNG using Chrome headless.
///
/// # Arguments
///
/// * `html` - HTML content to render
/// * `config` - Render configuration
///
/// # Returns
///
/// PNG image data as bytes.
///
/// # Errors
///
/// Returns error if:
/// - Chrome is not found or fails
/// - File I/O fails
/// - Image is too large (>90KB after optimization)
///
/// # Example
///
/// ```rust,ignore
/// use trmnl::render::{RenderConfig, render_html_to_png};
///
/// let html = "<html><body><h1>Hello!</h1></body></html>";
/// let png = render_html_to_png(html, &RenderConfig::default()).await?;
/// ```
pub async fn render_html_to_png(html: &str, config: &RenderConfig) -> Result<Vec<u8>, Error> {
    // Ensure temp directory exists
    tokio::fs::create_dir_all(&config.temp_dir)
        .await
        .map_err(|e| Error::Io(format!("Failed to create temp dir: {}", e)))?;

    let html_path = config.temp_dir.join("render.html");
    let screenshot_path = config.temp_dir.join("screenshot.png");
    let optimized_path = config.temp_dir.join("optimized.png");
    let chrome_data_dir = config.temp_dir.join("chrome-data");

    // Write HTML file
    tokio::fs::write(&html_path, html)
        .await
        .map_err(|e| Error::Io(format!("Failed to write HTML: {}", e)))?;

    // Ensure chrome data dir exists
    tokio::fs::create_dir_all(&chrome_data_dir)
        .await
        .map_err(|e| Error::Io(format!("Failed to create chrome data dir: {}", e)))?;

    let html_url = format!("file://{}", html_path.display());

    // Run Chrome headless
    let output = Command::new(&config.chrome_path)
        .arg("--headless=new")
        .arg("--no-sandbox")
        .arg("--disable-gpu")
        .arg("--disable-dev-shm-usage")
        .arg("--disable-software-rasterizer")
        .arg("--no-first-run")
        .arg("--disable-extensions")
        .arg("--disable-background-networking")
        .arg("--force-device-scale-factor=1")
        .arg("--hide-scrollbars")
        .arg("--default-background-color=ffffffff")
        .arg(format!("--user-data-dir={}", chrome_data_dir.display()))
        .arg(format!(
            "--window-size={},{}",
            config.width,
            config.height + 100 // Extra height for scrollbar avoidance
        ))
        .arg(format!("--screenshot={}", screenshot_path.display()))
        .arg(&html_url)
        .output()
        .await
        .map_err(|e| Error::Chrome(format!("Failed to run Chrome: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!("Chrome stderr: {}", stderr);
    }

    // Check if screenshot was created
    if !tokio::fs::try_exists(&screenshot_path)
        .await
        .unwrap_or(false)
    {
        return Err(Error::Chrome(
            "Chrome did not create screenshot".to_string(),
        ));
    }

    // Optimize if requested
    let final_path = if config.optimize {
        // Try to optimize with ImageMagick
        let convert_result = Command::new("convert")
            .arg(&screenshot_path)
            .arg("-crop")
            .arg(format!("{}x{}+0+0", config.width, config.height))
            .arg("+repage")
            .arg("-colors")
            .arg(config.color_depth.to_string())
            .arg("-depth")
            .arg("4")
            .arg(&optimized_path)
            .output()
            .await;

        match convert_result {
            Ok(output) if output.status.success() => {
                if tokio::fs::try_exists(&optimized_path)
                    .await
                    .unwrap_or(false)
                {
                    optimized_path
                } else {
                    screenshot_path
                }
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                tracing::warn!("ImageMagick optimization failed: {}", stderr);
                screenshot_path
            }
            Err(e) => {
                tracing::warn!("ImageMagick not available: {}", e);
                screenshot_path
            }
        }
    } else {
        screenshot_path
    };

    // Read the final image
    let png_data = tokio::fs::read(&final_path)
        .await
        .map_err(|e| Error::Io(format!("Failed to read screenshot: {}", e)))?;

    tracing::info!("Rendered PNG: {} bytes", png_data.len());

    // Check size
    if png_data.len() > MAX_IMAGE_SIZE {
        return Err(Error::ImageTooLarge {
            size: png_data.len(),
            max: MAX_IMAGE_SIZE,
        });
    }

    Ok(png_data)
}

/// Generate a timestamped filename for cache busting.
///
/// The TRMNL firmware compares filenames to detect new images.
/// Using timestamps ensures the device always fetches new content.
///
/// # Example
///
/// ```
/// use trmnl::render::timestamped_filename;
///
/// let filename = timestamped_filename();
/// assert!(filename.ends_with(".png"));
/// ```
pub fn timestamped_filename() -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{}.png", timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_config_defaults() {
        let config = RenderConfig::default();
        assert_eq!(config.width, 800);
        assert_eq!(config.height, 480);
        assert!(config.optimize);
        assert_eq!(config.color_depth, 16);
    }

    #[test]
    fn test_timestamped_filename() {
        let filename = timestamped_filename();
        assert!(filename.ends_with(".png"));
        // Should be a valid number
        let stem = filename.trim_end_matches(".png");
        assert!(stem.parse::<u64>().is_ok());
    }

    #[test]
    fn test_config_builder() {
        let config = RenderConfig::default()
            .with_chrome_path("/usr/bin/chromium")
            .with_temp_dir("/var/tmp/trmnl")
            .without_optimization();

        assert_eq!(config.chrome_path, "/usr/bin/chromium");
        assert_eq!(config.temp_dir, PathBuf::from("/var/tmp/trmnl"));
        assert!(!config.optimize);
    }
}
