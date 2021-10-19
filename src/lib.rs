//! Derp
#![warn(missing_docs)]
#![cfg_attr(not(test), no_std)]

#[cfg(test)]
mod mock;
mod prelude;

pub use embedded_time;

use prelude::*;

/// The `EmbeddedTrace` struct. :shrugs:
// pub struct EmbeddedTrace<C, W>
// where
//     C: Clock,
//     W: Write,
// {
//     clock: C,
//     writer: W,
// }

pub struct EmbeddedTraceDuration<T: embedded_time::TimeInt> {
    /// The name of this trace
    pub name: &'static str,
    /// The duration of this trace
    pub duration: embedded_time::duration::Nanoseconds<T>,
}

impl<T> core::fmt::Display for EmbeddedTraceDuration<T>
where
    T: embedded_time::TimeInt,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "<ETDur {}: {}ns>", self.name, self.duration)
    }
}

/// derp
pub trait EmbeddedTrace<C, W>
where
    C: Clock,
    W: Write,
{
    /// Gets the singleton instance of `EmbeddedTrace`.
    fn get() -> &'static Self;

    /// Mutably borrow a writer to write out the snapshot
    ///
    /// # Safety
    ///
    /// The implementer must safely guarantee that this writer can be used
    /// mutably. E.g., behind a mutex.
    fn borrow_writer<T, R>(borrower: T) -> R
    where
        T: Fn(&mut W) -> R;

    /// Takes a reading from the clock
    fn read_clock(&self) -> embedded_time::Instant<C>;

    /// takes the starting snapshot of a specific trace
    fn start_snapshot(&self) -> embedded_time::Instant<C> {
        self.read_clock()
    }

    /// computes the duration of the snapshot given the start time
    fn end_snapshot(
        &self,
        start_snapshot: embedded_time::Instant<C>,
        snapshot_name: &'static str,
    ) -> EmbeddedTraceDuration<C::T>
where {
        use core::convert::TryInto;

        let snap = self.read_clock();
        let duration = snap - start_snapshot;

        let micros: embedded_time::duration::Nanoseconds<C::T> = duration.try_into().unwrap();
        EmbeddedTraceDuration {
            name: snapshot_name,
            duration: micros,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn basic_snapshot() {
        let et = mock::ET::get();

        let start = et.start_snapshot();
        let sn = et.end_snapshot(start, "basic_snapshot");

        mock::ET::borrow_writer(|writer| write!(writer, "{}\n", sn).unwrap());
    }
}
