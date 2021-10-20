#![no_std]
#![no_main]

use embedded_profiling_examples as epe;
use epe::{bsp, hal, prelude::*, usb_serial, usb_serial_log};

use core::sync::atomic;
use hal::clock::GenericClockController;
use hal::pac::{interrupt, CorePeripherals, Peripherals, RTC};
use hal::prelude::*;
use hal::rtc;
use hal::sleeping_delay::SleepingDelay;

use cortex_m::peripheral::NVIC;
use dwt_systick_monotonic as dsm;
use embedded_profiling as ep;
use ep::embedded_time::Clock;
use panic_halt as _;

const CORE_FREQ: u32 = 120_000_000;

/// Shared atomic between RTC interrupt and sleeping_delay module
static INTERRUPT_FIRED: atomic::AtomicBool = atomic::AtomicBool::new(false);

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
    let mut clocks = GenericClockController::with_internal_32kosc(
        peripherals.GCLK,
        &mut peripherals.MCLK,
        &mut peripherals.OSC32KCTRL,
        &mut peripherals.OSCCTRL,
        &mut peripherals.NVMCTRL,
    );

    // Configure the RTC. a 1024 Hz clock is configured for us when enabling our
    // main clock
    let timer = rtc::Rtc::count32_mode(peripherals.RTC, 1024.hz(), &mut peripherals.MCLK);
    let mut sleeping_delay = SleepingDelay::new(timer, &INTERRUPT_FIRED);

    // enable interrupts
    unsafe {
        core.NVIC.set_priority(interrupt::RTC, 2);
        NVIC::unmask(interrupt::RTC);
    }

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

    while !usb_serial::user_present() {
        cortex_m::asm::wfi();
    }

    // initialize our profiling timer & structure
    let mut dwt_systic =
        dsm::DwtSystick::<CORE_FREQ>::new(&mut core.DCB, core.DWT, core.SYST, CORE_FREQ);
    unsafe {
        dwt_systic.reset();
    }
    unsafe {
        EP_SYSTICK_INSTANCE = Some(EPSystick { timer: dwt_systic });
    }

    // // Loop and profile our pi approximation math
    let et = EPSystick::get();
    loop {
        let start = et.start_snapshot();
        red_led.toggle().unwrap();
        sleeping_delay.delay_ms(250_u32);
        let sn = et.end_snapshot(start, "delay_250ms");

        EPSystick::borrow_writer(|writer| writeln!(writer, "{}", sn).unwrap());
    }
}

#[interrupt]
#[allow(non_snake_case)]
fn RTC() {
    // Let the sleepingtimer know that the interrupt fired, and clear it
    INTERRUPT_FIRED.store(true, atomic::Ordering::Release);
    unsafe {
        RTC::ptr()
            .as_ref()
            .unwrap()
            .mode0()
            .intflag
            .modify(|_, w| w.cmp0().set_bit());
    }
}
