# Changelog for hazard3-time-driver

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->

## Unreleased

- Repository renamed to [`jorgeandrecastro/Hazard3`](https://github.com/jorgeandrecastro/Hazard3).
- Former `*-riscv` releases remain preserved on crates.io but are yanked and are not recommended for new dependency resolution.

- Rename package `embassy-time-driver-riscv` to `hazard3-time-driver` while retaining the Rust library name.

## 0.1.1 - 2026-11-18

- Removed the `links = "embassy-time"` manifest key (unused: no `DEP_EMBASSY_TIME_*` variable is consumed in the workspace), so several copies of the crate no longer conflict at resolution time.

## 0.1.0 - 2026-11-18

- First release of the `embassy-time-driver-riscv` fork: unmodified copy of upstream `embassy-time-driver` from the Embassy `main` branch (changelog of upstream versions omitted), republished so that `embassy-rp-riscv` can be published on crates.io.
