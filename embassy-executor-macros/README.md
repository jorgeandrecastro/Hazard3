# embassy-executor-macros

[![Crates.io](https://img.shields.io/crates/v/embassy-executor-macros-riscv.svg)](https://crates.io/crates/embassy-executor-macros-riscv)
[![docs.rs](https://docs.rs/embassy-executor-macros-riscv/badge.svg)](https://docs.rs/embassy-executor-macros-riscv)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20or%20Apache--2.0-blue.svg)](https://opensource.org/licenses/MIT)

> **Fork of Embassy for RISC-V.** `embassy-executor-macros-riscv` is an independent, **unmodified copy** of [`embassy-executor-macros`](https://github.com/embassy-rs/embassy/tree/main/embassy-executor-macros)
> taken from the Embassy `main` branch, maintained by [Jorge Andre Castro](https://github.com/jorgeandrecastro) at
> <https://github.com/jorgeandrecastro/embassy-riscv>. It is republished under another name only because [`embassy-rp-riscv`](https://crates.io/crates/embassy-rp-riscv)
> needs the unreleased changes of `main`, and crates.io does not accept git dependencies.
> The library name is still `embassy_executor_macros`; use it with `package = "embassy-executor-macros-riscv"` in your `Cargo.toml`.
> **Use the `-riscv` versions of all Embassy crates together** (`embassy-time-riscv`, `embassy-executor-riscv`, `embassy-sync-riscv`, ...) to avoid duplicate/incompatible copies in your build.

An [Embassy](https://embassy.dev) project.

NOTE: Do not use this crate directly. The macros are re-exported by `embassy-executor`.

## Acknowledgements

This crate is a derivative work of the [Embassy](https://embassy.dev) project ([embassy-rs/embassy](https://github.com/embassy-rs/embassy)),
Copyright (c) Embassy project contributors, used under the MIT OR Apache-2.0 licenses. The original copyright and license notices are
kept (see `LICENSE-MIT` and `LICENSE-APACHE`). Embassy and its authors are not affiliated with, and do not endorse, this fork.
Thank you to the Embassy team and contributors for their work.
