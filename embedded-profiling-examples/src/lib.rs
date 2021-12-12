#![no_std]
pub use feather_m4::{self as bsp, hal, pac};

pub mod prelude;
#[cfg(feature = "usb")]
pub mod usb_serial;
#[cfg(feature = "usb")]
pub mod usb_serial_log;
