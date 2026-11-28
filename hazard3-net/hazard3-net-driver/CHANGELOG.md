# Changelog

## Unreleased (0.2.0)

### Breaking changes

- **Breaking:** rename the Rust library `xarxa_driver` to `hazard3_net_driver`; update imports and macro paths.
- Use matching new Hazard3 dependency versions where applicable.

### Other changes

- Rename package `xarxa-driver-riscv` to `hazard3-net-driver`.

## 0.1.0

- Initial `xarxa-driver-riscv` release.
- Republished the upstream `xarxa-driver` source pinned at revision `8f6f33b131deb9ff3290e0e01079537e1ed7b2ad`.
- Published under the `xarxa-driver-riscv` package name while keeping the Rust library name `xarxa_driver`.
- Reduced the `xarxa-riscv` Cargo feature count to comply with crates.io's 300-feature limit by removing the redundant `iface-count-7`, `iface-addr-count-7`, and `route-count-7` configuration variants.
- Preserved the remaining network protocol features and configuration options.
