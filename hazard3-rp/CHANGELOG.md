# Changelog for hazard3-rp

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),  
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## 0.4.0 - 2026-11-28

### Breaking changes

- **Breaking:** rename the Rust library `embassy_rp` to `hazard3_rp`; update imports and macro paths.
- Replace `embassy-futures` with `hazard3-futures` 0.1 and update `hazard3-embedded-hal` to 0.3.
- Use matching new Hazard3 dependency versions where applicable.

### Other changes

- Repository renamed to [`jorgeandrecastro/Hazard3`](https://github.com/jorgeandrecastro/Hazard3).
- Former `*-riscv` releases remain preserved on crates.io but are yanked and are not recommended for new dependency resolution.

- Rename package `embassy-rp-riscv` to `hazard3-rp`.
- Include the interrupt fallback linker script required to link RISC-V applications.

## 0.2.0 - 2026-11-21

**RISC-V only: all Cortex-M and RP2040 support has been removed.**

### Removed

- Remove the Cortex-M and RP2040 features and dependencies.
- Remove the Cortex-M CPU primitives and startup implementation.
- Remove the obsolete `SIO_IRQ_PROC1` interrupt reference from the multicore implementation.

### Changed

- Enable the `riscv` feature by default.
- Reject RP2040 and non-RISC-V targets at compile time.
- Depend on `rp-pac-riscv` 0.2, which no longer provides the Cortex-M interrupt vector table (`_vectors.rs`, `__INTERRUPTS`, `Vector`). RISC-V trap and interrupt handling is delegated to the RISC-V runtime.
- Update the RP235x multicore implementation to use the RISC-V `SIO_IRQ_FIFO` interrupt instead of the unavailable `SIO_IRQ_PROC1` symbol.
- Preserve the inter-core FIFO mechanism used by `spawn_core1`, `pause_core1` and `resume_core1`.

### Validation

- Successfully compile for RP2350A (`rp235xa`) on `riscv32imac-unknown-none-elf`.
- Successfully compile for RP2350B (`rp235xb`) on `riscv32imac-unknown-none-elf`.
- Validate both configurations with the `riscv`, `rt` and `time-driver` features enabled.

### Development

The following commands are used to validate the crate on the Hazard3 target (`riscv32imac-unknown-none-elf`):

```sh
# RP2350A
cargo check \
    --target riscv32imac-unknown-none-elf \
    --no-default-features \
    --features "rp235xa riscv rt time-driver"

# RP2350B
cargo check \
    --target riscv32imac-unknown-none-elf \
    --no-default-features \
    --features "rp235xb riscv rt time-driver"

# RP2350A, release profile
cargo check --release \
    --target riscv32imac-unknown-none-elf \
    --no-default-features \
    --features "rp235xa riscv rt time-driver"
```

## 0.1.1 - 2026-11-18

- Depend on `embassy-time-driver-riscv` 0.1.1 (which no longer sets the `links = "embassy-time"` manifest key).

## 0.1.0 - 2026-11-18

- First release of the `embassy-rp-riscv` fork, based on upstream `embassy-rp` 0.10.0 (changelog of upstream versions omitted).
- Add RISC-V (RP2350 Hazard3) support behind the `riscv` feature.
- Remove the `embassy-crypto-rng` feature.