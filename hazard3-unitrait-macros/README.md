# hazard3-unitrait-macros

> **Published replacement:** `hazard3-unitrait-macros` replaces the formerly published `unitrait-macros-riscv`.
> See the project repository for the publication status of the Hazard3 packages.
> Upstream license and attribution notices are retained.

[![Crates.io](https://img.shields.io/crates/v/hazard3-unitrait-macros.svg)](https://crates.io/crates/hazard3-unitrait-macros)
[![docs.rs](https://docs.rs/hazard3-unitrait-macros/badge.svg)](https://docs.rs/hazard3-unitrait-macros)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20or%20Apache--2.0-blue.svg)](https://opensource.org/licenses/MIT)

> **Fork for RISC-V.** `hazard3-unitrait-macros` is an independent, fork of `unitrait-macros` (unreleased 2.0.0 revision `bb96601` of [embassy-rs/unitrait](https://github.com/embassy-rs/unitrait)),
> maintained by [Jorge Andre Castro](https://github.com/jorgeandrecastro) at <https://github.com/jorgeandrecastro/Hazard3>.
> It was originally republished under another name because [`hazard3-rp`](https://crates.io/crates/hazard3-rp) needs these unreleased changes and crates.io does not accept git dependencies.
> Add `hazard3-unitrait-macros` directly in Cargo.toml and import it as `hazard3_unitrait_macros`.
> Use the related Hazard3 crates together to avoid duplicate/incompatible copies in your build.

Implementation detail of the `unitrait` crate

## Migration to 0.2.0

The Rust library is now `hazard3_unitrait_macros` instead of `unitrait_macros`. Update imports and macro paths; this is a source-breaking change. Add the package directly without a `package` alias:

```toml
[dependencies]
hazard3-unitrait-macros = "0.2"
```

Use compatible new versions of the other Hazard3 crates together. These versions are pending publication; older published versions may use different library names.

## Acknowledgements

This crate is a derivative work of the [Embassy](https://embassy.dev) project ([embassy-rs](https://github.com/embassy-rs)),
Copyright (c) Embassy project contributors, used under the MIT OR Apache-2.0 licenses. The original copyright and license notices are
kept (see `LICENSE-MIT` and `LICENSE-APACHE`). Embassy and its authors are not affiliated with, and do not endorse, this fork.
Thank you to the Embassy team and contributors for their work.
