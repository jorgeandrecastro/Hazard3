# Embassy-RISCV

> **RISC-V fork of Embassy.** This independent fork of [Embassy](https://github.com/embassy-rs/embassy)
> is maintained by [Jorge Andre Castro](https://github.com/jorgeandrecastro) at
> <https://github.com/jorgeandrecastro/embassy-riscv>. It adds RISC-V support, for example for the
> RP2350 Hazard3 cores, which upstream Embassy does not provide.

Embassy-RISCV keeps the upstream Rust library names while publishing RISC-V-compatible forks under
`-riscv` package names. Use the `-riscv` crates together to avoid duplicate or incompatible copies
of Embassy dependencies.

## RISC-V networking

[`embassy-net-riscv`](embassy-net/) is required to bring Embassy networking to this RISC-V ecosystem.
It cannot publish successfully when it depends directly on upstream Git dependencies, and it must be
aligned with the other `-riscv` crates.

This repository also provides the pinned Xarxa sources needed by `embassy-net-riscv`:

- [`xarxa-riscv`](xarxa-riscv/) — network stack package.
- [`xarxa-driver-riscv`](xarxa-riscv/xarxa-driver/) — driver interface package.

The Rust library names remain unchanged: `embassy_net`, `xarxa`, and `xarxa_driver`. Cargo users
must select the forked packages with `package = "embassy-net-riscv"`,
`package = "xarxa-riscv"`, or `package = "xarxa-driver-riscv"` as appropriate.

These networking forks are published under the same repository and are not replacements for upstream
Xarxa or upstream Embassy outside RISC-V projects.

## Test and build scope

No maintained CI workflow, example build matrix, or hardware integration test suite is provided.
The networking crates do not compile or test upstream examples and target-specific hardware
integrations in this repository. Their host-oriented checks and development commands are documented
in the individual crate READMEs.

## License

The Embassy-RISCV sources remain licensed under the upstream MIT OR Apache-2.0 terms:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

The integrated [`xarxa-riscv`](xarxa-riscv/) and [`xarxa-driver-riscv`](xarxa-riscv/xarxa-driver/)
sources retain the upstream 0-Clause BSD license.

## Acknowledgements

Embassy-RISCV is derivative work based on the Embassy project and the Xarxa project. All credit for
the original code goes to their respective authors and contributors:

- [Embassy](https://github.com/embassy-rs/embassy)
- [Xarxa](https://github.com/embassy-rs/xarxa)

The original copyright and license notices are preserved in the corresponding directories.
Embassy-RISCV is not affiliated with or endorsed by the Embassy or Xarxa projects.
