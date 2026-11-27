# hazard3-net-async

> **Published replacement:** `hazard3-net-async` replaces the formerly published `embassy-net-riscv`.
> See the project repository for the publication status of the Hazard3 packages.
> Rust library names and upstream license and attribution notices are retained.

[![Crates.io](https://img.shields.io/crates/v/hazard3-net-async.svg)](https://crates.io/crates/hazard3-net-async)
[![docs.rs](https://docs.rs/hazard3-net-async/badge.svg)](https://docs.rs/hazard3-net-async)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20or%20Apache--2.0-blue.svg)](https://opensource.org/licenses/MIT)

> **Fork of Embassy for RISC-V.** `hazard3-net-async` is an independent copy of [`embassy-net`](https://github.com/embassy-rs/embassy/tree/main/embassy-net)
> taken from the Embassy `main` branch and maintained by [Jorge Andre Castro](https://github.com/jorgeandrecastro) at
> <https://github.com/jorgeandrecastro/Hazard3>.
> The library name is still `embassy_net`; use it with `package = "hazard3-net-async"` in your `Cargo.toml`.

`hazard3-net-async` is a no-std, no-alloc async network stack designed for embedded systems.

It uses the [`hazard3-net`](../hazard3-net/) network stack (0.2), adds convenient async wrappers,
and implements the main loop. `hazard3-net` and [`hazard3-net-driver`](../hazard3-net/hazard3-net-driver/)
are copies of upstream Xarxa pinned to revision `8f6f33b131deb9ff3290e0e01079537e1ed7b2ad`,
prepared under new package names so that the crates.io release of `hazard3-net-async` has no
Git dependencies. The Rust library names remain `xarxa` and `xarxa_driver`.

## Table of contents

- [Getting started](#getting-started)
- [The Hazard3 packages](#the-hazard3-packages)
- [Vision of this fork](#vision-of-this-fork)
- [Features](#features)
- [Source layout](#source-layout)
- [Testing](#testing)
- [Development tools](#development-tools)
- [Hardware support](#hardware-support)
- [Examples](#examples)
- [Adding support for new hardware](#adding-support-for-new-hardware)
- [Interoperability](#interoperability)
- [Acknowledgements](#acknowledgements)

## Getting started

The Rust library is named `embassy_net`, the package is named `hazard3-net-async`:

```toml
[dependencies]
embassy-net = { package = "hazard3-net-async", version = "0.4", features = [
    "tcp",
    "udp",
    "dns",
    "ipv4",
    "medium-ethernet",
] }
```

Pick the features your application needs. See [`Cargo.toml`](Cargo.toml) for the full list.
The network stack underneath ([`hazard3-net`](../hazard3-net/)) also has count features
(`tcp-socket-count-N`, `udp-socket-count-N`, ...) that set how much static RAM it uses.

You also need:

- a network **driver** for your hardware, see [Hardware support](#hardware-support);
- an `embassy-time` driver, see [Interoperability](#interoperability);
- Rust **1.95** or newer, which is the minimum version of `hazard3-net` 0.2.

## The Hazard3 packages

Use the Hazard3 packages together (`hazard3-time`, `hazard3-sync`, `hazard3-executor`, ...) to avoid
duplicate or incompatible copies in your build.

| Package | Role |
| --- | --- |
| `hazard3-net-async` | This crate: async sockets and the main loop. |
| [`hazard3-net`](../hazard3-net/) | The network stack (`xarxa`). |
| [`hazard3-net-driver`](../hazard3-net/hazard3-net-driver/) | The `Driver` trait that network chip drivers implement. |
| [`hazard3-cyw43`](https://crates.io/crates/hazard3-cyw43) | WiFi driver for CYW43xx chips, used in the Raspberry Pi Pico W. |

## Vision of this fork

This fork offers an alternative `embassy-net` that is harmonized with the RISC-V support provided
by [`Hazard3`](https://github.com/jorgeandrecastro/Hazard3). It is maintained alongside
the other essential Hazard3 crates so that RISC-V projects can use a coherent set of packages, but
it is not limited to a single upstream or downstream use case. Its goals are deliberately limited to:

- **More tests.** Upstream has very few unit tests. This fork adds them module by module: error
  mapping, data structures, and compile-time checks of the public API. Runtime tests of the sockets
  against a mock driver are the next step. The test command below is the maintained host-side
  baseline; examples and hardware integrations are not considered tested by this fork unless that
  is explicitly stated.
- **More modularity.** Large source files are split into small, focused modules, which makes the
  code easier to read, review and test. So far:
  - the old `tcp.rs` is now a root module plus `socket`, `socket/io`, `listener` and `client`;
  - the old `udp.rs` is now a root module plus `error`, `socket`, `recv` and `send`.
- **Robustness.** Panicking placeholders are replaced by proper errors where the public API allows
  it (for example, `DnsClient::get_host_by_address` returns `Error::Unsupported` instead of
  calling `todo!()`).
- **Useful examples.** This fork may add or improve focused examples, but example coverage is not
  intended to replace upstream functionality or to imply that every upstream example builds.

The public API stays compatible with upstream `embassy-net`: refactors move code around, they do not
rename or remove public items. The only deviations are small additions, such as the `Unsupported`
variant of `dns::Error` in 0.3.0, which matters only to code that matches exhaustively on that enum.
Every notable change is listed in [`CHANGELOG.md`](CHANGELOG.md).

## Features

- IPv4, IPv6
- Ethernet, IP and IEEE 802.15.4 / 6LoWPAN mediums.
- TCP, UDP, raw sockets, DNS, DHCPv4
- TCP sockets implement the `embedded-io` async traits.
- Multicast
- Multiple interface support
- Timestamping sent and received packets for e.g PTP

See the [`hazard3-net`](../hazard3-net/) README for a detailed list of implemented and
unimplemented features of the network protocols.

This crate focuses on the network/transport layer protocols up to TCP. Higher-level application
layer protocols live in other crates built on top of `embassy-net`:

- [`edge-net`](https://crates.io/crates/edge-net)
- [`sntpc`](https://crates.io/crates/sntpc)
- PTP implementation coming soon

## Source layout

```text
hazard3-net-async/
├── Cargo.toml
├── CHANGELOG.md
├── README.md
└── src/
    ├── lib.rs              # Stack, shared helpers (TryError, wake helpers), main loop
    ├── fmt.rs              # logging / unwrap! macros (log or defmt)
    ├── time.rs             # embassy-time <-> xarxa time conversions
    ├── route.rs            # routing
    ├── neighbor.rs         # neighbor cache
    ├── raw.rs              # raw sockets
    ├── dns.rs              # DNS errors and embedded-nal-async DNS client (with unit tests)
    ├── iface/
    │   ├── mod.rs          # interfaces
    │   ├── dhcpv4.rs       # DHCPv4 client
    │   └── dhcpv4_server.rs# DHCPv4 server
    ├── tcp.rs              # TCP root: error types, shared TcpIo helper, re-exports
    ├── tcp/
    │   ├── socket.rs       # TcpSocket: new, connect/accept, options, state
    │   ├── socket/
    │   │   └── io.rs       # read/write/peek/flush, TcpReader, TcpWriter, embedded-io impls
    │   ├── listener.rs     # TcpListener            (feature `tcp-listener`)
    │   └── client.rs       # embedded-nal-async client (feature `embedded-nal`)
    ├── udp.rs              # UDP root: module docs, re-exports, UdpSocket struct, Drop
    └── udp/
        ├── error.rs        # BindError, SendError, RecvError, Display/Error impls, TryError mapping
        ├── socket.rs       # UdpSocket: new, bind, iface binding, addresses, state, options
        ├── recv.rs         # recv / peek / readiness (async, try_, poll_, zero-copy)
        └── send.rs         # send_to / send_packet_to (async, try_, poll_, in-place)
```

Unit tests live next to the code they test, in a `#[cfg(test)] mod tests` at the end of each file.

## Testing

The unit tests run on the host, not on the embedded target:

```sh
cargo test --lib --features "tcp,tcp-listener,embedded-nal,iface-bind,ipv4,medium-ethernet"

cargo test --features "dns,embedded-nal,ipv4,udp,medium-ethernet"
```

Depending on your setup, linking may need a host `embassy-time` driver (for example a mock driver).

Current coverage:

| Module | Kind of test |
| --- | --- |
| `dns.rs` | `GetQueryResultError`/`StartQueryError` to `dns::Error` mapping, `Display`, `Error` trait |
| `tcp.rs` | `Display`, `ErrorKind` mapping, `Copy`/`Eq`/`Error` traits of the error types |
| `tcp/client.rs` | runtime tests of the buffer pool used by `TcpClient` |
| `tcp/socket.rs`, `tcp/socket/io.rs`, `tcp/listener.rs` | compile-time checks of the public API and `embedded-io` impls |
| `udp/error.rs` | `Display` output and `udp::SendError` to `TryError<SendError>` mapping |

## Development tools

This repository has no automated CI. No build matrix runs the examples, target-specific
configurations, upstream Embassy HAL integrations or hardware integrations, and their build status
must not be assumed. The host-side checks below are the maintained baseline. Additional
dependencies or fixes may be required outside them.

### Compilation

```sh
cargo check --features "dns,embedded-nal,ipv4,udp,medium-ethernet"
```

With warnings promoted to errors:

```sh
RUSTFLAGS="-D warnings" cargo check --features "dns,embedded-nal,ipv4,udp,medium-ethernet"
```

The DNS client has feature-dependent code paths (`ipv4`, `ipv6`). When you change `dns.rs`, also check
each of them on its own:

```sh
cargo check --no-default-features --features "dns,embedded-nal,ipv4,medium-ethernet"
cargo check --no-default-features --features "dns,embedded-nal,ipv6,medium-ethernet"
```

### Documentation

The upstream `embassy-net` documentation produced warnings. This fork fixes the documentation
warnings in the maintained source and verifies the generated API documentation with:

```sh
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --features "dns,embedded-nal,ipv4,udp,medium-ethernet"
```

## Hardware support

- [`esp-radio`](https://crates.io/crates/esp-radio) for WiFi support ESP32 chips. Maintained by Espressif.
- [`hazard3-cyw43`](https://crates.io/crates/hazard3-cyw43) for WiFi on CYW43xx chips, used in the Raspberry Pi Pico W
- [`embassy-usb`](https://github.com/embassy-rs/embassy/tree/main/embassy-usb) for Ethernet-over-USB (CDC NCM) support.
- [`embassy-stm32`](https://github.com/embassy-rs/embassy/tree/main/embassy-stm32) for the builtin Ethernet MAC in all STM32 chips (STM32F1, STM32F2, STM32F4, STM32F7, STM32H7, STM32H5).
- [`embassy-net-wiznet`](https://github.com/embassy-rs/embassy/tree/main/embassy-net-wiznet) for Wiznet SPI Ethernet MAC+PHY chips (W5100S, W5500)
- [`embassy-net-enc28j60`](https://github.com/embassy-rs/embassy/tree/main/embassy-net-enc28j60) for the Microchip ENC28J60 SPI Ethernet MAC+PHY chip.
- [`embassy-net-adin1110`](https://github.com/embassy-rs/embassy/tree/main/embassy-net-adin1110) for the Analog Devices ADIN1110 SPI 10BASE-T1L single-pair Ethernet chip.
- [`embassy-net-esp-hosted`](https://github.com/embassy-rs/embassy/tree/main/embassy-net-esp-hosted) for using ESP32 chips with the [`esp-hosted`](https://github.com/espressif/esp-hosted) firmware as WiFi adapters for another non-ESP32 MCU.
- [`embassy-net-nrf91`](https://github.com/embassy-rs/embassy/tree/main/embassy-net-nrf91) for the cellular modem in Nordic nRF91-series chips.
- [`embassy-net-ppp`](https://github.com/embassy-rs/embassy/tree/main/embassy-net-ppp) for PPP over Serial, useful with cellular modems or for a network link to a host computer.
- [`embassy-nrf`](https://github.com/embassy-rs/embassy/tree/main/embassy-nrf) for IEEE 802.15.4 support on nrf chips.
- [`embassy-stm32-wpan`](https://github.com/embassy-rs/embassy/tree/main/embassy-stm32-wpan) for IEEE 802.15.4 support on STM32WB chips.
- [`embassy-net-tuntap`](https://github.com/embassy-rs/embassy/tree/main/embassy-net-tuntap) for Linux TUN/TAP interfaces, useful for running on `std` platforms.

## Examples

- For usage with Embassy HALs and network chip drivers, search [here](https://github.com/embassy-rs/embassy/tree/main/examples) for `eth` or `wifi`.
- The [`esp-hal` repo](https://github.com/esp-rs/esp-hal) has examples for use on bare-metal ESP32 chips.
- For usage on `std` platforms, see [the `std` examples](https://github.com/embassy-rs/embassy/tree/main/examples/std/src/bin)

## Adding support for new hardware

To add `embassy-net` support for new hardware (i.e. a new Ethernet or WiFi chip, or
an Ethernet/WiFi MCU peripheral), you have to implement the
[`hazard3-net-driver`](../hazard3-net/hazard3-net-driver/) `Driver` trait (library name
`xarxa_driver`). Enable its `async` feature and implement `Driver::register_waker`: this crate
needs it to sleep until the driver has something new.

Drivers should depend only on `hazard3-net-driver`, never on the main `hazard3-net-async` crate.
This allows existing drivers to keep working with newer major versions of this crate, without an
update, as long as the driver trait has no breaking changes.

## Interoperability

This crate can run on any executor.

[`hazard3-time`](https://crates.io/crates/hazard3-time) is used for timekeeping and timeouts. You must
link an `hazard3-time` driver in your project to use this crate.

## Acknowledgements

This crate is a derivative work of the [Embassy](https://embassy.dev) project ([embassy-rs/embassy](https://github.com/embassy-rs/embassy)),
Copyright (c) Embassy project contributors, used under the MIT OR Apache-2.0 licenses. The original copyright and license notices are
kept (see `LICENSE-MIT` and `LICENSE-APACHE`). Embassy and its authors are not affiliated with, and do not endorse, this fork.
Thank you to the Embassy team and contributors for their work.