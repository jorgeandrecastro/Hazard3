# hazard3-hal-internal

> **Published replacement:** `hazard3-hal-internal` replaces the formerly published `embassy-hal-internal-riscv`.
> See the project repository for the publication status of the Hazard3 packages.
> Rust library names and upstream license and attribution notices are retained.

[![Crates.io](https://img.shields.io/crates/v/hazard3-hal-internal.svg)](https://crates.io/crates/hazard3-hal-internal)
[![docs.rs](https://docs.rs/hazard3-hal-internal/badge.svg)](https://docs.rs/hazard3-hal-internal)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20or%20Apache--2.0-blue.svg)](https://opensource.org/licenses/MIT)

> **RISC-V fork of Embassy.** This is an independent fork of [Embassy](https://github.com/embassy-rs/embassy) maintained by
> [Jorge Andre Castro](https://github.com/jorgeandrecastro) at <https://github.com/jorgeandrecastro/Hazard3>.
> It adds RISC-V support (e.g. RP2350 Hazard3 cores) which is not provided upstream. All credit for Embassy goes to the
> Embassy team and contributors. Licensed under the same terms as upstream (MIT OR Apache-2.0). Not affiliated with or endorsed by the Embassy project.

An [Embassy](https://embassy.dev) project.

Internal implementation details for Embassy HALs. DO NOT USE DIRECTLY. Embassy HALs (`embassy-nrf`, `embassy-stm32`, `embassy-rp`) already reexport
everything you need to use them effectively.

## RISC-V support

This fork adds a `riscv` feature providing interrupt handling for the RP2350 Hazard3 RISC-V cores (`Xh3irq` extension), as a
counterpart of the Cortex-M NVIC code (`cortex-m` feature).

* `cortex-m` and `riscv` are **mutually exclusive** (enabling both is a compile error).
* The `interrupt` module offers the same kind of API on both architectures: `irq_enable`, `irq_disable`, `irq_is_enabled`,
  `irq_is_pending`, `irq_pend`, `irq_unpend`, `irq_get_priority`, `irq_set_priority` and `irq_next`.
* On RISC-V all external interrupts use the machine external interrupt. They are managed through the custom CSRs
  `meiea` (enable), `meipa` (pending), `meifa` (force), `meipra` (priority) and `meinext` (next pending IRQ).
* The `interrupt_mod!` macro (used by the HAL) generates weak default handlers, a dispatch table and the `MachineExternal`
  symbol. It is only generated when the HAL crate enables its `riscv` feature.

Use it through a HAL such as [`hazard3-rp`](https://crates.io/crates/hazard3-rp) rather than directly. The crate is imported as
`embassy_hal_internal` (same library name as upstream), so it can replace upstream in a dependency declaration with `package = "hazard3-hal-internal"`.

To build and flash firmware for the RP2350 RISC-V cores, see [`rp2350-riscv-linker`](https://crates.io/crates/rp2350-riscv-linker)
(version 0.2 or later): zero-config `memory.x`, RP2350 RISC-V image definition, `.cargo/config.toml` and flashing instructions.

## Credits and license

Based on [`embassy-hal-internal`](https://github.com/embassy-rs/embassy/tree/main/embassy-hal-internal) from the Embassy project; all credit
goes to the Embassy team and contributors. Licensed under either of Apache License 2.0 or MIT, at your option, like upstream.

## Acknowledgements

This crate is a derivative work of the [Embassy](https://embassy.dev) project ([embassy-rs/embassy](https://github.com/embassy-rs/embassy)),
Copyright (c) Embassy project contributors, used under the MIT OR Apache-2.0 licenses. The original copyright and license notices are
kept (see `LICENSE-MIT` and `LICENSE-APACHE`). Embassy and its authors are not affiliated with, and do not endorse, this fork.
Thank you to the Embassy team and contributors for their work.
