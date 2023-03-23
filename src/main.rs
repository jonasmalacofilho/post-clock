#![cfg(all(target_os = "linux", any(target_arch = "x86", target_arch = "x86_64")))]
#![deny(
    clippy::missing_safety_doc,
    clippy::undocumented_unsafe_blocks,
    unsafe_op_in_unsafe_fn
)]

use core::arch::asm;

use std::io;
use std::marker::PhantomData;
use std::ops::Add;
use std::thread;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use capctl::caps::CapState;
use time::OffsetDateTime;

// io_delay_type values are defined in:
// https://elixir.bootlin.com/linux/v6.2.6/source/arch/x86/kernel/io_delay.c#L16
const IO_DELAY_TYPE_PATH: &str = "/proc/sys/kernel/io_delay_type";
const IO_DELAY_TYPE_0X80: u8 = 0;

fn main() -> Result<()> {
    if current_io_delay_type().context("failed to read io_delay type")? == IO_DELAY_TYPE_0X80 {
        bail!("port 0x80 in use by the kernel for io_delay (see corresponding boot parameter)")
    }

    // SAFETY: access to I/O port 0x80 should™ not introduce memory unsafety on x86 and x86_64, and
    // we checked that the kernel isn't using port 0x80 for io_delay.
    let mut port = unsafe { Port::new(0x80).context("access to port 0x80 was denied")? };

    // Drop all permitted, inherittable and effective capabilities. Note that this alone doesn't
    // prevent this thread from regaining capabilities due to the special treatment of processes
    // with UID 0; see `capabilities(7)` and the provided `post-clock.service` for systemd.
    CapState::empty()
        .set_current()
        .context("failed to clear capabilities")?;

    loop {
        let now = OffsetDateTime::now_local()?;

        // Travel forward into the future so that `(now.hour(), now.minute())` is rounded to the
        // nearest minute.
        let now = now.add(Duration::from_secs(30));

        // Only update the current time every 30 seconds.
        for _ in 0..10 {
            port.write_byte(seven_segments(now.hour()));
            thread::sleep(Duration::from_secs(1));

            port.write_byte(seven_segments(now.minute()));
            thread::sleep(Duration::from_secs(1));

            port.write_byte(0xCC);
            thread::sleep(Duration::from_secs(1));
        }
    }
}

fn current_io_delay_type() -> Result<u8> {
    let io_delay_type = std::fs::read_to_string(IO_DELAY_TYPE_PATH)
        .with_context(|| format!("failed to read {}", IO_DELAY_TYPE_PATH))?;

    let io_delay_type = io_delay_type.trim_end();

    io_delay_type
        .parse()
        .with_context(|| format!("failed to parse `{}`", io_delay_type))
}

struct Port {
    address: u16,
    // Access permission to an I/O port on Linux is per thread, so make this `!Send` and `!Sync`.
    phantom: PhantomData<*mut ()>,
}

impl Port {
    /// Gains access to the I/O port at `address`.
    ///
    /// # Safety
    ///
    /// The I/O port must be safe to access in the context that the program will execute: reading
    /// from it or writing to it must not affect the kernel or other running processes in
    /// significant ways.
    unsafe fn new(address: u16) -> Result<Port> {
        // SAFETY: the caller ensures that's safe to access port at `address`.
        if unsafe { libc::ioperm(address.into(), 1, 1) } != 0 {
            return Err(io::Error::last_os_error().into());
        }

        Ok(Port {
            address,
            phantom: PhantomData,
        })
    }

    /// Writes a byte to the port.
    fn write_byte(&mut self, value: u8) {
        // SAFETY: `self` ensures that this thread has access to the underlying port: the necessary
        // permission bit has been set (with `ioperm`), and accessing this port was deemed safe.
        unsafe {
            // Note: memory mapped I/O is a thing, so don't add options(nomem).
            asm!("out dx, al", in("dx") self.address, in("al") value, options(nostack, preserves_flags));
        };
    }
}

/// Value that, in hexadecimal, visually matches the decimal representation of `number`.
///
/// # Panics
///
/// If `debug_assertions` are enabled, panics if `number` has more than two decimal digits.
fn seven_segments(number: u8) -> u8 {
    debug_assert!(number <= 99);
    (number / 10) << 4 | (number % 10)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_digit_numbers_in_seven_segments() {
        assert_eq!(seven_segments(6), 0x06);
        assert_eq!(seven_segments(12), 0x12);
        assert_eq!(seven_segments(59), 0x59);
    }
}
