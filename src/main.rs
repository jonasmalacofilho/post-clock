#![cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#![cfg(target_os = "linux")]
#![deny(clippy::missing_safety_doc)]
#![deny(clippy::undocumented_unsafe_blocks)]
#![deny(unsafe_op_in_unsafe_fn)]

mod display;

use std::ops::Add;
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use capctl::caps::CapState;
use time::OffsetDateTime;

use crate::display::Display;

fn main() -> Result<()> {
    let mut display = Display::new()?;

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
            display.decimal(now.hour());
            thread::sleep(Duration::from_secs(1));

            display.decimal(now.minute());
            thread::sleep(Duration::from_secs(1));

            display.hexadecimal(0xCC);
            thread::sleep(Duration::from_secs(1));
        }
    }
}
