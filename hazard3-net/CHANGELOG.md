# Changelog

## 0.4.0 - 2026-11-28

### Breaking changes

- **Breaking:** rename the Rust library `xarxa` to `hazard3_net`; update imports and macro paths.
- Use matching new Hazard3 dependency versions where applicable.

## 0.2.0 - 2026-11-27

### Breaking changes

- `AllocPool` now recycles its buffers. A dropped buffer goes to a free list instead of back
  to the heap, and is reused by the next `alloc`. As a result:
  - `max_count` now limits the total number of buffers taken from the heap (in use plus
    cached), not only the buffers in use.
  - The heap memory of cached buffers is kept until `AllocPool::trim()` is called.
- `AllocPool` protects its state with a `critical-section`. Firmware that uses `AllocPool` must
  provide a `critical-section` implementation (for example from the HAL of the chip). On a
  multi-core chip such as the RP2350, use an implementation that also protects against the
  other core, not `critical-section-single-hart`.
- `critical-section` is now a normal dependency on every target. It was only a dependency on
  targets without atomic read-modify-write. It is only linked when used.

### Added

- `AllocPool::trim()`: gives the heap memory of all cached buffers back to the allocator.
  Buffers that are in use are not affected, and each buffer is released in its own short
  critical section.
- 19 unit tests for `AllocPool`: exhaustion, `max_count` of 0, recycling, free-list integrity
  under any drop order, `trim`, use through `PoolRef`, slot layout and alignment for every
  supported `ALIGN`, a size guard on the pool struct, and a multi-threaded stress test.
- `rust-version = "1.95"` in `Cargo.toml`.
- README: getting started, a section on packet pools, a RISC-V and Hazard3 section, size and
  RAM tuning advice, and testing instructions.

### Changed

- `AllocPool` is faster in steady state: `alloc` and `drop` no longer call the heap allocator
  and no longer clear the buffer storage once a slot exists. Only new slots are zeroed.
- `AllocPool` is smaller: `alloc` has a single initialization path for new and recycled slots,
  and `free` is reduced to pushing the slot on the free list. The free list is stored in the
  header of the free slots, so it uses no extra memory.
- `StaticPool` and `AllocPool` keep the same `unsafe` boundaries; the `SAFETY` comments of
  `AllocPool` were rewritten for the new design.
- Rename package `xarxa-riscv` to `hazard3-net` while retaining the Rust library name.
- Repository renamed to [`jorgeandrecastro/Hazard3`](https://github.com/jorgeandrecastro/Hazard3).
- Documentation of the `tcp-cubic` feature now recommends `tcp-reno` on cores without a
  floating-point unit, such as Hazard3.
- Documentation of the `alloc` and `std` features updated for `critical-section`.
- Fixed a typo in the documentation of the `Pool` trait ("mist" to "most").

### Fixed

- `AllocPool` did not compile on targets without atomic read-modify-write (for example
  `thumbv6m` and `riscv32imc`), because it used `compare_exchange_weak` on an `AtomicUsize`.
  It no longer uses atomic read-modify-write operations.

### Notes

- Former `*-riscv` releases remain preserved on crates.io but are yanked and are not
  recommended for new dependency resolution.

## 0.1.1 - 2026-11-20

- Removed `tcp-cubic` from the default features to avoid enabling two mutually exclusive TCP
  congestion-control implementations.
- Removed `log` from the default features to avoid enabling `log` and `defmt` simultaneously.

## 0.1.0

- Initial `xarxa-riscv` release.
- Republished the upstream `xarxa` source pinned at revision
  `8f6f33b131deb9ff3290e0e01079537e1ed7b2ad`.
- Published under the `xarxa-riscv` package name while keeping the Rust library name `xarxa`.