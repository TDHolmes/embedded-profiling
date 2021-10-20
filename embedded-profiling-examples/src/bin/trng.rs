#![no_std]
#![no_main]

use embedded_profiling_examples as epe;
use epe::{bsp, hal, usb_serial, usb_serial_log};

use hal::clock::GenericClockController;
use hal::pac::{CorePeripherals, Peripherals};
use hal::prelude::*;
use hal::trng::Trng;

use panic_halt as _;

#[bsp::entry]
fn main() -> ! {
    let mut peripherals = Peripherals::take().unwrap();
    let mut core = CorePeripherals::take().unwrap();
    let mut clocks = GenericClockController::with_external_32kosc(
        peripherals.GCLK,
        &mut peripherals.MCLK,
        &mut peripherals.OSC32KCTRL,
        &mut peripherals.OSCCTRL,
        &mut peripherals.NVMCTRL,
    );
    let pins = bsp::Pins::new(peripherals.PORT);

    // initialize USB stuff
    let bus_allocator = bsp::usb_allocator(
        pins.usb_dm,
        pins.usb_dp,
        peripherals.USB,
        &mut clocks,
        &mut peripherals.MCLK,
    );
    usb_serial::init(&mut core.NVIC, bus_allocator);
    usb_serial_log::init().ok();

    // initialize our profiling timer & structure

    let mut red_led: bsp::RedLed = pins.d13.into();

    let mut delay = hal::delay::Delay::new(core.SYST, &mut clocks);

    // Create a struct as a representation of the random number generator peripheral
    let trng = Trng::new(&mut peripherals.MCLK, peripherals.TRNG);

    // Simple loop that blinks the red led with random on and off times that are
    // sourced from the random number generator.
    loop {
        red_led.toggle().unwrap();
        delay.delay_ms(trng.random_u8());
    }
}
