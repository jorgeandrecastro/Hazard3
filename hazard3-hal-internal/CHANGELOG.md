# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->
## 2026-11-28 (0.3.1)

### Breaking changes

- **Breaking:** rename the Rust library `embassy_hal_internal` to `hazard3_hal_internal`; update imports and macro paths.
- Use matching new Hazard3 dependency versions where applicable.

### Other changes

- Repository renamed to [`jorgeandrecastro/Hazard3`](https://github.com/jorgeandrecastro/Hazard3).
- Former `*-riscv` releases remain preserved on crates.io but are yanked and are not recommended for new dependency resolution. - 2026-11-18

- Rename package `embassy-hal-internal-riscv` to `hazard3-hal-internal`.

## 0.1.0 - 2026-11-18

- First release of the `embassy-hal-internal-riscv` fork, based on upstream `embassy-hal-internal` 0.5.0 (changelog of upstream versions omitted).
- Add RISC-V support behind the `riscv` feature (`interrupt_riscv`: machine external interrupt dispatch and `enable_external_irq`).
- Removed the unused `ratio` module.
- Removed the `drop` module (`OnDrop`, `DropBomb`). Use `core::mem::DropGuard` (Rust 1.100) instead.
