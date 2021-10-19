//! Derp
#![warn(missing_docs)]
#![cfg_attr(not(test), no_std)]

pub use embedded_time;

// traits
use core::fmt::Write;
use embedded_time::clock::Clock;

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

    /// Derp
    fn get_writer() -> &'static mut W;

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
    use once_cell::sync::OnceCell;
    use std::fmt::Write as fmt_write;
    use std::io::Write;
    use std_embedded_time;

    // this whole Stdout rigamarole seems hacky but I dunno how to work around it
    struct Stdout {
        stdout: std::io::Stdout,
    }

    impl Stdout {
        pub fn new() -> Stdout {
            Stdout {
                stdout: std::io::stdout(),
            }
        }
    }
    unsafe impl Sync for Stdout {}

    impl core::fmt::Write for Stdout {
        fn write_str(&mut self, s: &str) -> core::fmt::Result {
            write!(self.stdout, "{}", s).unwrap();
            Ok(())
        }
    }

    struct ET {
        clock: std_embedded_time::StandardClock,
    }

    static ET_INSTANCE: OnceCell<ET> = OnceCell::new();
    static mut ET_WRITER: Option<Stdout> = None;

    impl EmbeddedTrace<std_embedded_time::StandardClock, Stdout> for ET {
        fn get() -> &'static Self {
            ET_INSTANCE.get_or_init(|| ET {
                clock: std_embedded_time::StandardClock::default(),
            })
        }

        fn get_writer() -> &'static mut Stdout {
            unsafe {
                if ET_WRITER.is_none() {
                    ET_WRITER = Some(Stdout::new());
                }

                ET_WRITER.as_mut().unwrap()
            }
        }

        fn read_clock(&self) -> embedded_time::Instant<std_embedded_time::StandardClock> {
            self.clock.try_now().unwrap()
        }
    }

    #[test]
    fn basic_snapshot() {
        let et = ET::get();

        let start = et.start_snapshot();
        let sn = et.end_snapshot(start, "basic_snapshot");

        write!(ET::get_writer(), "{}\n", sn).unwrap();
    }
}
