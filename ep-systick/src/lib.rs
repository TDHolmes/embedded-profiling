//! [`EmbeddedProfiler`] implementation based on [`systick`](cortex_m::peripheral::SYST).
//!
//! This profiler depends on the [`SYST`] hardware common to most cortex-M devices.
//! The profiler's configured resolution is the same as the core clock. The cycle count clock is
//! free-running, so overflows are likely if you have long running functions to profile.
//! To mitigate this, one can use the `extended` feature, which extends the resolution of
//! the counter from 24 bit to [`u32`] or [`u64`] using the [`SysTick`] exception. It is set
//! to expire just before overflow, so you can expect an exception to fire every 2**24
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
//! let dwt_profiler = cortex_m::singleton!(: ep_systick::SysTickProfiler::<CORE_FREQ> =
//!     ep_systick::SysTickProfiler::<CORE_FREQ>::new(core.SYST, CORE_FREQ))
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
//! as discussed above, extend the native resolution of 24 bits to either 32 or 64 bits
//! using the [`SysTick`] exception. The exception fires ever 2**24 clock cycles.
//!
//! ### `container-u64`
//!
//! enables the `container-u64` feature in [`embedded-profiling`](embedded_profiling). Use
//! a [`u64`] as the time storage type instead of [`u32`] for longer running profiling.
//!
//! ### `proc-macros`
//!
//! enables the `proc-macros` feature in [`embedded-profiling`](embedded_profiling). Enables
//! the [`embedded_profiling::profile_function`] procedural macro.
//!
//! [`SYST`]: cortex_m::peripheral::SYST
//! [`SysTick`]: `cortex_m::peripheral::scb::Exception::SysTick`
//! [`embedded_profiling::profile_function`]: https://docs.rs/embedded-profiling/latest/embedded_profiling/attr.profile_function.html
#![cfg_attr(not(test), no_std)]

use cortex_m::peripheral::{syst::SystClkSource, SYST};
use embedded_profiling::{EPContainer, EPInstant, EPSnapshot, EmbeddedProfiler};

#[cfg(feature = "extended")]
/// Tracker of `systick` cycle count overflows to extend systick's 24 bit timer
static ROLLOVER_COUNT: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

/// The reload value of the [`systick`](cortex_m::peripheral::SYST) peripheral. Also is the max it can go (2**24).
const SYSTICK_RELOAD: u32 = 0x00FF_FFFF;

/// [`systick`](cortex_m::peripheral::SYST) implementation of [`EmbeddedProfiler`].
///
/// The frequency of the [`systick`](cortex_m::peripheral::SYST) is encoded using the parameter `FREQ`.
pub struct SysTickProfiler<const FREQ: u32> {
    #[allow(unused)]
    // we currently take SYST by value only to ensure ownership
    systick: SYST,
}

impl<const FREQ: u32> SysTickProfiler<FREQ> {
    /// Enable the [`systick`](cortex_m::peripheral::SYST) and provide a new [`EmbeddedProfiler`].
    ///
    /// Note that the `sysclk` parameter should come from e.g. the HAL's clock generation function
    /// so the real speed and the declared speed can be compared.
    ///
    /// # Panics
    /// asserts that the compile time constant `FREQ` matches the runtime provided `sysclk`
    pub fn new(mut systick: SYST, sysclk: u32) -> Self {
        assert!(FREQ == sysclk);

        systick.disable_counter();
        systick.set_clock_source(SystClkSource::Core);
        systick.clear_current();
        systick.set_reload(SYSTICK_RELOAD);
        systick.enable_counter();

        #[cfg(feature = "extended")]
        systick.enable_interrupt();

        Self { systick }
    }

    /// Reduce the fraction we need to convert between 1µs precision and whatever our core clock is running at
    pub(crate) const fn reduced_fraction() -> (EPContainer, EPContainer) {
        let gcd = gcd::binary_u64(1_000_000_u64, FREQ as u64) as EPContainer;
        (1_000_000 / gcd, FREQ as EPContainer / gcd)
    }
}

impl<const FREQ: u32> EmbeddedProfiler for SysTickProfiler<FREQ> {
    fn read_clock(&self) -> EPInstant {
        let raw_ticks = SYST::get_current();
        #[allow(unused_mut)]
        let mut count = EPContainer::from(SYSTICK_RELOAD - raw_ticks);

        #[cfg(feature = "extended")]
        {
            /// the resolution of [`systick`](cortex_m::peripheral::SYST), 2**24
            const SYSTICK_RESOLUTION: EPContainer = 16_777_216;

            // add on the number of times we've rolled over (systick resolution is 2**24)
            let rollover_count =
                EPContainer::from(ROLLOVER_COUNT.load(core::sync::atomic::Ordering::Acquire));
            count += rollover_count * SYSTICK_RESOLUTION;
        }

        let (red_num, red_denom) = Self::reduced_fraction();
        EPInstant::from_ticks(count * red_num / red_denom)
    }

    fn log_snapshot(&self, snapshot: &EPSnapshot) {
        log::info!("{}", snapshot);
    }
}

#[cfg(feature = "extended")]
use cortex_m_rt::exception;

#[cfg(feature = "extended")]
#[exception]
#[allow(non_snake_case)]
fn SysTick() {
    ROLLOVER_COUNT.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn check_reduced_fraction() {
        const FREQ: u32 = 120_000_000;
        let (num, den) = SysTickProfiler::<FREQ>::reduced_fraction();
        assert_eq!(1, num);
        assert_eq!(120, den);
    }
}
