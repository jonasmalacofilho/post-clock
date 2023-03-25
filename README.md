# post-clock

Once a system has booted, use the display for POST diagnostic codes as a clock.

## Background on POST diagnostic codes

During the boot process of x86 and x86-64 systems, the firmware typically outputs diagnostic
information to I/O port 0x80. These are called power-on self test (POST) codes, and some
motherboards are able to display them. There are also add-on cards that display POST codes.

After the firmware hands off execution to the OS kernel, no more POST codes are output. At that
point some motherboards can be configured to use the display for other purposes, like on-board
temperature sensor readouts.

However, it's also possible for the kernel and userspace to write arbitrary bytes to I/O port 0x80
and, if the POST code display hasn't been switched to a different input, have it display them.

One small complication is that some programs, including the Linux kernel, may by default use writes
to port 0x80 as a short delay timer.[^1][^2] But this behavior can be disabled: for example, Linux
can be configured to use port 0xED instead, which is also generally safe.

## System requirements and preparation

post-clock requires a recent Linux kernel and a x86 or x86-64 system.

The kernel should be configured to use a port different than 0x80 for I/O delays, or no port at all.
This can be done at build time with `CONFIG_IO_DELAY_0XED` and similar options or, more commonly,
with the `io_delay` boot parameter:

```
io_delay=       [X86] I/O delay method
        0x80
                Standard port 0x80 based delay
        0xed
                Alternate port 0xed based delay (needed on some systems)
        udelay
                Simple two microseconds delay
        none
                No delay
```

Other programs using port 0x80 for short delays should also be configured to use some other port or
delay mechanism.

The motherboard should be configured to _not_ output other information to that display once the
system has booted.

And, finally, the `CAP_SYS_RAWIO` capability is required for the [`ioperm(2)`][man:ioperm] system
call (post-clock will drop it and any other capabilities before entering its main loop).
Alternatively, post-clock can be executed as root.

## Linux packages

| Distribution | Package name |
| :-- | :-- |
| ArchLinux | [`post-clock`<sup>AUR</sup>][pkg:aur] |

## Manual installation

Because post-clock requires the `CAP_SYS_RAWIO` capability to run, it generally shouldn't be
installed in a user-writable location. This unfortunately also means that `cargo install post-clock`
isn't a good fit.[^3]

Instead, the recommended way to manually install post-clock is to clone the repository at the latest
release tag, build with cargo, and copy the resulting executable to the desired location:

```
$ git clone https://github.com/jonasmalacofilho/post-clock --branch v0.1.1
$ cd post-clock
$ cargo build --release
$ sudo install -Dm0755 -t /usr/local/bin/ target/release/post-clock
```

## Running as a service

A systemd system service file is provided in [`post-clock.service`][.service]. After installing it
to a suitable location,[^4] reload all unit files:

```
$ sudo install -Dm0644 -t /etc/systemd/system/ post-clock.service
$ sudo systemctl daemon-reload
```

Then, enable and start the service:

```
$ sudo systemctl enable --now post-clock.service
```

[^1]: http://www.faqs.org/docs/Linux-mini/IO-Port-Programming.html

[^2]: https://www.kernel.org/doc/html/latest/admin-guide/kernel-parameters.html

[^3]: While it's possible to pass `--root /usr/local/bin`, that would require running cargo as root,
  and it would _build_ post-clock as root, which isn't advised.

[^4]: See [`systemd.unit(5)`][man:systemd.unit].

[.service]: https://github.com/jonasmalacofilho/post-clock/blob/main/post-clock.service
[man:ioperm]: https://man7.org/linux/man-pages/man2/ioperm.2.html
[man:systemd.unit]: https://man7.org/linux/man-pages/man5/systemd.unit.5.html
[pkg:aur]: https://aur.archlinux.org/packages/post-clock
