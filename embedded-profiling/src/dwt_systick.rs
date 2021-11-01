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
}

impl<const FREQ: u32> crate::EmbeddedProfiler for DwtSystick<FREQ> {
    fn read_clock(&self) -> crate::EPInstant {
        // TODO: fix this gross conversion
        crate::EPInstant::from_ticks(
            (self.dwt.cyccnt.read() as u64 * 1_000_000_u64 / FREQ as u64) as u32,
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
