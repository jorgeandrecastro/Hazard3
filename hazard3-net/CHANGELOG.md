# Changelog

## Unreleased

- Rename package `xarxa-riscv` to `hazard3-net` while retaining the Rust library name.

## 0.1.1 - 2026-11-20

- Removed `tcp-cubic` from the default features to avoid enabling two mutually exclusive TCP
  congestion-control implementations.
- Removed `log` from the default features to avoid enabling `log` and `defmt` simultaneously.

## 0.1.0

- Initial `xarxa-riscv` release.
- Republished the upstream `xarxa` source pinned at revision
  `8f6f33b131deb9ff3290e0e01079537e1ed7b2ad`.
- Published under the `xarxa-riscv` package name while keeping the Rust library name `xarxa`.
