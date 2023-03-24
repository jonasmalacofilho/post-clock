use core::arch::asm;

use std::io;
use std::marker::PhantomData;

use anyhow::{ensure, Context, Result};

const PORT_0X80: u16 = 0x80;

// io_delay_type values are defined in:
// https://elixir.bootlin.com/linux/v6.2.6/source/arch/x86/kernel/io_delay.c#L16
const IO_DELAY_TYPE_PATH: &str = "/proc/sys/kernel/io_delay_type";
const IO_DELAY_TYPE_0X80: u8 = 0;

/// Model a 2-digit seven-segment display for POST codes from I/O port 0x80.
///
/// Access permission bits to I/O ports are per thread, so this type is neither `Send` nor `Sync`.
pub struct Display(PhantomData<*mut ()>);

impl Display {
    /// Gains access to the underlying I/O port of the display.
    pub fn new() -> Result<Self> {
        ensure!(
            linux_io_delay().context("failed to read io_delay type")? != IO_DELAY_TYPE_0X80,
            "port 0x80 in use by the kernel for io_delay (see corresponding boot parameter)"
        );

        // SAFETY: access to I/O port 0x80 should™ not introduce memory unsafety on x86 and x86_64,
        // and we checked that the kernel isn't using port 0x80 for io_delay.
        if unsafe { libc::ioperm(PORT_0X80.into(), 1, 1) } != 0 {
            return Err(io::Error::last_os_error().into());
        }

        Ok(Display(PhantomData))
    }

    /// Displays a hexadecimal value.
    pub fn hexadecimal(&mut self, value: u8) {
        // SAFETY: `self` ensures that this thread has access to port 0x80; `out` doesn't clobber
        // registers or flags, or accesses memory[^1].
        //
        // [^1]: Except if memory-mapped I/O is used, but that would require `unsafe` block
        // elsewheres, or access to `/dev/port` (the latter is a sibling of `/dev/mem`, which is
        // generally accepted to be outside of the Rust memory safety garantees, otherwise all file
        // I/O would be unsafe).
        unsafe {
            asm!(
                "out dx, al",
                in("dx") PORT_0X80,
                in("al") value,
                options(nomem, nostack, preserves_flags)
            );
        };
    }

    /// Displays a decimal value.
    ///
    /// # Panics
    ///
    /// If `debug_assertions` are enabled, panics if `number` has more than two decimal digits.
    pub fn decimal(&mut self, value: u8) {
        self.hexadecimal(to_hex_lookalike(value));
    }
}

fn linux_io_delay() -> Result<u8> {
    let io_delay_type = std::fs::read_to_string(IO_DELAY_TYPE_PATH)
        .with_context(|| format!("failed to read {}", IO_DELAY_TYPE_PATH))?;

    let io_delay_type = io_delay_type.trim_end();

    io_delay_type
        .parse()
        .with_context(|| format!("failed to parse `{}`", io_delay_type))
}

fn to_hex_lookalike(number: u8) -> u8 {
    debug_assert!(number <= 99);
    (number / 10) << 4 | (number % 10)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_to_hex_lookalikes() {
        assert_eq!(to_hex_lookalike(6), 0x06);
        assert_eq!(to_hex_lookalike(12), 0x12);
        assert_eq!(to_hex_lookalike(59), 0x59);
    }
}
