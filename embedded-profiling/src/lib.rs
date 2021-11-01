//! # Embedded-Profiling
//!
//! A lightweight framework for profiling functions, geared towards
//! `no-std` embedded environments. Initialization is very similar
//! to how the `log` crate is initialized. By default, there is a
//! no-op profiler that does nothing until you call [`set_profiler`].
//! Once your profiler has been installed, your profiling
//! functionality will be in use.
//!
//! ## Usage
//!
//! You can manually start & end your snapshot:
//! ```
//! let start = embedded_profiling::start_snapshot();
//! // (...) some expensive computation
//! let snapshot = embedded_profiling::end_snapshot(start, "name-of-computation");
//! // Optionally, log it
//! embedded_profiling::log_snapshot(&snapshot);
//! ```
//!
//! Or profile some code in a closure:
//! ```
//! embedded_profiling::profile("profile println", || {
//!     println!("profiling this closure");
//! });
//! ```
//!
//! ## With a Procedural Macro
//!
//! With the `proc-macros` feature enabled, you can simply annotate
//! the target function with the procedural macro
//! [`profile_function`](embedded_profiling_proc_macros::profile_function).
//! Note that you must first set your profiler with the [`set_profiler`]
//! function.
//! ```
//! # #[cfg(feature = "proc-macros")]
//! #[embedded_profiling::profile_function]
//! fn my_long_running_function() {
//!     println!("Hello, world!");
//! }
//! ```
#![warn(missing_docs)]
#![cfg_attr(not(test), no_std)]

use core::sync::atomic::{AtomicU8, Ordering};

#[cfg(feature = "dwt-systick")]
pub mod dwt_systick;
#[cfg(test)]
mod mock;
#[cfg(feature = "proc-macros")]
pub use embedded_profiling_proc_macros::profile_function;

pub use fugit;

// do the feature gating on a private type so our public documentation is only in one place
#[cfg(feature = "container-u32")]
type PrivContainer = u32;
#[cfg(all(feature = "container-u64", not(feature = "container-u32")))]
type PrivContainer = u64;

// the `not(feature = "container-u32")` clause is so we can successfully use `--all-features`.
/// The underlying container of our [`Duration`](fugit::Duration)/[`Instant`](fugit::Instant) types.
/// Can be either `u32` or `u64`, depending on features. (default: `u32`)
pub type EPContainer = PrivContainer;

/// Our [`Duration`](fugit::Duration) type, representing time elapsed in microseconds
pub type EPDuration = fugit::MicrosDuration<EPContainer>;

/// Our [`Instant`](fugit::Instant) type, representing a snapshot in time from
/// a clock with 1 µs precision (or at least, converted to this representation)
pub type EPInstant = fugit::Instant<EPContainer, 1, 1_000_000>;

/// A recorded snapshot
pub struct EPSnapshot {
    /// The name of this trace
    pub name: &'static str,
    /// The duration of this trace
    pub duration: EPDuration,
}

impl core::fmt::Display for EPSnapshot {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "<EPSS {}: {}>", self.name, self.duration)
    }
}

/// The main trait to implement. All that is required is a way to read time and a way
/// to output our results, if desired. You can also implement functions that
/// get called when a snapshot starts and ends.
pub trait EmbeddedProfiler {
    /// Takes a reading from the clock
    fn read_clock(&self) -> EPInstant;

    /// Optionally reset the clock to zero. This function will be called at the beginning of
    /// [`start_snapshot`].
    ///
    /// TODO: not sure if this API is worth while or not.
    fn reset_clock(&mut self) {}

    /// Optionally log the snapshot to some output, like a serial port
    fn log_snapshot(&self, _snapshot: &EPSnapshot) {}

    /// Optional function that gets called at the start of the snapshot recording.
    /// If one would want to very simple profiling, they could use `at_start` and `at_end`
    /// to simply toggle a GPIO.
    fn at_start(&self) {}

    /// Optional function that gets called at the end of the snapshot recording
    fn at_end(&self) {}

    /// takes the starting snapshot of a specific trace
    fn start_snapshot(&mut self) -> EPInstant {
        self.reset_clock();
        self.at_start();
        self.read_clock()
    }

    /// computes the duration of the snapshot given the start time
    fn end_snapshot(&self, start: EPInstant, name: &'static str) -> EPSnapshot {
        self.at_end();
        let now = self.read_clock();
        let duration = now.checked_duration_since(start).unwrap();

        EPSnapshot { name, duration }
    }
}

