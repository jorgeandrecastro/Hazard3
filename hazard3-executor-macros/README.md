# hazard3-executor-macros

> **Published replacement:** `hazard3-executor-macros` replaces the formerly published `embassy-executor-macros-riscv`.
> See the project repository for the publication status of the Hazard3 packages.
> Rust library names and upstream license and attribution notices are retained.

[![Crates.io](https://img.shields.io/crates/v/hazard3-executor-macros.svg)](https://crates.io/crates/hazard3-executor-macros)
[![docs.rs](https://docs.rs/hazard3-executor-macros/badge.svg)](https://docs.rs/hazard3-executor-macros)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20or%20Apache--2.0-blue.svg)](https://opensource.org/licenses/MIT)

> **Fork of Embassy for RISC-V.** `hazard3-executor-macros` is an independent, **unmodified copy** of [`embassy-executor-macros`](https://github.com/embassy-rs/embassy/tree/main/embassy-executor-macros)
> taken from the Embassy `main` branch, maintained by [Jorge Andre Castro](https://github.com/jorgeandrecastro) at
> <https://github.com/jorgeandrecastro/Hazard3>. It was originally republished under another name because [`hazard3-rp`](https://crates.io/crates/hazard3-rp)
> needs the unreleased changes of `main`, and crates.io does not accept git dependencies.
> The library name is still `embassy_executor_macros`; use it with `package = "hazard3-executor-macros"` in your `Cargo.toml`.
> **Use the `hazard3-*` versions of the forked crates together** (`hazard3-time`, `hazard3-executor`, `hazard3-sync`, ...) to avoid duplicate/incompatible copies in your build.

An [Embassy](https://embassy.dev) project.

NOTE: Do not use this crate directly. The macros are re-exported by `embassy-executor`.

## Acknowledgements

This crate is a derivative work of the [Embassy](https://embassy.dev) project ([embassy-rs/embassy](https://github.com/embassy-rs/embassy)),
Copyright (c) Embassy project contributors, used under the MIT OR Apache-2.0 licenses. The original copyright and license notices are
kept (see `LICENSE-MIT` and `LICENSE-APACHE`). Embassy and its authors are not affiliated with, and do not endorse, this fork.
Thank you to the Embassy team and contributors for their work.
