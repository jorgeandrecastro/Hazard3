# hazard3-embedded-hal

> **Published replacement:** `hazard3-embedded-hal` replaces the formerly published `embassy-embedded-hal-riscv`.
> See the project repository for the publication status of the Hazard3 packages.
> Rust library names and upstream license and attribution notices are retained.

[![Crates.io](https://img.shields.io/crates/v/hazard3-embedded-hal.svg)](https://crates.io/crates/hazard3-embedded-hal)
[![docs.rs](https://docs.rs/hazard3-embedded-hal/badge.svg)](https://docs.rs/hazard3-embedded-hal)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20or%20Apache--2.0-blue.svg)](https://opensource.org/licenses/MIT)

> **Fork of Embassy for RISC-V.** `hazard3-embedded-hal` is an independent, **unmodified copy** of [`embassy-embedded-hal`](https://github.com/embassy-rs/embassy/tree/main/embassy-embedded-hal)
> taken from the Embassy `main` branch, maintained by [Jorge Andre Castro](https://github.com/jorgeandrecastro) at
> <https://github.com/jorgeandrecastro/Hazard3>. It was originally republished under another name because [`hazard3-rp`](https://crates.io/crates/hazard3-rp)
> needs the unreleased changes of `main`, and crates.io does not accept git dependencies.
> The library name is still `embassy_embedded_hal`; use it with `package = "hazard3-embedded-hal"` in your `Cargo.toml`.
> **Use the `hazard3-*` versions of the forked crates together** (`hazard3-time`, `hazard3-executor`, `hazard3-sync`, ...) to avoid duplicate/incompatible copies in your build.

Collection of utilities to use `embedded-hal` and `embedded-storage` traits with Embassy.

- Shared SPI and I2C buses, both blocking and async, with a `SetConfig` trait allowing changing bus configuration (e.g. frequency) between devices on the same bus.
- Async utilities
    - Adapters to convert from blocking to (fake) async.
    - Adapters to insert yields on trait operations.
- Flash utilities
    - Split a flash memory into smaller partitions.
    - Concatenate flash memories together.
    - Simulated in-memory flash.

## Acknowledgements

This crate is a derivative work of the [Embassy](https://embassy.dev) project ([embassy-rs/embassy](https://github.com/embassy-rs/embassy)),
Copyright (c) Embassy project contributors, used under the MIT OR Apache-2.0 licenses. The original copyright and license notices are
kept (see `LICENSE-MIT` and `LICENSE-APACHE`). Embassy and its authors are not affiliated with, and do not endorse, this fork.
Thank you to the Embassy team and contributors for their work.
