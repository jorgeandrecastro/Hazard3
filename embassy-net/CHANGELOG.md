# Changelog for embassy-net-riscv

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## 0.1.0 - 2026-11-20

### Changed

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

## 2026-11-20

- First release of the `embassy-net-riscv` fork: copy of upstream `embassy-net` from the Embassy `main` branch (upstream changelog omitted).
- Republished under the `-riscv` package name for use with the RISC-V forked Embassy crates.