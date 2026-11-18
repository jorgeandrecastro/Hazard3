# embassy-embedded-hal

[![Crates.io](https://img.shields.io/crates/v/embassy-embedded-hal-riscv.svg)](https://crates.io/crates/embassy-embedded-hal-riscv)
[![docs.rs](https://docs.rs/embassy-embedded-hal-riscv/badge.svg)](https://docs.rs/embassy-embedded-hal-riscv)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20or%20Apache--2.0-blue.svg)](https://opensource.org/licenses/MIT)

> **Fork of Embassy for RISC-V.** `embassy-embedded-hal-riscv` is an independent, **unmodified copy** of [`embassy-embedded-hal`](https://github.com/embassy-rs/embassy/tree/main/embassy-embedded-hal)
> taken from the Embassy `main` branch, maintained by [Jorge Andre Castro](https://github.com/jorgeandrecastro) at
> <https://github.com/jorgeandrecastro/embassy-riscv>. It is republished under another name only because [`embassy-rp-riscv`](https://crates.io/crates/embassy-rp-riscv)
> needs the unreleased changes of `main`, and crates.io does not accept git dependencies.
> The library name is still `embassy_embedded_hal`; use it with `package = "embassy-embedded-hal-riscv"` in your `Cargo.toml`.
> **Use the `-riscv` versions of all Embassy crates together** (`embassy-time-riscv`, `embassy-executor-riscv`, `embassy-sync-riscv`, ...) to avoid duplicate/incompatible copies in your build.

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
