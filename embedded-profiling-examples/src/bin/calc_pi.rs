#![no_std]
#![no_main]

use embedded_profiling_examples as epe;
use epe::{bsp, hal, usb_serial, usb_serial_log, prelude::*};

use core::f32;
use hal::clock::GenericClockController;
use hal::pac::{CorePeripherals, Peripherals};
use hal::prelude::*;

use dwt_systick_monotonic as dsm;
use embedded_profiling as ep;
use ep::embedded_time::Clock;
use panic_halt as _;

const CORE_FREQ: u32 = 120_000_000;

// some math functions...

fn factorial(num: f32) -> f32 {
    let mut result = 1_f32;
    for i in 2..(num as usize) {
        result *= i as f32;
    }
    result
}

fn powf(input: f32, mut exp: f32) -> f32 {
    if input == 0. || exp == 0. {
        return 1.;
    }

    let mut output = input;
    exp -= 1.;
    while exp > 0. {
        output *= input;
        exp -= 1.;
    }
    output
}

fn calculate_pi_ramanujan(iterations: usize) -> f32 {
    let mult: f32 = (2. * core::f32::consts::SQRT_2) / 9801.;
    let mut sum = 0.;

    // using a for loop requires const_for & const_mut_refs features
    //   and still didn't seem to work with a range
    let mut current_iteration = 0;
    while current_iteration != iterations {
        let k = current_iteration as f32;
        let numerator = factorial(4. * k) * (1103. + 26390. * k);
        let denominator = powf(factorial(k), 4.) * powf(396_f32, 4. * k);

        sum += numerator / denominator;
        current_iteration += 1;
    }

    1. / (sum * mult)
}

// our embedded-profiling struct & main

struct EPSystick {
    pub timer: dsm::DwtSystick<CORE_FREQ>,
}

static mut EP_SYSTICK_INSTANCE: Option<EPSystick> = None;

impl embedded_profiling::EmbeddedTrace for EPSystick {
    type ETClock = dsm::DwtSystick<CORE_FREQ>;
    type Writer = usb_serial::UsbSerial<'static>;

    fn get() -> &'static Self {
        // you must initialize!
        unsafe { EP_SYSTICK_INSTANCE.as_ref().unwrap() }
    }

    fn borrow_writer<T, R>(borrower: T) -> R
    where
        T: Fn(&mut Self::Writer) -> R,
    {
        usb_serial::get(|usbserial| borrower(usbserial))
    }

    fn read_clock(&self) -> embedded_profiling::embedded_time::Instant<Self::ETClock> {
        self.timer.try_now().unwrap()
    }
}

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
    let mut red_led: bsp::RedLed = pins.d13.into();

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
    let mut dwt_systic =
        dsm::DwtSystick::<CORE_FREQ>::new(&mut core.DCB, core.DWT, core.SYST, CORE_FREQ);
    unsafe {
        dwt_systic.reset();
    }
    unsafe {
        EP_SYSTICK_INSTANCE = Some(EPSystick { timer: dwt_systic });
    }

    // Simple loop that blinks the red led with random on and off times that are
    // sourced from the random number generator.
    loop {
        red_led.toggle().unwrap();
        calculate_pi_ramanujan(500);
    }
}
