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
//! if let Some(snapshot) = embedded_profiling::end_snapshot(start, "name-of-computation") {
//!     // Optionally, log it if we didn't overflow
//!     embedded_profiling::log_snapshot(&snapshot);
//! }
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

#[cfg(test)]
mod mock;
#[cfg(feature = "proc-macros")]
pub use embedded_profiling_proc_macros::profile_function;

pub use fugit;

// do the feature gating on a private type so our public documentation is only in one place
#[cfg(not(feature = "container-u64"))]
type PrivContainer = u32;
#[cfg(feature = "container-u64")]
type PrivContainer = u64;

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

    /// Optionally log the snapshot to some output, like a serial port
    fn log_snapshot(&self, _snapshot: &EPSnapshot) {}

    /// Optional function that gets called at the start of the snapshot recording.
    /// If one would want to very simple profiling, they could use `at_start` and `at_end`
    /// to simply toggle a GPIO.
    fn at_start(&self) {}

    /// Optional function that gets called at the end of the snapshot recording
    fn at_end(&self) {}

    /// takes the starting snapshot of a specific trace
    fn start_snapshot(&self) -> EPInstant {
        self.at_start();
        self.read_clock()
    }

    /// computes the duration of the snapshot given the start time, if there hasn't been overflow
    ///
    fn end_snapshot(&self, start: EPInstant, name: &'static str) -> Option<EPSnapshot> {
        self.at_end();
        let now = self.read_clock();
        now.checked_duration_since(start)
            .map(|duration| EPSnapshot { name, duration })
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
///
/// ```
/// # struct MyProfiler;
/// # impl embedded_profiling::EmbeddedProfiler for MyProfiler { fn read_clock(&self) -> embedded_profiling::EPInstant { embedded_profiling::EPInstant::from_ticks(0) } }
/// # static MY_PROFILER: MyProfiler = MyProfiler;
/// let noop_profiler_ref = embedded_profiling::profiler();  // no-op profiler returned because we haven't set one yet
/// // interrupts should be disabled while this is called with something like `cortex_m::interrupt::free`
/// unsafe {
///     embedded_profiling::set_profiler(&MY_PROFILER).unwrap();
/// }
/// let my_profiler_ref = embedded_profiling::profiler();  // our profiler now returned
/// ```
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

/// Returns a reference to the configured profiler
///
/// If a profiler hasn't yet been set by [`set_profiler`], the no-op profiler
/// will be returned. Generally, you should use one of the other provided
/// functions rather than getting a reference through this function.
///
/// ```
/// let start = embedded_profiling::profiler().start_snapshot();
/// // (...)
/// let snapshot = embedded_profiling::profiler().end_snapshot(start, "doc-example");
/// ```
#[inline]
pub fn profiler() -> &'static dyn EmbeddedProfiler {
    if STATE.load(Ordering::Acquire) == INITIALIZED {
        unsafe { PROFILER }
    } else {
        static NOP: NoopProfiler = NoopProfiler;
        &NOP
    }
}

/// takes the starting snapshot of a specific trace
///
/// ```
/// let start = embedded_profiling::start_snapshot();
/// // (...)
/// let snapshot = embedded_profiling::end_snapshot(start, "doc-example");
/// ```
#[inline]
pub fn start_snapshot() -> EPInstant {
    profiler().start_snapshot()
}

/// computes the duration of the snapshot given the start time using the
/// globally configured profiler
#[inline]
pub fn end_snapshot(start: EPInstant, name: &'static str) -> Option<EPSnapshot> {
    profiler().end_snapshot(start, name)
}

/// Logs the given snapshot with the globally configured profiler
///
/// ```
/// let start = embedded_profiling::start_snapshot();
/// // (...)
/// if let Some(snapshot) = embedded_profiling::end_snapshot(start, "doc-example") {
///     embedded_profiling::log_snapshot(&snapshot);
/// }
#[inline]
pub fn log_snapshot(snapshot: &EPSnapshot) {
    profiler().log_snapshot(snapshot);
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
    if let Some(snapshot) = end_snapshot(start, name) {
        log_snapshot(&snapshot);
    }
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
    #[serial_test::serial]
    fn basic_duration() {
        let profiler = StdMockProfiler::default();

        let start = profiler.start_snapshot();
        std::thread::sleep(std::time::Duration::from_millis(25));
        let end = profiler.end_snapshot(start, "basic_dur").unwrap();
        profiler.log_snapshot(&end);
    }

    #[test]
    #[serial_test::serial]
    fn basic_duration_and_set_profiler() {
        // set the profiler, if it hasn't been already
        set_profiler();

        let start = start_snapshot();
        std::thread::sleep(std::time::Duration::from_millis(25));
        let end = end_snapshot(start, "basic_dur").unwrap();
        log_snapshot(&end);
    }

    #[test]
    #[serial_test::serial]
    fn profile_closure() {
        // set the profiler, if it hasn't been already
        set_profiler();

        profile("25ms closure", || {
            std::thread::sleep(std::time::Duration::from_millis(25));
        });
    }

    #[cfg(feature = "proc-macros")]
    #[test]
    #[serial_test::serial]
    fn profile_proc_macro() {
        #[profile_function]
        fn delay_25ms() {
            std::thread::sleep(std::time::Duration::from_millis(25));
        }

        // set the profiler, if it hasn't been already
        set_profiler();

        delay_25ms();
    }

    #[cfg(feature = "proc-macros")]
    #[test]
    #[serial_test::serial]
    fn check_call_and_order() {
        use Ordering::SeqCst;

        #[profile_function]
        fn delay_25ms() {
            std::thread::sleep(std::time::Duration::from_millis(25));
        }

        // set the profiler, if it hasn't been already
        set_profiler();

        delay_25ms();

        // check if our functions were called and if the order is right
        let stats = unsafe { &MOCK_PROFILER.as_ref().unwrap().funcs_called };
        let at_start_was_called = stats.at_start.called.load(SeqCst);
        let read_clock_was_called = stats.read_clock.called.load(SeqCst);
        let at_end_was_called = stats.at_end.called.load(SeqCst);
        // stats.read_clock (but skipped since we've already called it)
        let log_snapshot_was_called = stats.log_snapshot.called.load(SeqCst);

        let at_start_at = stats.at_start.at.load(SeqCst);
        let read_clock_at = stats.read_clock.at.load(SeqCst);
        let at_end_at = stats.at_end.at.load(SeqCst);
        let log_snapshot_at = stats.log_snapshot.at.load(SeqCst);

        if at_start_was_called {
            println!("at_start called #{}", at_start_at);
        } else {
            println!("at_start not called");
        }
        if read_clock_was_called {
            println!("read_clock called #{}", read_clock_at);
        } else {
            println!("read_clock not called");
        }
        if at_end_was_called {
            println!("at_end called #{}", at_end_at);
        } else {
            println!("at_end not called");
        }
        if log_snapshot_was_called {
            println!("log_snapshot called #{}", log_snapshot_at);
        } else {
            println!("log_snapshot not called");
        }

        assert!(at_start_was_called, "'at_start' was never called");
        assert!(read_clock_was_called, "'read_clock' was never called");
        assert!(at_end_was_called, "'at_end' was never called");
        assert!(log_snapshot_was_called, "'log_snapshot' was never called");

        assert_eq!(at_start_at, 0, "'at_start' called at wrong time");
        assert_eq!(read_clock_at, 1, "'read_clock' called at wrong time");
        assert_eq!(at_end_at, 2, "'at_end' called at wrong time");
        assert_eq!(log_snapshot_at, 3, "'log_snapshot' called at wrong time");
    }
}
