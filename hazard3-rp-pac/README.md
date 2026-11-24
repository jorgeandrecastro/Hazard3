# hazard3-rp-pac

> **Published replacement:** `hazard3-rp-pac` replaces the formerly published `rp-pac-riscv`.
> See the project repository for the publication status of the Hazard3 packages.
> Rust library names and upstream license and attribution notices are retained.

[![Crates.io](https://img.shields.io/crates/v/hazard3-rp-pac.svg)](https://crates.io/crates/hazard3-rp-pac)
[![docs.rs](https://docs.rs/hazard3-rp-pac/badge.svg)](https://docs.rs/hazard3-rp-pac)

> **Fork of [`rp-pac`](https://github.com/embassy-rs/rp-pac)**, prepared so that [`hazard3-rp`](https://crates.io/crates/hazard3-rp) can be published on crates.io.
>
> This fork provides a RISC-V-compatible Peripheral Access Crate (PAC) for Raspberry Pi RP2350 microcontrollers.
>
> See [Why this fork exists](#why-this-fork-exists).

This crate is a [Peripheral Access Crate](https://rust-embedded.github.io/book/start/registers.html) for Raspberry Pi RP-series microcontrollers.

The register definitions are automatically generated from the SVD files provided by the [Pico SDK](https://github.com/raspberrypi/pico-sdk), using [chiptool](https://github.com/embassy-rs/rp-pac/tree/main/chiptool).

The generated definitions are adapted to make the crate more suitable for writing Hardware Abstraction Layers (HALs), including converting identical register and field groups into arrays and merging identical registers and enumerations.

This crate is intended to be used by the [`hazard3-rp`](https://crates.io/crates/hazard3-rp) Rust Hardware Abstraction Layer (HAL).

## Supported chips

- **RP2350A, RP2350B, RP2354A, RP2354B**
  - [RP2350 Datasheet](https://datasheets.raspberrypi.com/rp2350/rp2350-datasheet.pdf)

> The RP2040 is not currently supported by this RISC-V-oriented fork.

## Architecture

The RP2350 supports both Arm Cortex-M33 and RISC-V execution modes.

This fork specifically targets the **RISC-V architecture**.

The PAC provides:

- Peripheral register definitions.
- RP2350 interrupt enumeration through `Interrupt`.
- Compatibility with the `riscv::InterruptNumber` trait.
- No Cortex-M-specific interrupt vector table.
- No dependency on Cortex-M interrupt handling mechanisms.

RISC-V trap and interrupt handling is delegated to the RISC-V runtime.

The interrupt enumeration retains the hardware interrupt numbers defined by the RP2350. Reserved interrupt numbers (40, 41 and 46) are excluded from valid `Interrupt` values.

## Usage

This crate has no default chip feature. You must explicitly enable the `rp235x` feature.

```toml
rp-pac = { package = "hazard3-rp-pac", version = "0.2", features = ["rp235x"] }
```

The library name remains `rp_pac`, preserving compatibility with existing code using:

```rust
use rp_pac::Interrupt;
```

### Building

When working on the crate itself:

```bash
cargo check \
  --target riscv32imac-unknown-none-elf \
  --no-default-features \
  --features rp235x
```

Documentation:

```bash
cargo doc --features rp235x
```

Before publishing:

```bash
cargo publish --dry-run
cargo publish
```

### RISC-V target

The intended target is:

```text
riscv32imac-unknown-none-elf
```

The RISC-V runtime is responsible for trap and interrupt dispatch.

## Changelog

The following entries describe releases under the former `rp-pac-riscv` name.

### 0.2.0

**RISC-V architecture cleanup and Cortex-M interrupt vector removal.**

This release removes the remaining ARM-specific interrupt vector infrastructure from the RP2350 PAC.

#### Removed

- Removed `src/rp235x/_vectors.rs`.
- Removed the `__INTERRUPTS` static interrupt vector table.
- Removed the PAC-specific `Vector` union.
- Removed ARM-oriented interrupt handler declarations and references.
- Removed the Cortex-M-specific vector table linker section `.vector_table.interrupts`.

#### Changed

- RISC-V trap and interrupt handling is now explicitly delegated to the RISC-V runtime.
- The `Interrupt` enumeration remains available for RP2350 hardware interrupt identification.
- `riscv::InterruptNumber` remains the interface for integrating interrupt numbers with the RISC-V ecosystem.
- Reserved interrupt numbers 40, 41 and 46 are excluded from valid interrupt variants.

#### Validation

The following checks were performed:

- Confirmed that no remaining `__INTERRUPTS`, `rp_pac::Vector` or `_vectors` references exist in the relevant PAC and Embassy source trees.
- Successfully compiled `rp-pac-riscv` for `riscv32imac-unknown-none-elf` with the `rp235x` feature enabled.
- Successfully compiled `embassy-rp-riscv` for the RISC-V target.

The Embassy build still reports 102 compiler warnings. These warnings do not prevent compilation, but should be reviewed separately.

### 0.1.0

- Initial publication of the RP PAC fork.
- Based on the Embassy `rp-pac` upstream revision.
- Renamed the package to `rp-pac-riscv` while preserving the Rust library name `rp_pac`.
- Prepared for crates.io distribution as a dependency of `embassy-rp-riscv`.

## License

The contents of this crate are auto-generated and licensed under the same terms as the underlying SVD files, which are licensed by Raspberry Pi Trading Ltd under the BSD-3-Clause license.

See [`LICENSE-BSD-3-Clause`](./LICENSE-BSD-3-Clause).

## Why this fork exists

`embassy-rp` (and therefore `hazard3-rp`) originally depends on a Git revision of [`rp-pac`](https://github.com/embassy-rs/rp-pac), specifically revision `9e59c6d`, corresponding to version 7.0.0 plus subsequent changes.

The version of `rp-pac` available on crates.io was older than the revision required by the current Embassy implementation.

Since crates.io does not allow publishing a crate with a Git dependency, `hazard3-rp` needed a crates.io-compatible PAC dependency.

This fork is based on that upstream revision and republishes the PAC under a different package name.

Its purpose is to provide the peripheral register definitions and interrupt numbering needed by the RISC-V implementation, without retaining Cortex-M-specific interrupt vector infrastructure.

The fork follows these principles:

- Preserve the upstream peripheral register definitions.
- Preserve the `rp_pac` Rust library name for source compatibility.
- Keep RP2350 hardware interrupt numbering.
- Delegate RISC-V trap and interrupt handling to the runtime.
- Avoid unnecessary Cortex-M-specific dependencies and infrastructure.

If upstream eventually publishes a sufficiently recent `rp-pac` release with the required RISC-V compatibility, `hazard3-rp` should migrate back to the upstream crate and this fork may be retired.

All credit goes to the [Embassy project](https://github.com/embassy-rs/rp-pac) and Raspberry Pi Ltd, the authors of the original PAC infrastructure and SVD files.

This project is not affiliated with or endorsed by either organization.