//! Manager of all USB serial communication
use crate::hal::usb::UsbBus;
use crate::pac;
use pac::interrupt;

use core::sync::atomic;

use cortex_m::peripheral::NVIC;
use usb_device::{class_prelude::UsbBusAllocator, prelude::*};
use usbd_serial::{SerialPort, USB_CLASS_CDC};

/// Our global singleton for USB serial communication
pub struct UsbSerial<'a> {
    usb_serial: SerialPort<'a, UsbBus>,
    usb_dev: UsbDevice<'a, UsbBus>,
}

const USB_INTERRUPTS: [interrupt; 3] = [
    pac::interrupt::USB_OTHER,
    pac::interrupt::USB_TRCPT0,
    pac::interrupt::USB_TRCPT1,
];

/// static global for `USB_SERIAL` to use under the hood. Needs to be a static as far as I can tell.
/// not directly used by our code.
static mut USB_ALLOCATOR: Option<UsbBusAllocator<UsbBus>> = None;
/// Our global singleton for USB serial communication. Accessed via `usb_serial::get`
static mut USB_SERIAL: Option<UsbSerial> = None;
/// Once we see our first user interaction, we set this to true
static USER_PRESENT: atomic::AtomicBool = atomic::AtomicBool::new(false);

impl<'a> UsbSerial<'a> {
    /// Initializes everything we need for USB serial communication
    fn init(nvic: &mut NVIC, usb_allocator: UsbBusAllocator<UsbBus>) {
        let usb_allocator = unsafe {
            USB_ALLOCATOR = Some(usb_allocator);
            USB_ALLOCATOR.as_ref().unwrap()
        };
        let usb_serial = SerialPort::new(usb_allocator);
        let usb_dev = UsbDeviceBuilder::new(usb_allocator, UsbVidPid(0x16c0, 0x27dd))
            .manufacturer("embedded-profiling")
            .product("examples")
            .serial_number("EPE")
            .device_class(USB_CLASS_CDC)
            .build();

        // Safety:
        // initializes a global static which is only accessed in interrupt handler `USB`, which isn't enabled
        // until after this is completed.
        unsafe {
            USB_SERIAL = Some(UsbSerial {
                usb_serial,
                usb_dev,
            });
            for interrupt in USB_INTERRUPTS {
                nvic.set_priority(interrupt, 1);
                NVIC::unmask(interrupt);
            }
        }
    }

    /// Polls the USB device and reads out any available serial data.
    ///
    /// To be called in the `USB` interrupt handler.
    pub fn poll(&mut self, read_buffer: &mut [u8]) -> usize {
        let mut total_bytes_read = 0;
        self.usb_dev.poll(&mut [&mut self.usb_serial]);

        if let Ok(bytes_read) = self.usb_serial.read(read_buffer) {
            total_bytes_read = bytes_read;
        }

        total_bytes_read
    }

    /// Writes bytes to USB serial
    ///
    /// # Arguments
    /// * bytes: raw bytes to write
    ///
    /// # Returns
    /// Number of bytes successfully written
    pub fn write(&mut self, bytes: &[u8]) -> usize {
        self.usb_serial.write(bytes).unwrap_or(0)
    }

    /// Writes a message over USB serial
    ///
    /// # Arguments
    /// * message: The message to write to the USB port
    ///
    /// # Returns
    /// number of bytes successfully written
    pub fn write_str(&mut self, message: &str) -> usize {
        let message_bytes = message.as_bytes();
        self.usb_serial.write(message_bytes).unwrap_or(0)
    }
}

/// Initializes our global singleton
pub fn init(nvic: &mut NVIC, usb_allocator: UsbBusAllocator<UsbBus>) {
    UsbSerial::init(nvic, usb_allocator);
}

/// Checks if a user is present at the serial port by checking if we've received any
/// bytes since boot
pub fn user_present() -> bool {
    USER_PRESENT.load(atomic::Ordering::Acquire)
}

/// Borrows the global singleton `UsbSerial` for a brief period with interrupts disabled
///
/// # Arguments
/// `borrower`: The closure that gets run borrowing the global `UsbSerial`
///
/// # Safety
/// the global singleton `UsbSerial` can be safely borrowed because we disable
/// interrupts while it is being borrowed, guaranteeing that interrupt handlers like
/// `USB` cannot mutate `UsbSerial` while we are as well.
///
/// # Panic
/// If `init` has not been called and we haven't initialized our global singleton `UsbSerial`,
/// we will panic.
pub fn get<T, R>(borrower: T) -> R
where
    T: Fn(&mut UsbSerial) -> R,
{
    usb_free(|_| unsafe {
        let usb_serial = USB_SERIAL.as_mut().expect("UsbSerial not initialized");
        borrower(usb_serial)
    })
}

/// Execute closure `f` in an interrupt-free context.
///
/// This as also known as a "critical section".
#[inline]
fn usb_free<F, R>(f: F) -> R
where
    F: FnOnce(&cortex_m::interrupt::CriticalSection) -> R,
{
    for interrupt in USB_INTERRUPTS {
        NVIC::mask(interrupt);
    }

    let r = f(unsafe { &cortex_m::interrupt::CriticalSection::new() });

    for interrupt in USB_INTERRUPTS {
        unsafe {
            NVIC::unmask(interrupt);
        }
    }

    r
}

/// Writes the given message out over USB serial.
///
/// # Arguments
/// * println args: variable arguments passed along to `ufmt::uwrite!`
///
/// # Warning
/// as this function deals with a static mut, and it is also accessed in the
/// USB interrupt handler, we both have unsafe code for unwrapping a static mut
/// as well as disabling of interrupts while we do so.
///
/// # Safety
/// the only time the static mut is used, we have interrupts disabled so we know
/// we have sole access
#[macro_export]
macro_rules! serial_write {
    ($($tt:tt)+) => {{
        use core::fmt::Write;

        let mut s: heapless::String<64> = heapless::String::new();
        core::write!(&mut s, $($tt)*).unwrap();
        crate::usb_serial::get(|usbserial| { usbserial.write_str(s.as_str()); });
    }};
}

fn poll_usb() {
    let mut buf = [0u8; 64];
    // Safety:
    // `USB_SERIAL`:
    // Only interrupt handler that accesses it. thread access is only done
    // while interrupts are disabled.
    //
    // `CLI_INPUT_PRODUCER`:
    // This is the only spot that we mutate it. When we initialize it to `Some()`,
    // interrupts are disabled so this handler cannot run.
    unsafe {
        if let Some(serial) = USB_SERIAL.as_mut() {
            let bytes_read = serial.poll(&mut buf);
            // serial.write(&buf[0..bytes_read]);
            if bytes_read != 0 {
                USER_PRESENT.store(true, atomic::Ordering::Release);
            }
        }
    }
}

#[interrupt]
#[allow(non_snake_case)]
fn USB_OTHER() {
    poll_usb();
}

#[interrupt]
#[allow(non_snake_case)]
fn USB_TRCPT0() {
    poll_usb();
}

#[interrupt]
#[allow(non_snake_case)]
fn USB_TRCPT1() {
    poll_usb();
}
