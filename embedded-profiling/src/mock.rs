use crate::{prelude::*, EmbeddedTrace};

use once_cell::sync::OnceCell;
use std::io::Write;
use std_embedded_time;

// this whole Stdout rigamarole seems hacky but I dunno how to work around it
pub struct Stdout {
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

pub struct ET {
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

    fn borrow_writer<T, R>(borrower: T) -> R
    where
        T: Fn(&mut Stdout) -> R,
    {
        // SAFETY: `borrow_writer` never called in interrupt context and only
        // ever used here. Therefore, we're always guaranteed that this is
        // the only mutable borrow that ever exists, and it only happens once
        // at a time.
        unsafe {
            if ET_WRITER.is_none() {
                ET_WRITER = Some(Stdout::new());
            }

            borrower(ET_WRITER.as_mut().unwrap())
        }
    }

    fn read_clock(&self) -> embedded_time::Instant<std_embedded_time::StandardClock> {
        self.clock.try_now().unwrap()
    }
}
