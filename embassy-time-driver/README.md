# embassy-time-driver

[![Crates.io](https://img.shields.io/crates/v/embassy-time-driver-riscv.svg)](https://crates.io/crates/embassy-time-driver-riscv)
[![docs.rs](https://docs.rs/embassy-time-driver-riscv/badge.svg)](https://docs.rs/embassy-time-driver-riscv)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20or%20Apache--2.0-blue.svg)](https://opensource.org/licenses/MIT)

> **Fork of Embassy for RISC-V.** `embassy-time-driver-riscv` is an independent, **unmodified copy** of [`embassy-time-driver`](https://github.com/embassy-rs/embassy/tree/main/embassy-time-driver)
> taken from the Embassy `main` branch, maintained by [Jorge Andre Castro](https://github.com/jorgeandrecastro) at
> <https://github.com/jorgeandrecastro/embassy-riscv>. It is republished under another name only because [`embassy-rp-riscv`](https://crates.io/crates/embassy-rp-riscv)
> needs the unreleased changes of `main`, and crates.io does not accept git dependencies.
> The library name is still `embassy_time_driver`; use it with `package = "embassy-time-driver-riscv"` in your `Cargo.toml`.
> **Use the `-riscv` versions of all Embassy crates together** (`embassy-time-riscv`, `embassy-executor-riscv`, `embassy-sync-riscv`, ...) to avoid duplicate/incompatible copies in your build.

This crate contains the driver trait necessary for adding [`embassy-time`](https://crates.io/crates/embassy-time) support
for a new hardware platform.

If you want to *use* `embassy-time` with already made drivers, you should depend on the main `embassy-time` crate, not on this crate.

If you are writing a driver, you  should depend only on this crate, not on the main `embassy-time` crate.
This will allow your driver to continue working for newer `embassy-time` major versions, without needing an update,
if the driver trait has not had breaking changes.

## How it works

`embassy-time` is backed by a global "time driver" specified at build time.
Only one driver can be active in a program.

All methods and structs transparently call into the active driver. This makes it
possible for libraries to use `embassy-time` in a driver-agnostic way without
requiring generic parameters.

## Acknowledgements

This crate is a derivative work of the [Embassy](https://embassy.dev) project ([embassy-rs/embassy](https://github.com/embassy-rs/embassy)),
Copyright (c) Embassy project contributors, used under the MIT OR Apache-2.0 licenses. The original copyright and license notices are
kept (see `LICENSE-MIT` and `LICENSE-APACHE`). Embassy and its authors are not affiliated with, and do not endorse, this fork.
Thank you to the Embassy team and contributors for their work.
