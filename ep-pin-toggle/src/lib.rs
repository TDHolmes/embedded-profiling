//! An [`EmbeddedProfiler`] implementation that toggles the given pin.
//!
//! This profiler is geared towards systems that have very limited resources and
//! just want to profile a function via an oscilloscope or logic analyzer. The
//! analyzer takes any GPIO that implements the
//! [`OutputPin`](embedded_hal::digital::v2::OutputPin) trait
//!
//! ## Example Usage
//!
//!```no_run
//! # struct MyPin;
//! # type MyPinError = ();
//! # impl embedded_hal::digital::v2::OutputPin for MyPin { type Error = ();
//! # fn set_low(&mut self) -> Result<(), Self::Error> { Ok(()) }
//! # fn set_high(&mut self) -> Result<(), Self::Error> { Ok(()) } }
//! # let pin = MyPin;
//! let ep_pin_toggle = cortex_m::singleton!(: ep_pin_toggle::EPPinToggle<MyPinError, MyPin> =
//!     ep_pin_toggle::EPPinToggle::new(pin)).unwrap();
//! unsafe {
//!     embedded_profiling::set_profiler(ep_pin_toggle).unwrap();
//! }
//! // (...)
//! embedded_profiling::profile("print_profile", || println!("Hello, world"));
//! ```
//!
//! ## Features
//!
//! ### `proc-macros`
//!
//! enables the `proc-macros` feature in [`embedded-profiling`](embedded_profiling). Enables
//! the [`macro@embedded_profiling::profile_function`] procedural macro.
#![cfg_attr(not(test), no_std)]

use core::cell::RefCell;
use embedded_hal::digital::v2::OutputPin;
use embedded_profiling::{EPInstant, EmbeddedProfiler};

/// Implements [`EmbeddedProfiler`] by toggling the given pin.
pub struct EPPinToggle<E, P>
where
    P: OutputPin<Error = E>,
{
    pin: RefCell<P>,
}

impl<E, P> EPPinToggle<E, P>
where
    P: OutputPin<Error = E>,
{
    /// Creates a new [`EPPinToggle`] with the given `pin`.
    pub fn new(pin: P) -> Self {
        Self {
            pin: RefCell::new(pin),
        }
    }

    /// Consumes [`EPPinToggle`], returning the `pin`.
    pub fn free(self) -> P {
        self.pin.into_inner()
    }
}

impl<E, P> EmbeddedProfiler for EPPinToggle<E, P>
where
    P: OutputPin<Error = E>,
{
    fn read_clock(&self) -> EPInstant {
        EPInstant::from_ticks(0)
    }

    fn at_start(&self) {
        self.pin.borrow_mut().set_high().ok();
    }

    fn at_end(&self) {
        self.pin.borrow_mut().set_low().ok();
    }
}
