# Releasing Hazard3

The `hazard3-*` packages replace the former `*-riscv` packages. Their Rust
library names stay unchanged. Preserve upstream license and attribution files.
Check crates.io for the current publication status. The latest releases of the
old packages carry migration notices.

## Preparation

- Check the package names and versions on crates.io before publishing. A
  published version cannot be replaced.
- Run the target checks documented in each crate's README, and review the
  package contents with `cargo package --list`.
- Check the dependency versions and publish in dependency order. A dependent
  package cannot be verified against crates.io until its dependencies exist.
- Run `cargo publish --dry-run` with the same target and features before each
  upload. Publish from a clean, committed tree.

## Hazard3 migration status

As of 2026-11-24, the repository rename and yank transition are ready for the
following patch releases. These versions include the new `Hazard3` repository
metadata and supersede the corresponding earlier crates.io releases:

| Package | Version |
|---|---|
| `hazard3-unitrait-macros` | 0.1.1 |
| `hazard3-unitrait` | 0.1.1 |
| `hazard3-executor-timer-queue` | 0.1.1 |
| `hazard3-time-driver` | 0.1.2 |
| `hazard3-time-queue-utils` | 0.1.1 |
| `hazard3-sync` | 0.1.1 |
| `hazard3-executor-macros` | 0.1.1 |
| `hazard3-executor` | 0.1.2 |
| `hazard3-time` | 0.1.3 |
| `hazard3-hal-internal` | 0.1.1 |
| `hazard3-embedded-hal` | 0.1.1 |
| `hazard3-usb-driver` | 0.1.1 |
| `hazard3-rp-pac` | 0.2.1 |
| `hazard3-rp` | 0.2.1 |
| `hazard3-net-driver` | 0.1.1 |
| `hazard3-net` | 0.1.2 |
| `hazard3-net-driver-channel` | 0.1.1 |
| `hazard3-net-async` | 0.3.1 |
| `hazard3-cyw43` | 0.2.2 |
| `hazard3-cyw43-pio` | 0.2.2 |

The RISC-V publication dry-runs passed, and a fresh consumer using the registry
version of `hazard3-rp` built for `riscv32imac-unknown-none-elf` with the
existing `embassy_rp` library name.

## General publication order

One dependency order for the migrated packages is:

1. `hazard3-unitrait-macros`, `hazard3-unitrait`, `hazard3-executor-timer-queue`
2. `hazard3-time-driver`, `hazard3-time-queue-utils`, `hazard3-sync`
3. `hazard3-executor-macros`, `hazard3-executor`, `hazard3-time`
4. `hazard3-hal-internal`, `hazard3-embedded-hal`, `hazard3-usb-driver`, `hazard3-rp-pac`
5. `hazard3-rp`, `hazard3-net/hazard3-net-driver`, `hazard3-net`
6. `hazard3-net-driver-channel`, `hazard3-net-async`, `hazard3-cyw43`, `hazard3-cyw43-pio`

These are repository directory names, not package names. Allow each upload to
appear in the registry before publishing its dependants. For example, from
`hazard3-cyw43-pio/`, after its dependencies are available:

```sh
cargo publish --dry-run --target riscv32imac-unknown-none-elf --features embassy-rp/rp235xa
cargo publish --target riscv32imac-unknown-none-elf --features embassy-rp/rp235xa
```

## Former packages

Yanking is not a redirect or a migration message. It prevents new dependency
resolution from selecting the yanked version; existing lockfiles can continue
using it, and published archives remain downloadable. It may break downstream
builds.

As of 2026-11-24, all versions of the 20 former packages, including the
latest migration-notice releases, are yanked. The `hazard3-*` replacements
remain available.

```sh
cargo yank OLD-PACKAGE@VERSION
# To undo a yank:
cargo yank --undo OLD-PACKAGE@VERSION
```

See the [Cargo yank documentation](https://doc.rust-lang.org/cargo/commands/cargo-yank.html).
