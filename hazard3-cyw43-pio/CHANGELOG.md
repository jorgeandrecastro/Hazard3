# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.5.0 - 2026-11-28

### Breaking changes

- **Breaking:** rename the Rust library `cyw43_pio` to `hazard3_cyw43_pio`; update imports and macro paths.
- Replace `embassy-futures` with `hazard3-futures` 0.1; use `hazard3-cyw43` 0.5 and `hazard3-rp` 0.4.
- Use matching new Hazard3 dependency versions where applicable.

### Other changes

- Repository renamed to [`jorgeandrecastro/Hazard3`](https://github.com/jorgeandrecastro/Hazard3).
- Former `*-riscv` releases remain preserved on crates.io but are yanked and are not recommended for new dependency resolution.

- Rename package `cyw43-pio-riscv` to `hazard3-cyw43-pio`.

## 0.2.1 - 2026-11-22

- Updated package alignment to `cyw43-riscv` 0.2.1.
- Added links to the Pico 2 W / ESP32-C3 ESP-NOW example and benchmark in the
  README, so the PIO transport package documents the hardware use case without
  embedding example binaries or benchmark artifacts in the crate binary.

## 0.2.0 - 2026-11-22

- Fork as `cyw43-pio-riscv`, retaining the `cyw43_pio` library name and using `embassy-rp-riscv` and `cyw43-riscv`.
- Updated dependency alignment for the RISC-V fork:
  - `cyw43-riscv` 0.2.0 for CYW43 and ESP-NOW action-frame support.
  - `embassy-rp-riscv` 0.2.0 for RP2350 RISC-V support.
  - `embassy-futures` 0.1.2 for the PIO transport implementation.
  - `embassy-net-riscv` 0.3.0 for the compatibility test dependency.
- Added README documentation for ESP-NOW hardware validation between the
  Pico 2 W/CYW43439 and an ESP32-C3 Mini.
- Fix deterministic single-bit corruption on RP2350 backplane reads by using two DMA channels (TX + RX). The single single-channel pattern raced with the PIO state machine in debug builds, causing the LSB of the last sampled word in `cmd_read` to be cleared.
- Breaking: `PioSpi::new` now takes two `dma::Channel` arguments (TX and RX) instead of one.
- Breaking: removed the `fixed` dependency. `PioSpi::new` and the `*_CLOCK_DIVIDER` constants use a `u32` clock divider with 16 integer and 8 fractional bits (the divider multiplied by 256).

The entries below describe the upstream `cyw43-pio` releases.

## 0.10.0 - 2026-03-10

- Updated to use new DMA `Channel` driver struct from embassy-rp
- Update cyw43 0.7.0
- Update embassy-rp 0.10.0

## 0.9.0 - 2025-11-27

- Select pio program based on core clock speed #4792

## 0.8.0 - 2025-08-28

- Bump cyw43 version

## 0.7.0 - 2025-08-26

## 0.6.0 - 2025-08-04

## 0.5.1 - 2025-07-16

## 0.5.0 - 2025-07-15

- Update embassy-rp to 0.5.0

## 0.3.0 - 2025-01-05

- Update embassy-time to 0.4.0
- Update cyw43 to 0.3.0
- Update embassy-rp to 0.3.0

## 0.2.0 - 2024-08-05

- Update to cyw43 0.2.0
- Update to embassy-rp 0.2.0

## 0.1.0 - 2024-01-11

- First release
