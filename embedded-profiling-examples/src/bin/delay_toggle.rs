#![no_std]
#![no_main]

use embedded_profiling_examples as epe;
use ep_pin_toggle::EPPinToggle;
use epe::{bsp, hal};

use hal::clock::GenericClockController;
use hal::pac::{CorePeripherals, Peripherals};
use hal::prelude::*;

use embedded_profiling as ep;
#[cfg(feature = "panic_halt")]
use panic_halt as _;

type EPPinToggleRedLed = EPPinToggle<core::convert::Infallible, bsp::RedLed>;

#[bsp::entry]
fn main() -> ! {
    let mut peripherals = Peripherals::take().unwrap();
    let core = CorePeripherals::take().unwrap();

    #[allow(unused)]
    let mut clocks = GenericClockController::with_external_32kosc(
        peripherals.GCLK,
        &mut peripherals.MCLK,
        &mut peripherals.OSC32KCTRL,
        &mut peripherals.OSCCTRL,
        &mut peripherals.NVMCTRL,
    );
    let mut delay = hal::delay::Delay::new(core.SYST, &mut clocks);

    let pins = bsp::Pins::new(peripherals.PORT);
    let mut red_led: bsp::RedLed = pins.d13.into();
    red_led.set_low().ok();

    #[cfg(feature = "panic_persist")]
    if let Some(_) = panic_persist::get_panic_message_bytes() {
        // blink SOS
        for delay_ms in [
            100_u32, 100_u32, 100_u32, 500_u32, 500_u32, 500_u32, 100_u32, 100_u32, 100_u32,
        ] {
            red_led.set_high().ok();
            delay.delay_ms(delay_ms);
            red_led.set_low().ok();
            delay.delay_ms(delay_ms);
        }
    }

    // initialize our profiling timer & structure
    let ep_pin_toggle: &'static EPPinToggleRedLed =
        cortex_m::singleton!(: EPPinToggleRedLed = EPPinToggle::new(red_led)).unwrap();
    unsafe {
        ep::set_profiler(ep_pin_toggle).unwrap();
    }

    // Loop and profile
    loop {
        profile_target(&mut delay);
    }
}

#[ep::profile_function]
#[inline(never)]
fn profile_target<D>(delay: &mut D)
where
    D: hal::ehal::blocking::delay::DelayUs<u32>,
{
    delay.delay_us(1234_u32);
}