struct NoopProfiler;

impl EmbeddedProfiler for NoopProfiler {
    fn read_clock(&self) -> EPInstant {
        EPInstant::from_ticks(0)
    }

    fn log_snapshot(&self, _snapshot: &EPSnapshot) {}
}

static mut PROFILER: &dyn EmbeddedProfiler = &NoopProfiler;

const UNINITIALIZED: u8 = 0;
const INITIALIZED: u8 = 2;

static STATE: AtomicU8 = AtomicU8::new(UNINITIALIZED);

/// Indicates that setting the profiler has gone awry, probably because the
/// profiler has already been set.
#[derive(Debug)]
pub struct SetProfilerError;

/// Sets the global profiler.
///
/// # Safety
/// Must be completed with no other threads running
/// or, in an embedded single core environment, with interrupts disabled.
///
/// # Errors
/// returns `Err(SetProfilerError)` when a global profiler has already been configured
pub unsafe fn set_profiler(
    profiler: &'static dyn EmbeddedProfiler,
) -> Result<(), SetProfilerError> {
    match STATE.load(Ordering::Acquire) {
        UNINITIALIZED => {
            PROFILER = profiler;
            STATE.store(INITIALIZED, Ordering::Release);
            Ok(())
        }
        INITIALIZED => Err(SetProfilerError),
        _ => unreachable!(),
    }
}

/// takes the starting snapshot of a specific trace
pub fn start_snapshot() -> EPInstant {
    unsafe { PROFILER }.read_clock()
}

/// computes the duration of the snapshot given the start time using the
/// globally configured profiler
pub fn end_snapshot(start: EPInstant, name: &'static str) -> EPSnapshot {
    unsafe { PROFILER }.end_snapshot(start, name)
}

/// Logs the given snapshot with the globally configured profiler
pub fn log_snapshot(snapshot: &EPSnapshot) {
    unsafe { PROFILER }.log_snapshot(snapshot);
}

/// Profiles the given closure `target` with name `name`.
///
/// ```
/// embedded_profiling::profile("profile println", || {
///     println!("profiling this closure");
/// });
/// ```
pub fn profile<T, R>(name: &'static str, target: T) -> R
where
    T: Fn() -> R,
{
    let start = start_snapshot();
    let ret = target();
    let snapshot = end_snapshot(start, name);

    log_snapshot(&snapshot);
    ret
}

#[cfg(test)]
mod test {
    use super::mock::StdMockProfiler;
    use super::*;

    #[cfg(feature = "proc-macros")]
    use crate as embedded_profiling;

    use std::sync::Once;

    static INIT_PROFILER: Once = Once::new();
    static mut MOCK_PROFILER: Option<StdMockProfiler> = None;

    fn set_profiler() {
        INIT_PROFILER.call_once(|| unsafe {
            if MOCK_PROFILER.is_none() {
                MOCK_PROFILER = Some(StdMockProfiler::default());
            }
            super::set_profiler(MOCK_PROFILER.as_ref().unwrap()).unwrap();
        });
    }

    #[test]
    fn basic_duration() {
        let profiler = StdMockProfiler::default();

        let start = profiler.start_snapshot();
        std::thread::sleep(std::time::Duration::from_millis(25));
        let end = profiler.end_snapshot(start, "basic_dur");
        profiler.log_snapshot(&end);
    }

    #[test]
    fn basic_duration_and_set_profiler() {
        // set the profiler, if it hasn't been already
        set_profiler();

        let start = start_snapshot();
        std::thread::sleep(std::time::Duration::from_millis(25));
        let end = end_snapshot(start, "basic_dur");
        log_snapshot(&end);
    }

    #[test]
    fn profile_closure() {
        // set the profiler, if it hasn't been already
        set_profiler();

        profile("25ms closure", || {
            std::thread::sleep(std::time::Duration::from_millis(25));
        });
    }

    #[cfg(feature = "proc-macros")]
    #[test]
    fn profile_proc_macro() {
        #[profile_function]
        fn delay_25ms() {
            std::thread::sleep(std::time::Duration::from_millis(25));
        }

        // set the profiler, if it hasn't been already
        set_profiler();

        delay_25ms();
    }
}
