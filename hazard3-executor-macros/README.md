# hazard3-executor-macros

> **Published replacement:** `hazard3-executor-macros` replaces the formerly published `embassy-executor-macros-riscv`.
> See the project repository for the publication status of the Hazard3 packages.
> Upstream license and attribution notices are retained.

[![Crates.io](https://img.shields.io/crates/v/hazard3-executor-macros.svg)](https://crates.io/crates/hazard3-executor-macros)
[![docs.rs](https://docs.rs/hazard3-executor-macros/badge.svg)](https://docs.rs/hazard3-executor-macros)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20or%20Apache--2.0-blue.svg)](https://opensource.org/licenses/MIT)

> **Fork of Embassy for RISC-V.** `hazard3-executor-macros` is an independent, fork of [`embassy-executor-macros`](https://github.com/embassy-rs/embassy/tree/main/embassy-executor-macros)
> taken from the Embassy `main` branch, maintained by [Jorge Andre Castro](https://github.com/jorgeandrecastro) at
> <https://github.com/jorgeandrecastro/Hazard3>. It was originally republished under another name because [`hazard3-rp`](https://crates.io/crates/hazard3-rp)
> needs the unreleased changes of `main`, and crates.io does not accept git dependencies.
> Add `hazard3-executor-macros` directly in Cargo.toml and import it as `hazard3_executor_macros`.
> **Use the `hazard3-*` versions of the forked crates together** (`hazard3-time`, `hazard3-executor`, `hazard3-sync`, ...) to avoid duplicate/incompatible copies in your build.

An [Embassy](https://embassy.dev) project.

NOTE: Do not use this crate directly. The macros are re-exported by `embassy-executor`.

## Migration to 0.2.0

The Rust library is now `hazard3_executor_macros` instead of `embassy_executor_macros`. Update imports and macro paths; this is a source-breaking change. Add the package directly without a `package` alias:

```toml
[dependencies]
hazard3-executor-macros = "0.2"
```

Use compatible new versions of the other Hazard3 crates together. These versions are pending publication; older published versions may use different library names.

## Acknowledgements

This crate is a derivative work of the [Embassy](https://embassy.dev) project ([embassy-rs/embassy](https://github.com/embassy-rs/embassy)),
Copyright (c) Embassy project contributors, used under the MIT OR Apache-2.0 licenses. The original copyright and license notices are
kept (see `LICENSE-MIT` and `LICENSE-APACHE`). Embassy and its authors are not affiliated with, and do not endorse, this fork.
Thank you to the Embassy team and contributors for their work.
