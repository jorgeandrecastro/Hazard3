# Changelog

## 0.1.0

- Initial `xarxa-driver-riscv` release.
- Republished the upstream `xarxa-driver` source pinned at revision `8f6f33b131deb9ff3290e0e01079537e1ed7b2ad`.
- Published under the `xarxa-driver-riscv` package name while keeping the Rust library name `xarxa_driver`.
- Reduced the `xarxa-riscv` Cargo feature count to comply with crates.io's 300-feature limit by removing the redundant `iface-count-7`, `iface-addr-count-7`, and `route-count-7` configuration variants.
- Preserved the remaining network protocol features and configuration options.
