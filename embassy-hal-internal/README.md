# embassy-hal-internal

> **RISC-V fork of Embassy.** This is an independent fork of [Embassy](https://github.com/embassy-rs/embassy) maintained by
> [Jorge Andre Castro](https://github.com/jorgeandrecastro) at <https://github.com/jorgeandrecastro/embassy-riscv>.
> It adds RISC-V support (e.g. RP2350 Hazard3 cores) which is not provided upstream. All credit for Embassy goes to the
> Embassy team and contributors. Licensed under the same terms as upstream (MIT OR Apache-2.0). Not affiliated with or endorsed by the Embassy project.

An [Embassy](https://embassy.dev) project.

Internal implementation details for Embassy HALs. DO NOT USE DIRECTLY. Embassy HALs (`embassy-nrf`, `embassy-stm32`, `embassy-rp`) already reexport
everything you need to use them effectively.
