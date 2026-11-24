# hazard3-sync

> **Published replacement:** `hazard3-sync` replaces the formerly published `embassy-sync-riscv`.
> See the project repository for the publication status of the Hazard3 packages.
> Rust library names and upstream license and attribution notices are retained.

[![Crates.io](https://img.shields.io/crates/v/hazard3-sync.svg)](https://crates.io/crates/hazard3-sync)
[![docs.rs](https://docs.rs/hazard3-sync/badge.svg)](https://docs.rs/hazard3-sync)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20or%20Apache--2.0-blue.svg)](https://opensource.org/licenses/MIT)

> **Fork of Embassy for RISC-V.** `hazard3-sync` is an independent, **unmodified copy** of [`embassy-sync`](https://github.com/embassy-rs/embassy/tree/main/embassy-sync)
> taken from the Embassy `main` branch, maintained by [Jorge Andre Castro](https://github.com/jorgeandrecastro) at
> <https://github.com/jorgeandrecastro/Hazard3>. It was originally republished under another name because [`hazard3-rp`](https://crates.io/crates/hazard3-rp)
> needs the unreleased changes of `main`, and crates.io does not accept git dependencies.
> The library name is still `embassy_sync`; use it with `package = "hazard3-sync"` in your `Cargo.toml`.
> **Use the `hazard3-*` versions of the forked crates together** (`hazard3-time`, `hazard3-executor`, `hazard3-sync`, ...) to avoid duplicate/incompatible copies in your build.

An [Embassy](https://embassy.dev) project.

Synchronization primitives and data structures with async support:

- [`Channel`](channel::Channel) - A Multiple Producer Multiple Consumer (MPMC) channel. Each message is only received by a single consumer.
- [`PriorityChannel`](priority_channel::PriorityChannel) - A Multiple Producer Multiple Consumer (MPMC) channel. Each message is only received by a single consumer. Higher priority items are shifted to the front of the channel.
- [`PubSubChannel`](pubsub::PubSubChannel) - A broadcast channel (publish-subscribe) channel. Each message is received by all consumers.
- [`Signal`](signal::Signal) - Signalling latest value to a single consumer.
- [`Watch`](watch::Watch) - Signalling latest value to multiple consumers.
- [`Mutex`](mutex::Mutex) - Mutex for synchronizing state between asynchronous tasks.
- [`Pipe`](pipe::Pipe) - Byte stream implementing `embedded_io` traits.
- [`WakerRegistration`](waitqueue::WakerRegistration) - Utility to register and wake a `Waker`.
- [`AtomicWaker`](waitqueue::AtomicWaker) - Utility to register and wake a `Waker` from interrupt context.
- [`MultiWakerRegistration`](waitqueue::MultiWakerRegistration) - Utility registering and waking multiple `Waker`'s.
- [`LazyLock`](lazy_lock::LazyLock) - A value which is initialized on the first access

## Interoperability

Futures from this crate can run on any executor.

## Acknowledgements

This crate is a derivative work of the [Embassy](https://embassy.dev) project ([embassy-rs/embassy](https://github.com/embassy-rs/embassy)),
Copyright (c) Embassy project contributors, used under the MIT OR Apache-2.0 licenses. The original copyright and license notices are
kept (see `LICENSE-MIT` and `LICENSE-APACHE`). Embassy and its authors are not affiliated with, and do not endorse, this fork.
Thank you to the Embassy team and contributors for their work.
