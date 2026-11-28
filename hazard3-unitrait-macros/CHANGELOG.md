# Changelog for hazard3-unitrait-macros

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->
## Unreleased (0.2.0)

### Breaking changes

- **Breaking:** rename the Rust library `unitrait_macros` to `hazard3_unitrait_macros`; update imports and macro paths.
- Use matching new Hazard3 dependency versions where applicable.

### Other changes

- Repository renamed to [`jorgeandrecastro/Hazard3`](https://github.com/jorgeandrecastro/Hazard3).
- Former `*-riscv` releases remain preserved on crates.io but are yanked and are not recommended for new dependency resolution. - 2026-11-18

- Rename package `unitrait-macros-riscv` to `hazard3-unitrait-macros`.

## 0.1.0 - 2026-11-18

- First release of the `unitrait-macros-riscv` fork: unmodified copy of upstream `unitrait-macros` (changelog of upstream versions omitted), republished so that `embassy-rp-riscv` can be published on crates.io.
