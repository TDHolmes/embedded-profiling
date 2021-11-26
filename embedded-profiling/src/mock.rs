use crate::EPInstant;

pub struct StdMockProfiler {
    start: std::time::Instant,
}

impl core::default::Default for StdMockProfiler {
    fn default() -> Self {
        Self {
            start: std::time::Instant::now(),
        }
    }
}

impl super::EmbeddedProfiler for StdMockProfiler {
    fn read_clock(&self) -> crate::EPInstant {
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.start);

        EPInstant::from_ticks(elapsed.as_micros().try_into().unwrap())
    }

    fn log_snapshot(&self, snapshot: &crate::EPSnapshot) {
        println!("{}", snapshot);
    }
}
