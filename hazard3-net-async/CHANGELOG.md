# Changelog for hazard3-net-async

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.5.0 - 2026-11-28

### Added

- `NeighborCache::for_each`: calls a closure for every entry in one pass, under one lock of the
  stack. It is O(n) and sees a consistent view of the cache.
- Unit tests in `src/time.rs` (not present in upstream `embassy-net`): round trips of instants
  in both directions, behaviour past the wrap of the 32-bit millisecond count, saturation of far
  offsets, clamping before the epoch, and duration conversions. They do not read the clock.
- Compile-time checks of the public API in `src/neighbor.rs`: signatures of every public method,
  and the `Copy`/`Eq`/`Debug` guarantees of `Neighbor`, `NeighborState` and `NeighborCache`.

### Changed

- `NeighborCache::iter` is documented as locking the stack once per entry, costing O(n²) for a full
  pass, and possibly skipping or repeating an entry if the cache changes meanwhile. Its behaviour is
  unchanged. Use `for_each` for a consistent pass.
- `src/time.rs`: offsets now saturate at `xarxa::time::Duration::MAX`, as documented, instead of at
  `u32::MAX` milliseconds.
- `src/time.rs`: the conversions between instants are split into private helpers that take `now` as
  a parameter. The public (`pub(crate)`) functions are unchanged, and each now reads the clock once.
- `src/time.rs`: removed the blanket `#![allow(unused)]`. Only `duration_to_xarxa` keeps a targeted
  `#[allow(dead_code)]`, because it is used by the DHCPv4 client and server, the reassembly
  timeout and the TCP socket options, and a build without those features would warn.

### Documentation

- README: the interoperability section refers to `hazard3-time` for timekeeping.
- README: `time.rs` and `neighbor.rs` in the source layout and in the test coverage table, and the
  version in the Getting started example.

### Removed / not provided

- No CI compilation or test suite is provided for this release.
- No examples or hardware integrations are compiled or tested.
- No automated build matrix or target-platform validation is provided.

## 0.4.0 - 2026-11-28

### Changed

- Updated the `hazard3-net` dependency to 0.2.0. See the
  [`hazard3-net` changelog](../hazard3-net/CHANGELOG.md) for what changed in the network stack.
  There are no other source changes in this crate.
