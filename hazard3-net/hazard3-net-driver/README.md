# hazard3-net-driver

> **Migration in progress:** `hazard3-net-driver` replaces the formerly published `xarxa-driver-riscv`.
> See the project repository for the publication status of the Hazard3 packages.
> Rust library names and upstream license and attribution notices are retained.

[![docs.rs](https://docs.rs/hazard3-net-driver/badge.svg)](https://docs.rs/hazard3-net-driver)
[![crates.io](https://img.shields.io/crates/v/hazard3-net-driver.svg)](https://crates.io/crates/hazard3-net-driver)

> **RISC-V Embassy fork.** `hazard3-net-driver` is an exact copy of upstream
> [`xarxa-driver`](https://github.com/embassy-rs/xarxa/tree/main/xarxa-driver) at commit
> `8f6f33b131deb9ff3290e0e01079537e1ed7b2ad`, republished for use by
> [`embassy-riscv`](https://github.com/jorgeandrecastro/embassy-riscv).
> The library name stays `xarxa_driver`; depend on this package with
> `package = "hazard3-net-driver"` in your `Cargo.toml`.

The driver interface of the [`xarxa`](https://crates.io/crates/xarxa) network
stack.

Driver implementations should depend on `xarxa-driver` instead of the full `xarxa` crate.
