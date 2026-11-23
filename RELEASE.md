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
using it. It may break downstream builds. Only consider yanking individual
versions after the new stack is published and verified, with an explicit
decision about the impact:

```sh
cargo yank OLD-PACKAGE@VERSION
# To undo a yank:
cargo yank --undo OLD-PACKAGE@VERSION
```

See the [Cargo yank documentation](https://doc.rust-lang.org/cargo/commands/cargo-yank.html).
