//! Refresh rate scheduling based on time of day and day of week.
//!
//! This module allows you to configure different refresh rates for different times,
//! helping optimize battery life while keeping displays fresh when needed.
//!
//! # Example Schedule (YAML)
//!
//! ```yaml
//! timezone: "America/New_York"
//! default_refresh_rate: 300  # 5 minutes
//!
//! schedule:
//!   # Sleep hours - very infrequent updates
//!   - days: all
//!     start: "23:00"
//!     end: "06:00"
//!     refresh_rate: 1800  # 30 minutes
//!
//!   # Morning routine - frequent updates
//!   - days: weekdays
//!     start: "06:00"
//!     end: "09:00"
//!     refresh_rate: 60  # 1 minute
//!
//!   # Work hours - moderate updates
//!   - days: weekdays
//!     start: "09:00"
//!     end: "18:00"
//!     refresh_rate: 120  # 2 minutes
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! use trmnl::schedule::RefreshSchedule;
//!
//! // Load schedule from YAML file
//! let schedule = RefreshSchedule::load("config/schedule.yaml")?;
//!
//! // Get current refresh rate based on time
//! let refresh_rate = schedule.get_refresh_rate();
//!
//! // Use in your display response
//! DisplayResponse::new(url, filename).with_refresh_rate(refresh_rate)
//! ```

use chrono::{DateTime, Datelike, NaiveTime, Timelike, Utc, Weekday};
use chrono_tz::Tz;
use serde::Deserialize;
use std::path::Path;

use crate::Error;

/// A refresh rate schedule configuration.
///
/// Loads from YAML and provides time-based refresh rate lookup.
#[derive(Debug, Clone, Deserialize)]
pub struct RefreshSchedule {
    /// Timezone for interpreting times (e.g., "America/New_York")
    pub timezone: String,
    /// Default refresh rate if no rule matches (seconds)
    pub default_refresh_rate: u32,
    /// List of schedule rules (evaluated in order, first match wins)
    pub schedule: Vec<ScheduleRule>,
}

/// A single schedule rule.
#[derive(Debug, Clone, Deserialize)]
pub struct ScheduleRule {
    /// Days this rule applies to
    pub days: DaySelector,
    /// Start time (HH:MM, 24-hour format)
    pub start: String,
    /// End time (HH:MM, 24-hour format)
    pub end: String,
    /// Refresh rate in seconds
    pub refresh_rate: u32,
}

/// Day selector for schedule rules.
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum DaySelector {
    /// A specific list of days (e.g., ["mon", "tue", "wed"])
    List(Vec<String>),
    /// A named group: "all", "weekdays", "weekends", or a single day name
    Named(String),
}

impl RefreshSchedule {
    /// Load schedule from a YAML file.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let schedule = RefreshSchedule::load("config/schedule.yaml")?;
    /// ```
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let content = std::fs::read_to_string(path.as_ref()).map_err(|e| {
            Error::Config(format!(
                "Failed to read schedule file '{}': {}",
                path.as_ref().display(),
                e
            ))
        })?;
        Self::from_yaml(&content)
    }

    /// Parse schedule from a YAML string.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let yaml = r#"
    /// timezone: "UTC"
    /// default_refresh_rate: 300
    /// schedule: []
    /// "#;
    /// let schedule = RefreshSchedule::from_yaml(yaml)?;
    /// ```
    pub fn from_yaml(yaml: &str) -> Result<Self, Error> {
        serde_yaml::from_str(yaml)
            .map_err(|e| Error::Config(format!("Invalid schedule YAML: {}", e)))
    }

    /// Get the refresh rate for the current time.
    ///
    /// Evaluates rules in order and returns the first match,
    /// or `default_refresh_rate` if no rules match.
    pub fn get_refresh_rate(&self) -> u32 {
        let tz: Tz = self
            .timezone
            .parse()
            .unwrap_or(chrono_tz::America::New_York);
        let now = Utc::now().with_timezone(&tz);
        self.get_refresh_rate_for_time(now)
    }

    /// Get the refresh rate for a specific time.
    ///
    /// Useful for testing or for pre-calculating schedules.
    pub fn get_refresh_rate_for_time<T: chrono::TimeZone>(&self, dt: DateTime<T>) -> u32 {
        let weekday = dt.weekday();
        let time = NaiveTime::from_hms_opt(dt.hour(), dt.minute(), 0).unwrap_or_default();

        for rule in &self.schedule {
            if rule.matches(weekday, time) {
                tracing::debug!(
                    "Schedule rule matched: {:?} {} -> {} refresh_rate={}",
                    rule.days,
                    rule.start,
                    rule.end,
                    rule.refresh_rate
                );
                return rule.refresh_rate;
            }
        }

        tracing::debug!(
            "No schedule rule matched, using default: {}",
            self.default_refresh_rate
        );
        self.default_refresh_rate
    }
}

