# hazard3-net

[![docs.rs](https://docs.rs/hazard3-net/badge.svg)](https://docs.rs/hazard3-net)
[![crates.io](https://img.shields.io/crates/v/hazard3-net.svg)](https://crates.io/crates/hazard3-net)
[![License](https://img.shields.io/badge/license-0BSD-blue.svg)](LICENSE-0BSD.txt)

A network stack for embedded and real-time systems, with a packet-pool design,
zero-copy sockets and first-class support for RISC-V.

> **RISC-V Embassy fork.** `hazard3-net` is a fork of [Xarxa](https://github.com/embassy-rs/xarxa),
> based on upstream commit `8f6f33b131deb9ff3290e0e01079537e1ed7b2ad`, maintained for the
> [`Hazard3`](https://github.com/jorgeandrecastro/Hazard3) ecosystem. It replaces the formerly
> published `xarxa-riscv` package.
>
> The package name is `hazard3-net` and the Rust crate name is `hazard3_net`
> (see [Getting started](#getting-started)).

`hazard3-net` supports `no_std` environments and does not need a heap. A heap-backed pool is
available behind the `alloc` feature.

## Migration to 0.4.0

The Rust library is now `hazard3_net` instead of `xarxa`. Update imports and macro paths; this is a source-breaking change. Add the package directly without a `package` alias:

```toml
[dependencies]
hazard3-net = "0.4"
```

Use compatible versions of the other Hazard3 crates together; older releases may use different library names.

## Table of contents

- [Design goals](#design-goals)
- [Getting started](#getting-started)
- [Packet pools](#packet-pools)
- [RISC-V and Hazard3](#risc-v-and-hazard3)
- [Reducing code size and RAM](#reducing-code-size-and-ram)
- [Xarxa vs. smoltcp](#xarxa-vs-smoltcp)
- [Features](#features)
- [Not yet implemented](#not-yet-implemented)
- [Testing](#testing)
- [License](#license)

## Design goals

The original Xarxa design follows these priorities, in decreasing order of importance:

- **Minimal `unsafe`:** the stack parses and rewrites externally supplied packets, so memory
  safety matters. Protocol code is written in safe Rust. The `unsafe` code is confined to the
  packet pools (`src/pool`), which hand out pointers into memory they manage. It is small,
  documented with `SAFETY` comments, and covered by unit tests.
- **Suitable for small embedded systems:** low RAM use and small code size.
- **High performance:** efficient packet processing and memory management.
- **Scalability:** support for faster links, more sockets and larger buffers.

This fork keeps these principles and stays compatible with the Embassy RISC-V ecosystem.

## Getting started

The Rust crate is named `hazard3_net`, and the package is named `hazard3-net`:

```toml
[dependencies]
hazard3-net = { version = "0.4", default-features = false, features = [
    "medium-ethernet",
    "ipv4",
    "udp",
    "tcp",
    "dhcpv4",
    "tcp-reno",
] }
```

Create a stack on top of a packet pool:

```rust
use hazard3_net::{AllocPool, Stack};

// Up to 64 buffers of 1514 bytes, taken from the heap (needs the `alloc` feature).
static POOL: AllocPool<1514> = AllocPool::new(64);

let stack = Stack::new(&POOL, 0x1234_5678);
```

The default features enable almost everything, including `std`. This is convenient on a host
but not what you want on a microcontroller: use `default-features = false` and pick what you
need.

The minimum supported Rust version is **1.95** (edition 2024).

## Packet pools

All packets of the stack live in a pool. Interfaces, sockets and drivers pass owned packet
handles around instead of copying bytes. Pick the pool that fits your system:

| Pool | Memory | Heap | Notes |
|------|--------|------|-------|
| `StaticPool` | A fixed number of buffers, in a `static` | No | Fully deterministic, no `alloc` needed. |
| `AllocPool` | Buffers taken from the heap, up to a limit | Yes (`alloc` feature) | Recycles buffers, see below. |

### `AllocPool` recycling

`AllocPool` keeps the buffers you drop in a free list and reuses them. In steady state,
allocating and dropping a packet never calls the heap allocator and never clears the buffer
storage, so it is fast, takes constant time and does not fragment the heap.

- `max_count` limits the **total** number of buffers taken from the heap (in use plus cached).
- A dropped buffer stays cached. Its heap memory is only released by `AllocPool::trim()`.
- The free list is protected by a [`critical-section`](https://crates.io/crates/critical-section).
  On RISC-V and Cortex-M this takes a couple of instructions, with no retry loop, so the
  latency is bounded. Your firmware **must provide a `critical-section` implementation**; the
  HAL of your chip usually does.

## RISC-V and Hazard3

- **Target.** The RP2350 Hazard3 cores use `riscv32imac-unknown-none-elf`.
- **Atomics.** On targets with atomic read-modify-write (the `A` extension), the pools use
  atomics. On targets without it (for example `riscv32imc` or `thumbv6m`), they use a critical
  section instead.
- **`critical-section` on a multi-core chip.** The RP2350 has two cores. Use the
  implementation of your HAL, which must also protect against the other core. Do **not** use
  `critical-section-single-hart` on a multi-core chip.
- **Congestion control.** Avoid `tcp-cubic` on cores without a floating-point unit, such as
  Hazard3: it uses `f64` and pulls in slow soft-float code. Use `tcp-reno`, or none.

## Reducing code size and RAM

This crate does not define build profiles: Cargo only reads them from the `Cargo.toml` at the
root of the project being built, so the profiles of your firmware crate are the ones that count.
Add these settings to the `Cargo.toml` of your firmware:

```toml
[profile.release]
opt-level = "s"      # or "z" for the smallest binary, 3 for the fastest code
lto = "fat"
codegen-units = 1
panic = "abort"
debug = false
```

Then size the stack for your application with the count features, for example
`tcp-socket-count-2`, `udp-socket-count-1`, `neighbor-cache-count-4` or
`dhcp-options-buf-size-64`. Each of them lowers static RAM. The full list is in
[`Cargo.toml`](Cargo.toml) and on [docs.rs](https://docs.rs/hazard3-net).

Measure before and after a change, for example with `cargo bloat --release` and `cargo size`.

## Xarxa vs. smoltcp

Xarxa is a rewrite and refactoring of [smoltcp](https://github.com/smoltcp-rs/smoltcp),
addressing several architectural limitations identified during its development.

It is largely a port of smoltcp to a different internal design. Many protocol implementations
stay mostly unchanged, such as TCP and wire-level packet handling. Other components,
particularly the main network stack, have a different architecture while preserving the
original protocol logic wherever possible.

Two major design decisions distinguish Xarxa from smoltcp.

### 1. Memory management

smoltcp manages network buffers using the following model:

- The device implementation owns a buffer pool containing received and transmitted Ethernet
  frames.
- The device provides borrowed buffers to the network stack through `TxToken` and `RxToken`.
- Each socket type (TCP, UDP, ICMP and raw) owns its own RX and TX ring buffers.
- The network stack copies data between device buffers and socket buffers.

This architecture has several consequences:

- Zero-copy packet processing is not possible, because data must be copied between device and
  socket buffers.
- Multi-interface support is difficult to implement. The traditional smoltcp architecture is
  centered around a single interface.
- `TxToken` and `RxToken` make the `Device` trait incompatible with dynamic dispatch.
- Sending a reply through a different interface is difficult, because the receive-side token is
  tied to the original device.
- Memory consumption can become significant. Each UDP or raw socket requires at least two MTUs
  of buffer memory, about 3 KB per socket with a 1500-byte MTU.
- DNS and DHCP therefore use dedicated socket types instead of being built on UDP and raw
  sockets.
- Multi-interface configurations require separate buffer pools for each interface.

Xarxa instead uses a **packet pool**. It transfers owned packet handles between components of
the network stack. Devices, interfaces, sockets and other components can own packets without
relying on the same borrowed-buffer model.

This enables:

- Multiple network interfaces without a dedicated packet pool for each one.
- Zero-copy packet processing: a received packet can be written into a buffer, dispatched
  through the stack, queued in a socket and delivered to the application without unnecessary
  copies.
- More flexible packet ownership and routing.
- Avoidance of structural limitations such as the
  [smoltcp TX queue clog issue](https://github.com/smoltcp-rs/smoltcp/issues/594), which can
  occur when sending to multiple IP addresses from a single socket.

### 2. Repr structures

smoltcp uses plain Rust structures such as `IpRepr` and `UdpRepr` to represent packets. At
ingress, packet bytes are parsed into these structures. At egress, the structures are
serialized back into bytes. The network stack operates primarily on these representations.

Xarxa instead operates directly on packet bytes. This has several advantages:

- **Performance:** avoids unnecessary serialization and deserialization.
- **Code size:** reduces intermediate representations and the processing around them.
- **Raw packet preservation:** raw sockets keep fields from IP headers, including options and
  fields the stack does not interpret. In smoltcp, raw sockets can lose such fields, because
  packets are deserialized and reserialized through structures that omit unused fields.

## Features

The capabilities below describe the upstream Xarxa implementation on which this fork is based.
Actual availability depends on the selected Cargo features and on the target.

### Network interfaces

- Multiple network interfaces.
- Dynamically add and remove interfaces.
- Independent configuration per interface, including IP addresses.
- Route table for selecting the outgoing interface.
- Mixing of different network media.
- Hardware address and link-state reporting from device drivers.

### Ethernet

Feature: `medium-ethernet`

- IPv4 ARP.
- IPv6 NDISC.
- Neighbor cache with expiration and renewal on use.
- Egress packet buffering while neighbor resolution is pending.
- No socket queue congestion caused by unreachable neighbors.

### Pure IP

Feature: `medium-ip`

- Pure IP interface medium.

### IEEE 802.15.4

Feature: `medium-ieee802154`

- IEEE 802.15.4 interfaces and 6LoWPAN header compression.
- IPHC compression for IPv6 headers, NHC compression for UDP and extension headers.
- In-place compression and decompression in packet buffers.
- Address contexts for decompression.
- NDISC over IEEE 802.15.4.
- Link-local addresses derived from extended addresses.
- `sixlowpan-fragmentation`: 6LoWPAN fragmentation.
- `sixlowpan-reassembly`: 6LoWPAN reassembly.

### IPv4

Feature: `ipv4`

- DHCP client: `dhcpv4`.
- Raw DHCP lease option access by option number: `dhcpv4-options`.
- Hostname sent to the DHCP server: `hostname`.
- DHCP server: `dhcpv4-server`, with configurable address pools, gateways and DNS options, and
  an API to inspect and remove leases.
- Fragmentation: `ipv4-fragmentation`. Reassembly: `ipv4-reassembly`.

### IPv6

Feature: `ipv6`

- Automatic link-local address generation from the MAC address using EUI-64.
- SLAAC: address and default-route configuration from router advertisements, with address and
  route lifetime management: `slaac`.

### ICMP

- Automatic ping replies: `icmp-ping-reply`.
- Routing of incoming ICMP errors to the socket responsible for the originating packet:
  `icmp-errors`.

### UDP

Feature: `udp`

- Zero-copy transmission and reception.
- Connected and unconnected sockets, receiving from any remote endpoint or from a fixed one.

### Raw sockets

- Zero-copy transmission and reception.
- Ethernet-layer raw sockets (`raw-ethernet`) send and receive complete Ethernet frames,
  without IP routing.
- IP-layer raw sockets (`raw-ip`) send and receive raw IP packets, with the routing of the
  stack.
- IP headers are byte-copied rather than parsed and re-emitted.

### TCP

Feature: `tcp`

- Full TCP implementation, with listeners and an accept queue: `tcp-listener`.
- Socket buffers allocated only when a connection is accepted.
- Window scaling, configurable keepalive, zero-window probes.
- RTT estimation and automatic retransmission timeout adjustment.
- Compile-time choice of CUBIC (`tcp-cubic`), Reno (`tcp-reno`) or no congestion control.
  Enable at most one.
- Nagle's algorithm and delayed ACK, on by default and can be disabled.
- TCP timestamps: `tcp-timestamps`. SACK transmission: `tcp-sack`.

### DNS

Feature: `dns`

- DNS client with multiple servers and exponential-backoff retransmission.
- Multicast DNS for `.local` names: `mdns`.

### Multicast

Feature: `multicast`

- Join and leave groups independently on each interface.
- IGMPv1 and IGMPv2 for IPv4, MLDv2 for IPv6.
- Membership reports on join and leave, and replies to router queries.
- Automatic joining of IPv6 solicited-node groups.
- Multicast hardware addresses reported to drivers with hardware filtering.

### Interface binding

Feature: `iface-bind`

- Bind sockets to a network interface, like Linux `SO_BINDTODEVICE`.

### Packet metadata

- Hardware timestamps for RX and TX: `packetmeta-timestamp`.
- Opaque packet identifiers: `packetmeta-id`.

### Checksum offload

- Interfaces report which checksums they can validate or compute.
- The stack skips the checksum work the hardware already does.

### Logging and serialization

- `log` or `defmt` logging (enable only one), and `packet-log` to log every packet decoded.
- `serde` support for some types.

## Not yet implemented

These items were not implemented in the upstream project at the referenced revision. Check
their status in this fork independently.

- Acting on link state: skipping down interfaces on egress and restarting DHCP/SLAAC after
  link-up.
- IPv6 Duplicate Address Detection (DAD).
- IPv6 RDNSS: DNS servers from router advertisements.
- An equivalent of smoltcp's `any_ip`.
- IPv6 fragmentation and reassembly.
- TCP segmentation offload.
- Processing of SACK ranges received from peers (sending SACK is supported).
- Hash-map-based socket storage for O(1) packet dispatch instead of O(n). Mostly useful with
  thousands of sockets, and could be optional behind a Cargo feature.
- Possible multithreading support, which would need per-socket synchronization and probably
  `std`.

## Testing

```sh
# Everything, with the default features (host)
cargo test --all

# Only the pool tests, with the default features (host)
cargo test --lib pool

# Only the pool tests, with a minimal feature set (host)
cargo test --lib --no-default-features \
    --features "alloc std medium-ip ipv4 udp" pool

# Check that a bare-metal RISC-V build still compiles
rustup target add riscv32imac-unknown-none-elf
cargo build --no-default-features --features "alloc medium-ethernet ipv4 udp" \
    --target riscv32imac-unknown-none-elf
```

The crate refuses to build unless at least one medium (`medium-ethernet`, `medium-ip` or
`medium-ieee802154`) **and** at least one IP version (`ipv4` or `ipv6`) are enabled. With
`--no-default-features`, always add them yourself, as in the commands above.

`AllocPool` has unit tests for exhaustion, recycling, free-list integrity, `trim`, buffer
layout and alignment, and multi-threaded use. They are meant to catch side effects of changes
made elsewhere in the crate. Run them whenever you touch the pools or `PacketBuf`.

## License

`hazard3-net` is distributed under the terms of the **0-Clause BSD License (0BSD)**. See
[LICENSE-0BSD.txt](LICENSE-0BSD.txt) for the full text. Upstream license and attribution
notices are retained.

## Acknowledgements

This project is based on [Xarxa](https://github.com/embassy-rs/xarxa), originally developed by
the Embassy contributors. The original implementation and architectural work are credited to
their respective authors.

Upstream benchmarks (throughput and code size, compared with smoltcp and lwIP) are available in
[embassy-rs/xarxa-bench](https://github.com/embassy-rs/xarxa-bench). They are upstream Xarxa
measurements, not independent measurements of this fork.