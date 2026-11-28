# Changelog for hazard3-executor-macros

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).


## 0.3.1 - 2026-11-28

### Breaking changes

- **Breaking:** rename the Rust library `embassy_executor_macros` to `hazard3_executor_macros`; update imports and macro paths.
- Use matching new Hazard3 dependency versions where applicable.

### Other changes

- Repository renamed to [`jorgeandrecastro/Hazard3`](https://github.com/jorgeandrecastro/Hazard3).
- Former `*-riscv` releases remain preserved on crates.io but are yanked and are not recommended for new dependency resolution. - 2026-11-18

- Rename package `embassy-executor-macros-riscv` to `hazard3-executor-macros`.

## 0.1.0 - 2026-11-18

- First release of the `embassy-executor-macros-riscv` fork: unmodified copy of upstream `embassy-executor-macros` from the Embassy `main` branch (changelog of upstream versions omitted), republished so that `embassy-rp-riscv` can be published on crates.io.