- Repository renamed to [`jorgeandrecastro/Hazard3`](https://github.com/jorgeandrecastro/Hazard3).
- Rename package `embassy-net-riscv` to `hazard3-net-async` while retaining the Rust library name.
- Former `*-riscv` releases remain preserved on crates.io but are yanked and are not recommended
  for new dependency resolution.

### Documentation

- README: new Getting started section and a section on the Hazard3 packages that go with this
  crate, a clearer Testing section, and a single Development tools section.
- README: the CYW43 WiFi driver entry now points to
  [`hazard3-cyw43`](https://crates.io/crates/hazard3-cyw43) instead of the upstream `cyw43`.

### Removed / not provided

- No CI compilation or test suite is provided for this release.
- No examples or hardware integrations are compiled or tested.
- No automated build matrix or target-platform validation is provided.

## 0.3.0 - 2026-11-22

### Changed

- **Breaking:** `dns::Error` has a new variant `Unsupported`. Code that matches exhaustively on
  `dns::Error` must handle it. `dns::Error` also derives `Hash` now.
- `dns::DnsClient` (feature `embedded-nal`): `get_host_by_address` now returns
  `Err(Error::Unsupported)` instead of calling `todo!()`, which panicked at runtime when a crate
  attempted a reverse lookup. Reverse (PTR) lookups are still not implemented.
- `dns::DnsClient` (feature `embedded-nal`): `get_host_by_name` with `AddrType::Either` now also
  falls back to the secondary record type when the first query **fails**, not only when it returns
  no address. The fallback is skipped for `InvalidName` and `NameTooLong`, since a second query
  would fail the same way.
- `src/dns.rs`: `get_host_by_name` calls `Stack::dns_query` directly and uses `.first()` instead of
  `.get(0)`; the `DnsClient` documentation points to the module documentation instead of
  duplicating it; the `From<GetQueryResultError>` conversion documents why every variant maps to
  `Error::Failed`. `DnsClient::query` is unchanged.

### Added

- Unit tests in `src/dns.rs` (not present in upstream `embassy-net`): mapping of
  `GetQueryResultError` and `StartQueryError` variants to `dns::Error`, `Display` output of
  `dns::Error` (including `Unsupported`), and the `core::error::Error` guarantee. The tests format
  errors with `heapless::String` and no longer need a custom buffer writer.
- README: `dns.rs` in the test coverage table and in the project vision.

### Removed / not provided

- No CI compilation or test suite is provided for this release.
- No examples or hardware integrations are compiled or tested.
- No automated build matrix or target-platform validation is provided.

## 0.2.0 - 2026-11-21

### Changed

- Modularity: split the monolithic `src/udp.rs` into focused modules, following the same approach
  as the earlier `tcp.rs` split. The public API and the `udp::` paths are unchanged
  (`udp::UdpSocket`, `udp::BindError`, `udp::SendError`, `udp::RecvError`, `udp::UdpMetadata`,
  `udp::RecvPacket`, `udp::PacketMeta`, `udp::IfaceHandle`, ...), so this is not a breaking change.
  - `src/udp.rs`: module documentation, module wiring, re-exports, the `UdpSocket` struct and its
    `Drop` impl.
  - `src/udp/error.rs`: `BindError`, `SendError`, `RecvError`, their `Display` and
    `core::error::Error` impls, and the `From<xarxa::udp::SendError>` conversion to
    `TryError<SendError>`.
  - `src/udp/socket.rs`: `UdpSocket` construction, `bind`, interface binding (feature
    `iface-bind`), addresses, state (`is_open`, `can_recv`, `close`), hop limit and `try_alloc`.
  - `src/udp/recv.rs`: receiving and peeking (`recv*`, `peek*`, `poll_recv_ready`,
    `wait_recv_ready`).
  - `src/udp/send.rs`: sending (`send_to*`, `send_packet_to`, `try_send_packet_to`, in-place
    `send_to_with`) and the shared send helpers.
- The internal helper `UdpSocket::with` is now `pub(super)` so that the sibling modules can use it.
  It is not part of the public API.

### Added

- Unit tests in `src/udp/error.rs` (not present in upstream `embassy-net`): `Display` output of the
  UDP error types and the mapping of every `xarxa::udp::SendError` variant to `TryError<SendError>`.
- `SPDX-License-Identifier: MIT OR Apache-2.0` header on the new UDP source files.
- README: UDP files in the source layout, the UDP split in the project vision, and `udp/error.rs`
  in the test coverage table.

### Removed / not provided

- No CI compilation or test suite is provided for this release.
- No examples or hardware integrations are compiled or tested.
- No automated build matrix or target-platform validation is provided.

## 0.1.0 - 2026-11-20

### Changed

- Forked and integrated the pinned upstream Xarxa sources under new package names:
  `xarxa-riscv` and `xarxa-driver-riscv`. The Rust library names remain `xarxa` and
  `xarxa_driver`. The `embassy-net-riscv` manifest now uses local path dependencies on this
  repository's forks instead of Git dependencies, allowing publication on crates.io.
- Modularity: split the monolithic `src/tcp.rs` into focused modules. The public API and the
  `tcp::` paths are unchanged (`tcp::TcpSocket`, `tcp::TcpReader`, `tcp::TcpWriter`,
  `tcp::TcpListener`, `tcp::client::TcpClient`, ...), so this is not a breaking change.
  - `src/tcp.rs`: error types, shared `TcpIo` helper, module wiring and re-exports.
  - `src/tcp/socket.rs`: `TcpSocket` construction, `connect`/`accept`, options and state.
  - `src/tcp/socket/io.rs`: data transfer (`read`/`write`/`peek`/`flush`), `TcpReader`,
    `TcpWriter` and the `embedded-io` trait impls.
  - `src/tcp/listener.rs`: `TcpListener` (feature `tcp-listener`).
  - `src/tcp/client.rs`: `embedded-nal-async` client (feature `embedded-nal`).

### Added

- Unit tests in every TCP module (not present in upstream `embassy-net`):
  - `tcp.rs`: `Display` output, `ConnectError`/`Error` to `embedded_io_async::ErrorKind` mapping,
    and `Copy`/`Eq`/`core::error::Error` guarantees of the error types.
  - `tcp/client.rs`: runtime tests of the connection buffer pool (exhaustion, reuse, aliasing,
    slot layout).
  - `tcp/socket.rs`, `tcp/socket/io.rs`, `tcp/listener.rs`: compile-time checks of the public API
    signatures and of the `embedded-io` trait impls.
- `SPDX-License-Identifier: MIT OR Apache-2.0` header on the new TCP source files.
- README: project vision, source layout and testing sections.

### Removed / not provided

- No CI compilation or test suite is provided for this release.
- No examples or hardware integrations are compiled or tested.
- No automated build matrix or target-platform validation is provided.

## 2026-11-20

- First release of the `embassy-net-riscv` fork: copy of upstream `embassy-net` from the Embassy `main` branch (upstream changelog omitted).
- Republished under the `-riscv` package name for use with the RISC-V forked Embassy crates.