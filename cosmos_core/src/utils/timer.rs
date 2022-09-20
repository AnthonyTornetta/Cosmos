use std::time::SystemTime;

pub struct UtilsTimer {
    start: SystemTime,
}

impl UtilsTimer {
    pub fn start() -> Self {
        Self {
            start: SystemTime::now(),
        }
    }

    pub fn reset(&mut self) {
        self.start = SystemTime::now();
    }

    pub fn log_duration(&self, message: &str) {
        println!(
            "{} {}ms",
            message,
            SystemTime::now()
                .duration_since(self.start)
                .unwrap()
                .as_millis()
        )
    }
}