impl ScheduleRule {
    /// Check if this rule matches the given day and time.
    fn matches(&self, weekday: Weekday, time: NaiveTime) -> bool {
        // Check if the day matches
        if !self.day_matches(weekday) {
            return false;
        }

        // Parse start and end times
        let start = parse_time(&self.start);
        let end = parse_time(&self.end);

        match (start, end) {
            (Some(s), Some(e)) => {
                if s <= e {
                    // Normal range (e.g., 09:00 - 17:00)
                    time >= s && time < e
                } else {
                    // Overnight range (e.g., 23:00 - 06:00)
                    time >= s || time < e
                }
            }
            _ => false,
        }
    }

    /// Check if this rule applies to the given weekday.
    fn day_matches(&self, weekday: Weekday) -> bool {
        match &self.days {
            DaySelector::Named(name) => match name.to_lowercase().as_str() {
                "all" => true,
                "weekdays" => matches!(
                    weekday,
                    Weekday::Mon | Weekday::Tue | Weekday::Wed | Weekday::Thu | Weekday::Fri
                ),
                "weekends" => matches!(weekday, Weekday::Sat | Weekday::Sun),
                _ => {
                    // Single day name
                    weekday_from_str(name) == Some(weekday)
                }
            },
            DaySelector::List(days) => days.iter().any(|d| weekday_from_str(d) == Some(weekday)),
        }
    }
}

/// Parse a time string (HH:MM) into NaiveTime.
fn parse_time(s: &str) -> Option<NaiveTime> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 2 {
        return None;
    }
    let hour: u32 = parts[0].parse().ok()?;
    let minute: u32 = parts[1].parse().ok()?;
    NaiveTime::from_hms_opt(hour, minute, 0)
}

/// Convert a day name to Weekday.
fn weekday_from_str(s: &str) -> Option<Weekday> {
    match s.to_lowercase().as_str() {
        "mon" | "monday" => Some(Weekday::Mon),
        "tue" | "tuesday" => Some(Weekday::Tue),
        "wed" | "wednesday" => Some(Weekday::Wed),
        "thu" | "thursday" => Some(Weekday::Thu),
        "fri" | "friday" => Some(Weekday::Fri),
        "sat" | "saturday" => Some(Weekday::Sat),
        "sun" | "sunday" => Some(Weekday::Sun),
        _ => None,
    }
}

// =============================================================================
// Global Schedule (optional convenience pattern)
// =============================================================================

use std::sync::OnceLock;

/// Global schedule instance, loaded once at startup.
static SCHEDULE: OnceLock<Option<RefreshSchedule>> = OnceLock::new();

/// Initialize the global schedule from a file.
///
/// Call this once at application startup. If the file doesn't exist
/// or is invalid, a warning is logged and the default rate will be used.
///
/// # Example
///
/// ```rust,ignore
/// trmnl::schedule::init_global_schedule("config/schedule.yaml");
///
/// // Later, in handlers:
/// let rate = trmnl::schedule::get_global_refresh_rate();
/// ```
pub fn init_global_schedule(path: &str) {
    let schedule = match RefreshSchedule::load(path) {
        Ok(s) => {
            tracing::info!(
                "Loaded TRMNL schedule with {} rules, default={}s",
                s.schedule.len(),
                s.default_refresh_rate
            );
            Some(s)
        }
        Err(e) => {
            tracing::warn!("Failed to load TRMNL schedule: {}", e);
            None
        }
    };
    let _ = SCHEDULE.set(schedule);
}

