# xarxa-driver-riscv

[![docs.rs](https://docs.rs/xarxa-driver-riscv/badge.svg)](https://docs.rs/xarxa-driver-riscv)
[![crates.io](https://img.shields.io/crates/v/xarxa-driver-riscv.svg)](https://crates.io/crates/xarxa-driver-riscv)

> **RISC-V Embassy fork.** `xarxa-driver-riscv` is an exact copy of upstream
> [`xarxa-driver`](https://github.com/embassy-rs/xarxa/tree/main/xarxa-driver) at commit
> `8f6f33b131deb9ff3290e0e01079537e1ed7b2ad`, republished for use by
> [`embassy-riscv`](https://github.com/jorgeandrecastro/embassy-riscv).
> The library name stays `xarxa_driver`; depend on this package with
> `package = "xarxa-driver-riscv"` in your `Cargo.toml`.

The driver interface of the [`xarxa`](https://crates.io/crates/xarxa) network
stack.

Driver implementations should depend on `xarxa-driver` instead of the full `xarxa` crate.
