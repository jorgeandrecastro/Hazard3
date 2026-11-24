# hazard3-time-queue-utils

> **Published replacement:** `hazard3-time-queue-utils` replaces the formerly published `embassy-time-queue-utils-riscv`.
> See the project repository for the publication status of the Hazard3 packages.
> Rust library names and upstream license and attribution notices are retained.

[![Crates.io](https://img.shields.io/crates/v/hazard3-time-queue-utils.svg)](https://crates.io/crates/hazard3-time-queue-utils)
[![docs.rs](https://docs.rs/hazard3-time-queue-utils/badge.svg)](https://docs.rs/hazard3-time-queue-utils)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20or%20Apache--2.0-blue.svg)](https://opensource.org/licenses/MIT)

> **Fork for RISC-V.** `hazard3-time-queue-utils` is an independent, **unmodified copy** of `embassy-time-queue-utils` (Embassy `main` branch),
> maintained by [Jorge Andre Castro](https://github.com/jorgeandrecastro) at <https://github.com/jorgeandrecastro/Hazard3>.
> It was originally republished under another name because [`hazard3-rp`](https://crates.io/crates/hazard3-rp) needs these unreleased changes and crates.io does not accept git dependencies.
> The library name is still `embassy_time_queue_utils`; use it with `package = "hazard3-time-queue-utils"` in your `Cargo.toml`.
> Use the related Hazard3 crates together to avoid duplicate/incompatible copies in your build.

This crate contains timer queues to help implementing an [`embassy-time-driver`](https://crates.io/crates/embassy-time-driver).

As a HAL user, you should not need to depend on this crate.

As a HAL implementer, you need to depend on this crate if you want to implement a time driver,
but how you should do so is documented in `embassy-time-driver`.

## Acknowledgements

This crate is a derivative work of the [Embassy](https://embassy.dev) project ([embassy-rs](https://github.com/embassy-rs)),
Copyright (c) Embassy project contributors, used under the MIT OR Apache-2.0 licenses. The original copyright and license notices are
kept (see `LICENSE-MIT` and `LICENSE-APACHE`). Embassy and its authors are not affiliated with, and do not endorse, this fork.
Thank you to the Embassy team and contributors for their work.
