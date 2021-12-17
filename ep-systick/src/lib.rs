//! [`EmbeddedProfiler`] implementation based on [`systick`](cortex_m::peripheral::SYST).
//!
//! This profiler depends on the [`SYST`] hardware common to most cortex-M devices.
//! The profiler's configured resolution is the same as the core clock. The cycle count clock is
//! free-running, so overflows are likely if you have long running functions to profile.
//! To mitigate this, one can use the `extended` feature, which extends the resolution of
//! the counter from [`u32`] to [`u64`] using the [`SysTick`] exception. It is set
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
//! [`SYST`]: cortex_m::peripheral::SYST
//! [`SysTick`]: `cortex_m::peripheral::scb::Exception::SysTick`

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

    /// binary GCD function stolen from wikipedia, made const
    const fn gcd(mut u: EPContainer, mut v: EPContainer) -> EPContainer {
        // Base cases: gcd(n, 0) = gcd(0, n) = n
        if u == 0 {
            return v;
        } else if v == 0 {
            return u;
        }

        // Using identities 2 and 3:
        // gcd(2ⁱ u, 2ʲ v) = 2ᵏ gcd(u, v) with u, v odd and k = min(i, j)
        // 2ᵏ is the greatest power of two that divides both u and v
        let i = u.trailing_zeros();
        u >>= i;
        let j = v.trailing_zeros();
        v >>= j;

        // min(i, j);
        let k = if i <= j { i } else { j };

        loop {
            // u and v are odd at the start of the loop
            // debug_assert!(u % 2 == 1, "u = {} is even", u);
            // debug_assert!(v % 2 == 1, "v = {} is even", v);

            // Swap if necessary so u <= v
            if u > v {
                // swap(&mut u, &mut v);
                let tmp = u;
                u = v;
                v = tmp;
            }

            // Using identity 4 (gcd(u, v) = gcd(|v-u|, min(u, v))
            v -= u;

            // Identity 1: gcd(u, 0) = u
            // The shift by k is necessary to add back the 2ᵏ factor that was removed before the loop
            if v == 0 {
                return u << k;
            }

            // Identity 3: gcd(u, 2ʲ v) = gcd(u, v) (u is known to be odd)
            v >>= v.trailing_zeros();
        }
    }

    /// Reduce the fraction we need to convert between 1µs precision and whatever our core clock is running at
    pub(crate) const fn reduced_fraction() -> (EPContainer, EPContainer) {
        let gcd = Self::gcd(1_000_000, FREQ as EPContainer) as EPContainer;
        (1_000_000 / gcd, FREQ as EPContainer / gcd)
    }
}

impl<const FREQ: u32> EmbeddedProfiler for SysTickProfiler<FREQ> {
    fn read_clock(&self) -> EPInstant {
        let raw_ticks = SYST::get_current();
        #[allow(unused_mut)]
        let mut count = (SYSTICK_RELOAD - raw_ticks) as EPContainer;

        #[cfg(feature = "extended")]
        {
            /// the resolution of [`systick`](cortex_m::peripheral::SYST), 2**24
            const SYSTICK_RESOLUTION: EPContainer = 16777216;

            // add on the number of times we've rolled over (systick resolution is 2**24)
            let rollover_count =
                ROLLOVER_COUNT.load(core::sync::atomic::Ordering::Acquire) as EPContainer;
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
