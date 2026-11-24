# hazard3-usb-driver

> **Published replacement:** `hazard3-usb-driver` replaces the formerly published `embassy-usb-driver-riscv`.
> See the project repository for the publication status of the Hazard3 packages.
> Rust library names and upstream license and attribution notices are retained.

[![Crates.io](https://img.shields.io/crates/v/hazard3-usb-driver.svg)](https://crates.io/crates/hazard3-usb-driver)
[![docs.rs](https://docs.rs/hazard3-usb-driver/badge.svg)](https://docs.rs/hazard3-usb-driver)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20or%20Apache--2.0-blue.svg)](https://opensource.org/licenses/MIT)

> **Fork of Embassy for RISC-V.** `hazard3-usb-driver` is an independent, **unmodified copy** of [`embassy-usb-driver`](https://github.com/embassy-rs/embassy/tree/main/embassy-usb-driver)
> taken from the Embassy `main` branch, maintained by [Jorge Andre Castro](https://github.com/jorgeandrecastro) at
> <https://github.com/jorgeandrecastro/Hazard3>. It was originally republished under another name because [`hazard3-rp`](https://crates.io/crates/hazard3-rp)
> needs the unreleased changes of `main`, and crates.io does not accept git dependencies.
> The library name is still `embassy_usb_driver`; use it with `package = "hazard3-usb-driver"` in your `Cargo.toml`.
> **Use the `hazard3-*` versions of the forked crates together** (`hazard3-time`, `hazard3-executor`, `hazard3-sync`, ...) to avoid duplicate/incompatible copies in your build.

This crate contains the driver traits for [`embassy-usb`]. HAL/BSP crates can implement these
traits to add support for using `embassy-usb` for a given chip/platform.

The traits are kept in a separate crate so that breaking changes in the higher-level [`embassy-usb`]
APIs don't cause a semver-major bump of this crate. This allows existing HALs/BSPs to be used
with the newer `embassy-usb` without needing updates.

If you're writing an application using USB, you should depend on the main [`embassy-usb`] crate
instead of this one.

[`embassy-usb`]: https://crates.io/crates/embassy-usb

## Interoperability

This crate can run on any executor.

## Acknowledgements

This crate is a derivative work of the [Embassy](https://embassy.dev) project ([embassy-rs/embassy](https://github.com/embassy-rs/embassy)),
Copyright (c) Embassy project contributors, used under the MIT OR Apache-2.0 licenses. The original copyright and license notices are
kept (see `LICENSE-MIT` and `LICENSE-APACHE`). Embassy and its authors are not affiliated with, and do not endorse, this fork.
Thank you to the Embassy team and contributors for their work.
