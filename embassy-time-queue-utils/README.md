# embassy-time-queue-utils

[![Crates.io](https://img.shields.io/crates/v/embassy-time-queue-utils-riscv.svg)](https://crates.io/crates/embassy-time-queue-utils-riscv)
[![docs.rs](https://docs.rs/embassy-time-queue-utils-riscv/badge.svg)](https://docs.rs/embassy-time-queue-utils-riscv)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20or%20Apache--2.0-blue.svg)](https://opensource.org/licenses/MIT)

> **Fork for RISC-V.** `embassy-time-queue-utils-riscv` is an independent, **unmodified copy** of `embassy-time-queue-utils` (Embassy `main` branch),
> maintained by [Jorge Andre Castro](https://github.com/jorgeandrecastro) at <https://github.com/jorgeandrecastro/embassy-riscv>.
> It is republished under another name only because [`embassy-rp-riscv`](https://crates.io/crates/embassy-rp-riscv) needs these unreleased changes and crates.io does not accept git dependencies.
> The library name is still `embassy_time_queue_utils`; use it with `package = "embassy-time-queue-utils-riscv"` in your `Cargo.toml`.
> Use the `-riscv` versions of all the related Embassy crates together to avoid duplicate/incompatible copies in your build.

This crate contains timer queues to help implementing an [`embassy-time-driver`](https://crates.io/crates/embassy-time-driver).

As a HAL user, you should not need to depend on this crate.

As a HAL implementer, you need to depend on this crate if you want to implement a time driver,
but how you should do so is documented in `embassy-time-driver`.

## Acknowledgements

This crate is a derivative work of the [Embassy](https://embassy.dev) project ([embassy-rs](https://github.com/embassy-rs)),
Copyright (c) Embassy project contributors, used under the MIT OR Apache-2.0 licenses. The original copyright and license notices are
kept (see `LICENSE-MIT` and `LICENSE-APACHE`). Embassy and its authors are not affiliated with, and do not endorse, this fork.
Thank you to the Embassy team and contributors for their work.
