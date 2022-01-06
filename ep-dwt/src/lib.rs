//! [`EmbeddedProfiler`] implementation based on [`DWT`].
//!
//! This profiler depends on the [`DWT`] hardware which is not available on cortex-M0.
//! The profiler's resolution is the same as the core clock. The cycle count clock is
//! free-running, so overflows are likely if you have long running functions to profile.
//! To mitigate this, one can use the `extended` feature, which extends the resolution of
//! the counter from [`u32`] to [`u64`] using the [`DebugMonitor`] exception. It is set
//! to expire just before overflow, so you can expect an exception to fire every 2**32
//! clock cycles.
//!
//! Snapshots are logged using [`log::info!`], so having a logger installed is required
//! if you want to use [`embedded_profiling::log_snapshot`] or functions that call it
//! (like [`embedded_profiling::profile_function`]).
//!
//! ## Example Usage
//!
//!```no_run
//! # use cortex_m::peripheral::Peripherals as CorePeripherals;
//! # const CORE_FREQ: u32 = 120_000_000;
//! let mut core = CorePeripherals::take().unwrap();
//! // (...)
//! let dwt_profiler = cortex_m::singleton!(: ep_dwt::DwtProfiler::<CORE_FREQ> =
//!     ep_dwt::DwtProfiler::<CORE_FREQ>::new(&mut core.DCB, core.DWT, CORE_FREQ))
//! .unwrap();
//! unsafe {
//!     embedded_profiling::set_profiler(dwt_profiler).unwrap();
//! }
//! // (...)
//! embedded_profiling::profile("print_profile", || println!("Hello, world"));
//! ```
//!
//! ## Features
//!
//! ### `extended`
//!
//! Extends the [`DWT`] cycle counter's native resolution from 32 bit to 64 bit using
//! the cycle compare functionality and the [`DebugMonitor`] exception. The exception will
//! fire every 2**32 clock cycles. Enables the [`embedded-profiling`](embedded_profiling)
//! feature `container-u64`.
//!
//! ### `proc-macros`
//!
//! enables the `proc-macros` feature in [`embedded-profiling`](embedded_profiling). Enables
//! the [`embedded_profiling::profile_function`] procedural macro.
//!
//! [`DWT`]: cortex_m::peripheral::DWT
//! [`DebugMonitor`]: `cortex_m::peripheral::scb::Exception::DebugMonitor`
//! [`embedded_profiling::profile_function`]: https://docs.rs/embedded-profiling/latest/embedded_profiling/attr.profile_function.html
#![cfg_attr(not(test), no_std)]

use embedded_profiling::{EPContainer, EPInstant, EPInstantGeneric, EPSnapshot, EmbeddedProfiler};

use cortex_m::peripheral::{DCB, DWT};

#[cfg(feature = "extended")]
use core::sync::atomic::{AtomicU32, Ordering};
#[cfg(feature = "extended")]
use cortex_m_rt::exception;

#[cfg(feature = "extended")]
/// Tracker of `cyccnt` cycle count overflows to extend this timer to 64 bit
static ROLLOVER_COUNT: AtomicU32 = AtomicU32::new(0);

#[cfg(feature = "extended")]
// For extended mode to work, we really need a u64 container. Double check this.
static_assertions::assert_type_eq_all!(EPContainer, u64);

/// DWT trace unit implementing [`EmbeddedProfiler`].
///
/// The frequency of the [`DWT`] is encoded using the parameter `FREQ`.
pub struct DwtProfiler<const FREQ: u32> {
    dwt: DWT,
}

impl<const FREQ: u32> DwtProfiler<FREQ> {
    /// Enable the [`DWT`] and provide a new [`EmbeddedProfiler`].
    ///
    /// Note that the `sysclk` parameter should come from e.g. the HAL's clock generation function
    /// so the real speed and the declared speed can be compared.
    ///
    /// # Panics
    /// asserts that the compile time constant `FREQ` matches the runtime provided `sysclk`
    #[must_use]
    pub fn new(dcb: &mut DCB, mut dwt: DWT, sysclk: u32) -> Self {
        assert!(FREQ == sysclk);

        // Enable the DWT block
        dcb.enable_trace();
        #[cfg(feature = "extended")]
        // Enable DebugMonitor exceptions to fire to track overflows
        unsafe {
            dcb.demcr.modify(|f| f | 1 << 16);
        }
        DWT::unlock();

        // reset cycle count and enable it to run
        unsafe { dwt.cyccnt.write(0) };
        dwt.enable_cycle_counter();

        Self { dwt }
    }
}

impl<const FREQ: u32> EmbeddedProfiler for DwtProfiler<FREQ> {
    fn read_clock(&self) -> EPInstant {
        // get the cycle count and add the rollover if we're extended
        #[allow(unused_mut)]
        let mut count = EPContainer::from(self.dwt.cyccnt.read());
        #[cfg(feature = "extended")]
        {
            count += EPContainer::from(ROLLOVER_COUNT.load(Ordering::Relaxed))
                * EPContainer::from(u32::MAX);
        }

        // convert count and return the instant
        embedded_profiling::convert_instant(EPInstantGeneric::<1, FREQ>::from_ticks(count))
    }

    fn log_snapshot(&self, snapshot: &EPSnapshot) {
        log::info!("{}", snapshot);
    }
}

#[cfg(feature = "extended")]
#[exception]
#[allow(non_snake_case)]
fn DebugMonitor() {
    ROLLOVER_COUNT.fetch_add(1, Ordering::Relaxed);
}
