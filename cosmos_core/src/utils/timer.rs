//! Used for debugging only
//!
//! A very basic timer

use std::time::SystemTime;

use bevy::log::info;

/// Used for debugging - logs the difference in time
pub struct UtilsTimer {
    start: SystemTime,
}

impl UtilsTimer {
    /// Starts the timer
    pub fn start() -> Self {
        Self { start: SystemTime::now() }
    }

    /// Resets the timer
    pub fn reset(&mut self) {
        self.start = SystemTime::now();
    }

    /// info! the difference in time - does not reset timer.
    pub fn log_duration(&self, message: &str) {
        info!(
            "{} {}ms",
            message,
            SystemTime::now().duration_since(self.start).unwrap().as_millis()
        );
    }

    /// info! the difference in time - does not reset timer.
    pub fn log_duration_if_at_least(&self, message: &str, min_millis: u128) {
        let ms = SystemTime::now().duration_since(self.start).unwrap().as_millis();
        if ms >= min_millis {
            info!("{} {ms}ms", message);
        }
    }
}
