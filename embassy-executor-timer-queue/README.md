# embassy-executor-time-queue

[![Crates.io](https://img.shields.io/crates/v/embassy-executor-timer-queue-riscv.svg)](https://crates.io/crates/embassy-executor-timer-queue-riscv)
[![docs.rs](https://docs.rs/embassy-executor-timer-queue-riscv/badge.svg)](https://docs.rs/embassy-executor-timer-queue-riscv)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20or%20Apache--2.0-blue.svg)](https://opensource.org/licenses/MIT)

> **Fork for RISC-V.** `embassy-executor-timer-queue-riscv` is an independent, **unmodified copy** of `embassy-executor-timer-queue` (Embassy `main` branch),
> maintained by [Jorge Andre Castro](https://github.com/jorgeandrecastro) at <https://github.com/jorgeandrecastro/embassy-riscv>.
> It is republished under another name only because [`embassy-rp-riscv`](https://crates.io/crates/embassy-rp-riscv) needs these unreleased changes and crates.io does not accept git dependencies.
> The library name is still `embassy_executor_timer_queue`; use it with `package = "embassy-executor-timer-queue-riscv"` in your `Cargo.toml`.
> Use the `-riscv` versions of all the related Embassy crates together to avoid duplicate/incompatible copies in your build.

This crate defines the timer queue item that embassy-executor provides, and a way to access it, for
executor-integrated timer queues. The crate decouples the release cycle of embassy-executor from
that of the queue implementations'.

As a HAL implementer, you only need to depend on this crate if you want to implement executor-integrated
timer queues yourself, without using [`embassy-time-queue-utils`](https://crates.io/crates/embassy-time-queue-utils).

As a HAL user, you should not need to depend on this crate.

## Acknowledgements

This crate is a derivative work of the [Embassy](https://embassy.dev) project ([embassy-rs](https://github.com/embassy-rs)),
Copyright (c) Embassy project contributors, used under the MIT OR Apache-2.0 licenses. The original copyright and license notices are
kept (see `LICENSE-MIT` and `LICENSE-APACHE`). Embassy and its authors are not affiliated with, and do not endorse, this fork.
Thank you to the Embassy team and contributors for their work.
