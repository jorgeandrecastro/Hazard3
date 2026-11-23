# hazard3-time

> **Migration in progress:** `hazard3-time` replaces the formerly published `embassy-time-riscv`.
> See the project repository for the publication status of the Hazard3 packages.
> Rust library names and upstream license and attribution notices are retained.

[![Crates.io](https://img.shields.io/crates/v/hazard3-time.svg)](https://crates.io/crates/hazard3-time)
[![docs.rs](https://docs.rs/hazard3-time/badge.svg)](https://docs.rs/hazard3-time)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20or%20Apache--2.0-blue.svg)](https://opensource.org/licenses/MIT)

> **Fork of Embassy for RISC-V.** `hazard3-time` is an independent, **unmodified copy** of [`embassy-time`](https://github.com/embassy-rs/embassy/tree/main/embassy-time)
> taken from the Embassy `main` branch, maintained by [Jorge Andre Castro](https://github.com/jorgeandrecastro) at
> <https://github.com/jorgeandrecastro/embassy-riscv>. It was originally republished under another name because [`hazard3-rp`](https://crates.io/crates/hazard3-rp)
> needs the unreleased changes of `main`, and crates.io does not accept git dependencies.
> The library name is still `embassy_time`; use it with `package = "hazard3-time"` in your `Cargo.toml`.
> **Use the `hazard3-*` versions of the forked crates together** (`hazard3-time`, `hazard3-executor`, `hazard3-sync`, ...) to avoid duplicate/incompatible copies in your build.

Timekeeping, delays and timeouts.

Timekeeping is done with elapsed time since system boot. Time is represented in
ticks, where the tick rate is defined either by the driver (in the case of a fixed-rate
tick) or chosen by the user with a [tick rate](#tick-rate) feature. The chosen
tick rate applies to everything in `embassy-time` and thus determines the maximum
timing resolution of <code>(1 / tick_rate) seconds</code>.

Tick counts are 64 bits. The default tick rate of 1Mhz supports
representing time spans of up to ~584558 years, which is big enough for all practical
purposes and allows not having to worry about overflows.

## Global time driver

The `embassy-time` crate is backed by a global "time driver" specified at build time.
Only one driver can be active in a program.

All methods and structs transparently call into the active driver. This makes it
possible for libraries to use `embassy_time` in a driver-agnostic way without
requiring generic parameters.

For more details, check the [`embassy_time_driver`](https://crates.io/crates/embassy-time-driver) crate.

## Instants and Durations

[`Instant`] represents a given instant of time (relative to system boot), and [`Duration`]
represents the duration of a span of time. They implement the math operations you'd expect,
like addition and subtraction.

## Delays and timeouts

[`Timer`] allows performing async delays. [`Ticker`] allows periodic delays without drifting over time.

An implementation of the `embedded-hal` delay traits is provided by [`Delay`], for compatibility
with libraries from the ecosystem.

## Wall-clock time

The `embassy-time` crate deals exclusively with a monotonically increasing tick count.
Therefore it has no direct support for wall-clock time ("real life" datetimes
like `2021-08-24 13:33:21`).

If persistence across reboots is not needed, support can be built on top of
`embassy_time` by storing the offset between "seconds elapsed since boot"
and "seconds since unix epoch".

## Acknowledgements

This crate is a derivative work of the [Embassy](https://embassy.dev) project ([embassy-rs/embassy](https://github.com/embassy-rs/embassy)),
Copyright (c) Embassy project contributors, used under the MIT OR Apache-2.0 licenses. The original copyright and license notices are
kept (see `LICENSE-MIT` and `LICENSE-APACHE`). Embassy and its authors are not affiliated with, and do not endorse, this fork.
Thank you to the Embassy team and contributors for their work.
