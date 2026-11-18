# rp-pac-riscv

[![Crates.io](https://img.shields.io/crates/v/rp-pac-riscv.svg)](https://crates.io/crates/rp-pac-riscv)
[![docs.rs](https://docs.rs/rp-pac-riscv/badge.svg)](https://docs.rs/rp-pac-riscv)

> **Fork of [`rp-pac`](https://github.com/embassy-rs/rp-pac)**, published only so that [`embassy-rp-riscv`](https://crates.io/crates/embassy-rp-riscv) can be published on crates.io.
> See "Why this fork exists" below.

This is a [Peripheral Access Crate](https://rust-embedded.github.io/book/start/registers.html) for Raspberry Pi Silicon microcontrollers.


This crate has been automatically generated from the SVD files in [pico-sdk](https://github.com/raspberrypi/pico-sdk), using [chiptool](https://github.com/embassy-rs/rp-pac-riscvchiptool/). Fixes are added to the SVD file to make the
crate more amenable to writing HALs with, such as converting sets of identical registers/fields to arrays, merging identical registers and enums, etc.

This crate is used for the [`embassy-rp`](github.com/embassy-rs/embassy/) Rust Hardware Abstraction Layer (HAL) for the RPxxx microcontrollers.

## Supported chips

- **RP2040**: [Datasheet](https://datasheets.raspberrypi.org/rp2040/rp2040_datasheet.pdf)
- **RP2350A, RP2350B, RP2354A, RP2354B**: [Datasheet](https://datasheets.raspberrypi.com/rp2350/rp2350-datasheet.pdf)

## Usage

This crate has **no default chip**: you must enable exactly one of the `rp2040` or `rp235x` features, otherwise the build fails with
`You must enable either the rp2040 or the rp235x Cargo features` (and `file not found for module inner`).

```toml
rp-pac = { package = "rp-pac-riscv", version = "0.1", features = ["rp235x"] }
```

This also applies when working on the crate itself:

```sh
cargo doc --features rp235x
cargo publish --dry-run --features rp235x
cargo publish --features rp235x
```

## License

The contents of this crate are auto-generated and licensed under the same terms as the underlying SVD file, which is licensed by Raspberry Pi Trading Ltd under a BSD-3-Clause licence.

See [`LICENSE-BSD-3-Clause`](./LICENSE-BSD-3-Clause).

## Why this fork exists

`embassy-rp` (and so `embassy-rp-riscv`) depends on a **git revision** of `rp-pac` (`9e59c6d`, version 7.0.0 plus later changes), because
the latest `rp-pac` published on crates.io (7.0.0, January 2025) is older than what the current Embassy code needs. crates.io rejects any
crate that has a git dependency, so `embassy-rp-riscv` could not be published as is.

This crate is an **unmodified copy of that exact revision** of [`embassy-rs/rp-pac`](https://github.com/embassy-rs/rp-pac) (generated code,
no change in the register definitions), republished under another name so that the dependency can come from crates.io:

* the library name stays `rp_pac`, so code using `rp_pac::...` works unchanged;
* only the package name, version (`0.1.0`), repository metadata and this README differ;
* if upstream publishes a recent `rp-pac` on crates.io, `embassy-rp-riscv` should go back to it and this crate can be retired.

All credit goes to the Embassy project (`embassy-rs/rp-pac`) and to Raspberry Pi Ltd, the author of the SVD files this crate is generated from.
Not affiliated with or endorsed by either of them.
