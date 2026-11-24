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

## Resume the Hazard3 migration

As of 2026-11-23, these packages are published: `hazard3-unitrait-macros`
0.1.0, `hazard3-unitrait` 0.1.0, `hazard3-executor-timer-queue` 0.1.0,
`hazard3-time-driver` 0.1.1, `hazard3-time-queue-utils` 0.1.0, `hazard3-sync`
0.1.0 and `hazard3-executor-macros` 0.1.0. Check crates.io again before
resuming, because a published version cannot be replaced.

The remaining packages, in publication order, are:

| Package | Version |
|---|---|
| `hazard3-executor` | 0.1.1 |
| `hazard3-time` | 0.1.2 |
| `hazard3-hal-internal` | 0.1.0 |
| `hazard3-embedded-hal` | 0.1.0 |
| `hazard3-usb-driver` | 0.1.0 |
| `hazard3-rp-pac` | 0.2.0 |
| `hazard3-rp` | 0.2.0 |
| `hazard3-net-driver` | 0.1.0 |
| `hazard3-net` | 0.1.1 |
| `hazard3-net-driver-channel` | 0.1.0 |
| `hazard3-net-async` | 0.3.0 |
| `hazard3-cyw43` | 0.2.1 |
| `hazard3-cyw43-pio` | 0.2.1 |

Install the `riscv32imac-unknown-none-elf` target and authenticate with
crates.io. From the repository root, define this helper; it performs a dry run
before publishing each package:

```sh
publish_hazard3() {
    dir=$1
    shift
    cargo publish --dry-run --manifest-path "$dir/Cargo.toml" --target riscv32imac-unknown-none-elf "$@" || return
    cargo publish --manifest-path "$dir/Cargo.toml" --target riscv32imac-unknown-none-elf "$@"
}
```

Run the following commands in order, one at a time. Stop if a command fails;
wait for each upload to become available on crates.io before publishing its
dependants:

```sh
publish_hazard3 hazard3-time
publish_hazard3 hazard3-embedded-hal
publish_hazard3 hazard3-usb-driver
publish_hazard3 hazard3-rp-pac --features rp235x
publish_hazard3 hazard3-rp --features rp235xa,time-driver
publish_hazard3 hazard3-net/hazard3-net-driver
publish_hazard3 hazard3-net --no-default-features --features async,medium-ethernet,ipv4,udp,tcp
publish_hazard3 hazard3-net-driver-channel
publish_hazard3 hazard3-net-async --features medium-ethernet,ipv4,udp,tcp
publish_hazard3 hazard3-cyw43 --features action-frames
publish_hazard3 hazard3-cyw43-pio --features embassy-rp/rp235xa
```

Crates.io may rate-limit new packages. If it returns HTTP 429, wait until the
indicated time and resume at the failed package. If an upload's result is
unclear, check crates.io before retrying. After all uploads, build and link a
fresh RISC-V consumer using the registry packages without local path or patch
overrides, and verify that the existing `embassy_*` imports work. Do not yank
the old packages before this verification succeeds.

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
using it. It may break downstream builds. After the new stack is published and
verified, keep the latest migration-notice release of each old package available
and yank only its older releases, after checking the current versions:

```sh
cargo yank OLD-PACKAGE@VERSION
# To undo a yank:
cargo yank --undo OLD-PACKAGE@VERSION
```

See the [Cargo yank documentation](https://doc.rust-lang.org/cargo/commands/cargo-yank.html).
