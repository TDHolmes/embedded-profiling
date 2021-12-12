use crate::EPInstant;
use core::sync::{atomic, atomic::Ordering::SeqCst};

// an enum would fit better here, but we want the atomicity of atomics so we can
//   have interior mutability
pub struct CalledFunc {
    pub called: atomic::AtomicBool,
    pub at: atomic::AtomicUsize,
}

impl core::default::Default for CalledFunc {
    fn default() -> Self {
        Self {
            called: atomic::AtomicBool::new(false),
            at: atomic::AtomicUsize::new(0),
        }
    }
}

pub struct CalledFuncs {
    pub count: atomic::AtomicUsize,
    pub read_clock: CalledFunc,
    pub log_snapshot: CalledFunc,
    pub at_start: CalledFunc,
    pub at_end: CalledFunc,
}

impl core::default::Default for CalledFuncs {
    fn default() -> Self {
        Self {
            count: atomic::AtomicUsize::new(0),
            read_clock: Default::default(),
            log_snapshot: Default::default(),
            at_start: Default::default(),
            at_end: Default::default(),
        }
    }
}

pub struct StdMockProfiler {
    start: std::time::Instant,
    pub funcs_called: CalledFuncs,
}

impl core::default::Default for StdMockProfiler {
    fn default() -> Self {
        Self {
            start: std::time::Instant::now(),
            funcs_called: Default::default(),
        }
    }
}

impl super::EmbeddedProfiler for StdMockProfiler {
    fn read_clock(&self) -> crate::EPInstant {
        // First, log that we've been called and when
        if !self.funcs_called.read_clock.called.load(SeqCst) {
            let when = self.funcs_called.count.load(SeqCst);

            self.funcs_called.read_clock.called.store(true, SeqCst);
            self.funcs_called.read_clock.at.store(when, SeqCst);
            self.funcs_called.count.store(when + 1, SeqCst);
        }

        // now actually do the profiler stuff
        let now = std::time::Instant::now();
        let elapsed = now.duration_since(self.start);

        EPInstant::from_ticks(elapsed.as_micros().try_into().unwrap())
    }

    fn log_snapshot(&self, snapshot: &crate::EPSnapshot) {
        // First, log that we've been called and when
        if !self.funcs_called.log_snapshot.called.load(SeqCst) {
            let when = self.funcs_called.count.load(SeqCst);

            self.funcs_called.log_snapshot.called.store(true, SeqCst);
            self.funcs_called.log_snapshot.at.store(when, SeqCst);
            self.funcs_called.count.store(when + 1, SeqCst);
        }

        // now actually do the profiler stuff
        println!("{}", snapshot);
    }

    fn at_start(&self) {
        // First, log that we've been called and when
        if !self.funcs_called.at_start.called.load(SeqCst) {
            let when = self.funcs_called.count.load(SeqCst);

            self.funcs_called.at_start.called.store(true, SeqCst);
            self.funcs_called.at_start.at.store(when, SeqCst);
            self.funcs_called.count.store(when + 1, SeqCst);
        }
    }

    fn at_end(&self) {
        // First, log that we've been called and when
        if !self.funcs_called.at_end.called.load(SeqCst) {
            let when = self.funcs_called.count.load(SeqCst);

            self.funcs_called.at_end.called.store(true, SeqCst);
            self.funcs_called.at_end.at.store(when, SeqCst);
            self.funcs_called.count.store(when + 1, SeqCst);
        }
    }
}