/// Get the current refresh rate based on the global schedule.
///
/// Returns 60 seconds if no schedule is loaded.
pub fn get_global_refresh_rate() -> u32 {
    const DEFAULT_REFRESH_RATE: u32 = 60;

    match SCHEDULE.get() {
        Some(Some(schedule)) => schedule.get_refresh_rate(),
        _ => DEFAULT_REFRESH_RATE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_time() {
        assert_eq!(parse_time("09:00"), NaiveTime::from_hms_opt(9, 0, 0));
        assert_eq!(parse_time("23:30"), NaiveTime::from_hms_opt(23, 30, 0));
        assert_eq!(parse_time("invalid"), None);
        assert_eq!(parse_time("12"), None);
    }

    #[test]
    fn test_weekday_from_str() {
        assert_eq!(weekday_from_str("mon"), Some(Weekday::Mon));
        assert_eq!(weekday_from_str("Monday"), Some(Weekday::Mon));
        assert_eq!(weekday_from_str("MON"), Some(Weekday::Mon));
        assert_eq!(weekday_from_str("sat"), Some(Weekday::Sat));
        assert_eq!(weekday_from_str("invalid"), None);
    }

    #[test]
    fn test_schedule_rule_day_match_named() {
        let rule = ScheduleRule {
            days: DaySelector::Named("weekdays".to_string()),
            start: "09:00".to_string(),
            end: "17:00".to_string(),
            refresh_rate: 60,
        };
        assert!(rule.day_matches(Weekday::Mon));
        assert!(rule.day_matches(Weekday::Fri));
        assert!(!rule.day_matches(Weekday::Sat));
        assert!(!rule.day_matches(Weekday::Sun));
    }

    #[test]
    fn test_schedule_rule_day_match_list() {
        let rule = ScheduleRule {
            days: DaySelector::List(vec![
                "mon".to_string(),
                "wed".to_string(),
                "fri".to_string(),
            ]),
            start: "09:00".to_string(),
            end: "17:00".to_string(),
            refresh_rate: 60,
        };
        assert!(rule.day_matches(Weekday::Mon));
        assert!(rule.day_matches(Weekday::Wed));
        assert!(rule.day_matches(Weekday::Fri));
        assert!(!rule.day_matches(Weekday::Tue));
        assert!(!rule.day_matches(Weekday::Sat));
    }

    #[test]
    fn test_schedule_rule_time_match() {
        let rule = ScheduleRule {
            days: DaySelector::Named("all".to_string()),
            start: "09:00".to_string(),
            end: "17:00".to_string(),
            refresh_rate: 60,
        };
        let time_10am = NaiveTime::from_hms_opt(10, 0, 0).unwrap();
        let time_8am = NaiveTime::from_hms_opt(8, 0, 0).unwrap();
        let time_6pm = NaiveTime::from_hms_opt(18, 0, 0).unwrap();
        let time_5pm = NaiveTime::from_hms_opt(17, 0, 0).unwrap();

        assert!(rule.matches(Weekday::Mon, time_10am));
        assert!(!rule.matches(Weekday::Mon, time_8am));
        assert!(!rule.matches(Weekday::Mon, time_6pm));
        assert!(!rule.matches(Weekday::Mon, time_5pm)); // End is exclusive
    }

    #[test]
    fn test_overnight_rule() {
        let rule = ScheduleRule {
            days: DaySelector::Named("all".to_string()),
            start: "23:00".to_string(),
            end: "06:00".to_string(),
            refresh_rate: 1800,
        };
        let time_midnight = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
        let time_3am = NaiveTime::from_hms_opt(3, 0, 0).unwrap();
        let time_11pm = NaiveTime::from_hms_opt(23, 30, 0).unwrap();
        let time_noon = NaiveTime::from_hms_opt(12, 0, 0).unwrap();

        assert!(rule.matches(Weekday::Mon, time_midnight));
        assert!(rule.matches(Weekday::Mon, time_3am));
        assert!(rule.matches(Weekday::Mon, time_11pm));
        assert!(!rule.matches(Weekday::Mon, time_noon));
    }

    #[test]
    fn test_from_yaml() {
        let yaml = r#"
timezone: "America/New_York"
default_refresh_rate: 300
schedule:
  - days: weekdays
    start: "09:00"
    end: "17:00"
    refresh_rate: 60
  - days: all
    start: "23:00"
    end: "06:00"
    refresh_rate: 1800
"#;
        let schedule = RefreshSchedule::from_yaml(yaml).unwrap();
        assert_eq!(schedule.timezone, "America/New_York");
        assert_eq!(schedule.default_refresh_rate, 300);
        assert_eq!(schedule.schedule.len(), 2);
        assert_eq!(schedule.schedule[0].refresh_rate, 60);
        assert_eq!(schedule.schedule[1].refresh_rate, 1800);
    }

    #[test]
    fn test_empty_schedule_returns_default() {
        let yaml = r#"
timezone: "UTC"
default_refresh_rate: 300
schedule: []
"#;
        let schedule = RefreshSchedule::from_yaml(yaml).unwrap();
        assert_eq!(schedule.get_refresh_rate(), 300);
    }
}
