# hazard3-net-driver

> **Migration in progress:** `hazard3-net-driver` replaces the formerly published `xarxa-driver-riscv`.
> See the project repository for the publication status of the Hazard3 packages.
> Upstream license and attribution notices are retained.

[![docs.rs](https://docs.rs/hazard3-net-driver/badge.svg)](https://docs.rs/hazard3-net-driver)
[![crates.io](https://img.shields.io/crates/v/hazard3-net-driver.svg)](https://crates.io/crates/hazard3-net-driver)

> **RISC-V Embassy fork.** `hazard3-net-driver` is a fork of upstream
> [`xarxa-driver`](https://github.com/embassy-rs/xarxa/tree/main/xarxa-driver) at commit
> `8f6f33b131deb9ff3290e0e01079537e1ed7b2ad`, republished for use by
> [`Hazard3`](https://github.com/jorgeandrecastro/Hazard3).
> Add `hazard3-net-driver` directly in Cargo.toml and import it as `hazard3_net_driver`.

The driver interface of the [`xarxa`](https://crates.io/crates/xarxa) network
stack.

Driver implementations should depend on `hazard3-net-driver` instead of the full `hazard3-net` crate.

## Migration to 0.2.0

The Rust library is now `hazard3_net_driver` instead of `xarxa_driver`. Update imports and macro paths; this is a source-breaking change. Add the package directly without a `package` alias:

```toml
[dependencies]
hazard3-net-driver = "0.2"
```

Use compatible new versions of the other Hazard3 crates together. These versions are pending publication; older published versions may use different library names.
