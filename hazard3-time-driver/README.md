# hazard3-time-driver

> **Published replacement:** `hazard3-time-driver` replaces the formerly published `embassy-time-driver-riscv`.
> See the project repository for the publication status of the Hazard3 packages.
> Upstream license and attribution notices are retained.

[![Crates.io](https://img.shields.io/crates/v/hazard3-time-driver.svg)](https://crates.io/crates/hazard3-time-driver)
[![docs.rs](https://docs.rs/hazard3-time-driver/badge.svg)](https://docs.rs/hazard3-time-driver)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20or%20Apache--2.0-blue.svg)](https://opensource.org/licenses/MIT)

> **Fork of Embassy for RISC-V.** `hazard3-time-driver` is an independent, fork of [`embassy-time-driver`](https://github.com/embassy-rs/embassy/tree/main/embassy-time-driver)
> taken from the Embassy `main` branch, maintained by [Jorge Andre Castro](https://github.com/jorgeandrecastro) at
> <https://github.com/jorgeandrecastro/Hazard3>. It was originally republished under another name because [`hazard3-rp`](https://crates.io/crates/hazard3-rp)
> needs the unreleased changes of `main`, and crates.io does not accept git dependencies.
> Add `hazard3-time-driver` directly in Cargo.toml and import it as `hazard3_time_driver`.
> **Use the `hazard3-*` versions of the forked crates together** (`hazard3-time`, `hazard3-executor`, `hazard3-sync`, ...) to avoid duplicate/incompatible copies in your build.

This crate contains the driver trait necessary for adding [`embassy-time`](https://crates.io/crates/embassy-time) support
for a new hardware platform.

If you want to *use* `embassy-time` with already made drivers, you should depend on the main `embassy-time` crate, not on this crate.

If you are writing a driver, you  should depend only on this crate, not on the main `embassy-time` crate.
This will allow your driver to continue working for newer `embassy-time` major versions, without needing an update,
if the driver trait has not had breaking changes.

> **Note (0.1.1):** the `links = "embassy-time"` key was removed from the manifest. Make sure only one copy of this crate (and a single `tick-*` configuration) ends up in your binary.

## Migration to 0.2.0

The Rust library is now `hazard3_time_driver` instead of `embassy_time_driver`. Update imports and macro paths; this is a source-breaking change. Add the package directly without a `package` alias:

```toml
[dependencies]
hazard3-time-driver = "0.2"
```

Use compatible new versions of the other Hazard3 crates together. These versions are pending publication; older published versions may use different library names.

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
