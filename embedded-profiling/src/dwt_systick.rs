//! # `Monotonic` implementation based on `DWT` and `SysTick`

use cortex_m::peripheral::{syst::SystClkSource, DCB, DWT, SYST};
use log;

/// DWT and Systick combination implementing `embedded_time::Clock` and `rtic_monotonic::Monotonic`
///
/// The frequency of the `DWT` and `SysTick` is encoded using the parameter `FREQ`.
pub struct DwtSystick<const FREQ: u32> {
    dwt: DWT,
    systick: SYST,
}

impl<const FREQ: u32> DwtSystick<FREQ> {
    /// Enable the DWT and provide a new `Monotonic` based on `DWT` and `SysTick`.
    ///
    /// Note that the `sysclk` parameter should come from e.g. the HAL's clock generation function
    /// so the real speed and the declared speed can be compared.
    ///
    /// # Panics
    /// asserts that the compile time constant `FREQ` matches the runtime provided `sysclk`
    pub fn new(dcb: &mut DCB, dwt: DWT, systick: SYST, sysclk: u32) -> Self {
        assert!(FREQ == sysclk);

        dcb.enable_trace();
        DWT::unlock();

        unsafe { dwt.cyccnt.write(0) };

        let mut timer = Self { dwt, systick };

        timer.dwt.enable_cycle_counter();

        timer.systick.set_clock_source(SystClkSource::Core);
        timer.systick.enable_counter();

        timer
    }

    /// binary GCD function stolen from wikipedia, made const
    const fn gcd(mut u: u32, mut v: u32) -> u32 {
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
    pub(crate) const fn reduced_fraction() -> (u32, u32) {
        let gcd = Self::gcd(1_000_000, FREQ);
        (1_000_000 / gcd, FREQ / gcd)
    }
}

impl<const FREQ: u32> crate::EmbeddedProfiler for DwtSystick<FREQ> {
    fn read_clock(&self) -> crate::EPInstant {
        let (red_num, red_denom) = Self::reduced_fraction();
        crate::EPInstant::from_ticks(
            (self.dwt.cyccnt.read() * red_num / red_denom) as crate::EPContainer,
        )
    }

    fn reset_clock(&mut self) {
        unsafe {
            self.dwt.cyccnt.write(0);
        }
        self.systick.clear_current();
    }

    fn log_snapshot(&self, snapshot: &crate::EPSnapshot) {
        log::info!("{}", snapshot);
    }
}
