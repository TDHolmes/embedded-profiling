//! implementation of `log`
use log::{Level, LevelFilter, Metadata, Record, SetLoggerError};

use core::sync::atomic;

use crate::{serial_write, usb_serial};

struct UsbSerialLogger {
    enabled_level: atomic::AtomicUsize,
}

static LOGGER: UsbSerialLogger = UsbSerialLogger::new();

impl UsbSerialLogger {
    const fn new() -> Self {
        Self {
            // 0 is not a valid log level, but can't seem to initialize here for some reason, so do it later in `init`
            enabled_level: atomic::AtomicUsize::new(0),
        }
    }

    fn set_level(&self, new_level: Level) {
        self.enabled_level
            .store(new_level as usize, atomic::Ordering::Release);
    }
}

impl log::Log for UsbSerialLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() as usize <= self.enabled_level.load(atomic::Ordering::Acquire)
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let level = record.level();
            if level == Level::Info {
                // leave INFO prefix off for expected normal output
                usb_serial::get(|_| serial_write!("{}\n", record.args()));
            } else {
                usb_serial::get(|_| serial_write!("{}: {}\n", record.level(), record.args()));
            }
        }
    }

    fn flush(&self) {}
}

/// Initializes the USB serial based logger
///
/// # Errors
/// Propagates through `Err(SetLoggerError)` if `log::set_logger_racy` returns this.
pub fn init() -> Result<(), SetLoggerError> {
    #[cfg(debug_assertions)]
    const LEVEL: Level = Level::Debug;
    #[cfg(not(debug_assertions))]
    const LEVEL: Level = Level::Info;

    #[cfg(debug_assertions)]
    let max_level = LevelFilter::Trace;
    #[cfg(not(debug_assertions))]
    let max_level = LevelFilter::Debug;

    LOGGER.set_level(LEVEL);
    cortex_m::interrupt::free(|_| unsafe {
        log::set_logger_racy(&LOGGER).map(|()| log::set_max_level(max_level))
    })
}
