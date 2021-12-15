//! [`EmbeddedProfiler`] implementation based on [`DWT`].
#![cfg_attr(not(test), no_std)]
use embedded_profiling::{EPContainer, EPInstant, EPSnapshot, EmbeddedProfiler};

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

impl<const FREQ: u32> EmbeddedProfiler for DwtProfiler<FREQ> {
    fn read_clock(&self) -> EPInstant {
        // get the cycle count and add the rollover if we're extended
        #[allow(unused_mut)]
        let mut count = self.dwt.cyccnt.read() as EPContainer;
        #[cfg(feature = "extended")]
        {
            count +=
                ROLLOVER_COUNT.load(Ordering::Relaxed) as EPContainer * u32::MAX as EPContainer;
        }

        // convert count and return the instant
        let (red_num, red_denom) = Self::reduced_fraction();
        EPInstant::from_ticks(count * red_num / red_denom)
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
