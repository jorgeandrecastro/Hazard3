# xarxa-riscv

[![docs.rs](https://docs.rs/xarxa-riscv/badge.svg)](https://docs.rs/xarxa-riscv)
[![crates.io](https://img.shields.io/crates/v/xarxa-riscv.svg)](https://crates.io/crates/xarxa-riscv)
[![License](https://img.shields.io/badge/license-0BSD-blue.svg)](LICENSE-0BSD.txt)

> **RISC-V Embassy fork.** `xarxa-riscv` is a fork of the original [Xarxa](https://github.com/embassy-rs/xarxa), based on upstream commit `8f6f33b131deb9ff3290e0e01079537e1ed7b2ad`, maintained for integration into the [`embassy-riscv`](https://github.com/jorgeandrecastro/embassy-riscv) ecosystem.
>
> The package is published as `xarxa-riscv`. The Rust crate name remains `xarxa`; use `package = "xarxa-riscv"` in your `Cargo.toml`.

`xarxa-riscv` is an embedded network stack derived from Xarxa, designed for embedded and real-time systems. It supports `no_std` environments and does not require `alloc`.

## Design Goals

The original Xarxa design follows these priorities, in decreasing order of importance:

- **No `unsafe`:** The network stack processes externally supplied packets and performs complex parsing and manipulation. Avoiding unsafe Rust helps reduce the risk of memory-safety vulnerabilities.
- **Suitable for small embedded systems:** Low RAM consumption and small code size.
- **High performance:** Efficient packet processing and memory management.
- **Scalability:** Support for larger systems with faster network links, more sockets and larger buffers.

The RISC-V fork aims to preserve these design principles while maintaining compatibility with the Embassy RISC-V ecosystem.

## Xarxa vs. smoltcp

Xarxa is a rewrite and refactoring of [smoltcp](https://github.com/smoltcp-rs/smoltcp), addressing several architectural limitations identified during its development.

It is largely a port of smoltcp to a different internal design. Many protocol implementations remain mostly unchanged, such as TCP and wire-level packet handling. Other components, particularly the main network stack, have a different architecture while preserving the original protocol logic wherever possible.

Two major design decisions distinguish Xarxa from smoltcp.

### 1. Memory Management

smoltcp manages network buffers using the following model:

- The device implementation owns a buffer pool containing received and transmitted Ethernet frames.
- The device provides borrowed buffers to the network stack through `TxToken` and `RxToken`.
- Each socket type (TCP, UDP, ICMP and raw) owns its own RX and TX ring buffers.
- The network stack copies data between device buffers and socket buffers.

This architecture has several consequences:

- Zero-copy packet processing is not possible because data must be copied between device and socket buffers.
- Multi-interface support is difficult to implement. The traditional smoltcp architecture is centered around a single interface.
- `TxToken` and `RxToken` make the `Device` trait incompatible with dynamic dispatch.
- Sending a reply through a different interface is difficult because the receive-side token is tied to the original device.
- Memory consumption can become significant.
- Each UDP or raw socket requires at least two MTUs of buffer memory. With a 1500-byte MTU, this represents approximately 3 KB per socket.
- DNS and DHCP therefore use dedicated socket types instead of being implemented on top of UDP and raw sockets.
- Multi-interface configurations require separate buffer pools for each interface.

Xarxa instead uses a **packet pool**.

It transfers owned packet handles between components of the network stack. Devices, interfaces, sockets and other components can own packets without relying on the same borrowed-buffer model.

This architecture enables:

- Multiple network interfaces without dedicated packet pools for each interface.
- Zero-copy packet processing: a received packet can be written into a buffer, dispatched through the stack, queued in a socket and delivered to the application without unnecessary copies.
- More flexible packet ownership and routing.
- Avoidance of structural limitations such as the [smoltcp TX queue clog issue](https://github.com/smoltcp-rs/smoltcp/issues/594), which can occur when sending to multiple IP addresses from a single socket.

### 2. Repr Structures

smoltcp uses plain Rust structures such as `IpRepr` and `UdpRepr` to represent packets.

At ingress, packet bytes are parsed and deserialized into these structures. At egress, the structures are serialized back into bytes. The network stack operates primarily on these representations.

Xarxa instead operates directly on packet bytes.

This design has several advantages:

- **Performance:** Avoids unnecessary serialization and deserialization operations.
- **Code size:** Reduces intermediate representations and associated processing.
- **Raw packet preservation:** Raw sockets can preserve fields from IP headers, including options and fields that the stack does not explicitly interpret.

In smoltcp, raw sockets may lose certain IP header fields because packets are deserialized and reserialized. The representation structures intentionally omit fields that the stack does not use. Xarxa's direct packet-byte approach avoids this limitation.

## Benchmarks

The original Xarxa project reports better throughput and smaller code size compared with smoltcp and lwIP.

The upstream benchmark results are available here:

![Throughput benchmark](https://raw.githubusercontent.com/embassy-rs/xarxa-bench/refs/heads/main/bench-throughput.svg)

![Code size benchmark](https://raw.githubusercontent.com/embassy-rs/xarxa-bench/refs/heads/main/bench-codesize.svg)

Benchmark source code: [embassy-rs/xarxa-bench](https://github.com/embassy-rs/xarxa-bench).

*These are upstream Xarxa benchmarks, not independent measurements of the RISC-V fork.*

## Features

The following capabilities describe the upstream Xarxa implementation on which this fork is based. Actual availability depends on the selected Cargo features and target compatibility.

### Network Interfaces

- Multiple network interface support.
- Dynamically add and remove interfaces from the network stack.
- Independent configuration for each interface, including IP addresses.
- Route table for selecting the outgoing interface.
- Support for mixing different network media.
- Hardware address and link-state reporting from device drivers.

### Ethernet

Feature: `medium-ethernet`

- IPv4 ARP.
- IPv6 NDISC.
- Neighbor cache with expiration and renewal on use.
- Egress packet buffering while neighbor resolution is pending.
- Avoids socket queue congestion caused by unreachable neighbors.

### Pure IP

Feature: `medium-ip`

- Pure IP interface medium.

### IEEE 802.15.4

Feature: `medium-ieee802154`

- IEEE 802.15.4 interface support.
- 6LoWPAN header compression.
- IPHC compression for IPv6 headers.
- NHC compression for UDP and extension headers.
- In-place compression and decompression in packet buffers.
- Address contexts for decompression.
- NDISC over IEEE 802.15.4.
- Link-local addresses derived from extended addresses.

Additional features:

- `sixlowpan-fragmentation`: 6LoWPAN fragmentation.
- `sixlowpan-reassembly`: 6LoWPAN reassembly.

### IPv4

Feature: `ipv4`

- IPv4 protocol support.
- DHCP client: `dhcpv4`.
- Raw DHCP lease option access by option number: `dhcpv4-options`.
- Hostname transmission to the DHCP server: `hostname`.
- DHCP server: `dhcpv4-server`.
- Configurable address pools, gateways and DNS options.
- API to inspect and remove DHCP leases.
- IPv4 fragmentation: `ipv4-fragmentation`.
- IPv4 reassembly: `ipv4-reassembly`.

### IPv6

Feature: `ipv6`

- IPv6 protocol support.
- Automatic link-local address generation from the MAC address using EUI-64.
- SLAAC: automatic address and default-route configuration from router advertisements.
- Address and route lifetime management: `slaac`.

### ICMP

- Automatic ping replies: `icmp-ping-reply`.
- Routing incoming ICMP errors to the socket responsible for the originating packet: `icmp-errors`.

### UDP

Feature: `udp`

- UDP socket implementation.
- Zero-copy transmission and reception.
- Connected and unconnected sockets.
- Support for receiving packets from any remote IP address and port, or from a fixed remote endpoint.

### Raw Sockets

- Zero-copy transmission and reception.
- Ethernet-layer raw sockets: `raw-ethernet`.
- IP-layer raw sockets: `raw-ip`.
- Ethernet raw sockets transmit and receive complete Ethernet frames without IP routing.
- IP raw sockets transmit and receive raw IP packets while benefiting from network-stack routing.
- IP headers are byte-copied rather than parsed and re-emitted, preserving fields and options unsupported by the stack.

### TCP

Feature: `tcp`

- Full TCP implementation.
- TCP listeners with an accept queue: `tcp-listener`.
- Buffers are allocated only when a connection is accepted.
- Window scaling.
- Configurable keepalive.
- RTT estimation and automatic retransmission timeout adjustment.
- Compile-time selection of CUBIC, Reno or no congestion control.
- Nagle's algorithm enabled by default, with the option to disable it.
- Delayed ACK enabled by default, with the option to disable it.
- Zero-window probes.
- TCP timestamps: `tcp-timestamps`.
- TCP SACK transmission: `tcp-sack`.

### DNS

Feature: `dns`

- DNS client.
- Multiple DNS server support.
- Retransmission with exponential backoff.
- Multicast DNS for `.local` names: `mdns`.

### Multicast

Feature: `multicast`

- Join and leave multicast groups independently on each interface.
- IGMPv1 and IGMPv2 for IPv4.
- MLDv2 for IPv6.
- Membership reports on join and leave.
- Responses to multicast router queries.
- Automatic joining of IPv6 solicited-node multicast groups.
- Reporting multicast hardware addresses to drivers that support hardware-level filtering.

### Interface Binding

Feature: `iface-bind`

- Bind sockets to a specific network interface.
- Similar functionality to Linux `SO_BINDTODEVICE`.

### Packet Metadata

- Hardware timestamping support for RX and TX: `packetmeta-timestamp`.
- Opaque packet identifiers for correlating packets through the network stack: `packetmeta-id`.

### Checksum Offload

- Interfaces can report which checksums they can validate or calculate.
- The network stack skips checksum operations that are already handled by the hardware.

## Not Yet Implemented

The following items were identified as not yet implemented in the upstream project at the referenced revision. Their status in this fork should be checked independently.

- Acting on link state: skipping down interfaces during egress routing and restarting DHCP/SLAAC after link-up.
- IPv6 Duplicate Address Detection (DAD).
- IPv6 RDNSS: DNS server configuration through router advertisements.
- An equivalent to smoltcp's `any_ip`.
- IPv6 fragmentation and reassembly.
- TCP segmentation offload.
- TCP SACK reception and processing of ranges received from peers.
- Hash-map-based socket storage to achieve O(1) packet dispatch instead of O(n). This is primarily useful for systems with thousands of sockets and may be optional behind a Cargo feature.
- Possible multithreading support, requiring per-socket synchronization primitives and potentially `std`.

## RISC-V Fork

`xarxa-riscv` maintains the original Xarxa network-stack architecture while providing a dedicated package for the RISC-V-oriented Embassy ecosystem.

The fork is maintained separately from upstream to allow target-specific compatibility work and integration without requiring modifications to the original project.

Repository: [jorgeandrecastro/embassy-riscv](https://github.com/jorgeandrecastro/embassy-riscv)

## License

`xarxa-riscv` is distributed under the terms of the **0-Clause BSD License (0BSD)**.

See [LICENSE-0BSD.txt](LICENSE-0BSD.txt) for the full license text.

## Acknowledgements

This project is based on [Xarxa](https://github.com/embassy-rs/xarxa), originally developed by the Embassy contributors.

The original implementation and architectural work are credited to their respective authors and contributors.