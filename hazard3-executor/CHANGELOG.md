# Changelog for hazard3-executor

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- next-header -->
## Unreleased

- Rename package `embassy-executor-riscv` to `hazard3-executor` while retaining the Rust library name.

## 0.1.1 - 2026-11-18

- Depend on `embassy-time-driver-riscv` 0.1.1 (which no longer sets the `links = "embassy-time"` manifest key).

## 0.1.0 - 2026-11-18

- First release of the `embassy-executor-riscv` fork: unmodified copy of upstream `embassy-executor` from the Embassy `main` branch (changelog of upstream versions omitted), republished so that `embassy-rp-riscv` can be published on crates.io.
