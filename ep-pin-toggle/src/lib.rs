//! An [`EmbeddedProfiler`] implementation that only toggles the given pin
#![cfg_attr(not(test), no_std)]

use core::cell::RefCell;
use embedded_hal::digital::v2::OutputPin;
use embedded_profiling::{EPInstant, EmbeddedProfiler};

/// Implements [`EmbeddedProfiler`] by just toggling the pin
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
