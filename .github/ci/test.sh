#!/bin/bash
## on push branch~=gh-readonly-queue/main/.*
## on pull_request

set -euo pipefail

export RUSTUP_HOME=/ci/cache/rustup
export CARGO_HOME=/ci/cache/cargo
export CARGO_TARGET_DIR=/ci/cache/target

# needed for "dumb HTTP" transport support
# used when pointing stm32-metapac to a CI-built one.
export CARGO_NET_GIT_FETCH_WITH_CLI=true

cargo test --manifest-path ./hazard3-executor/Cargo.toml --features metadata-name
cargo test --manifest-path ./embassy-futures/Cargo.toml
cargo test --manifest-path ./hazard3-sync/Cargo.toml
cargo test --manifest-path ./hazard3-embedded-hal/Cargo.toml
cargo test --manifest-path ./hazard3-hal-internal/Cargo.toml
cargo test --manifest-path ./hazard3-time/Cargo.toml --features mock-driver,embassy-time-queue-utils/generic-queue-8
cargo test --manifest-path ./hazard3-time-driver/Cargo.toml
cargo test --manifest-path ./embassy-ptp-driver/Cargo.toml --all-features

cargo test --manifest-path ./embassy-boot/Cargo.toml

cargo test --manifest-path ./embassy-nrf/Cargo.toml --no-default-features --features nrf52840,time-driver-rtc1

for chip in rp235xa rp235xb; do
    cargo build --manifest-path ./hazard3-rp/Cargo.toml --target riscv32imac-unknown-none-elf --no-default-features --features "riscv,rt,$chip,time-driver"
    rp_deps=$(cargo tree --manifest-path ./hazard3-rp/Cargo.toml --target riscv32imac-unknown-none-elf --no-default-features --features "riscv,rt,$chip,time-driver" --edges normal --prefix none)
    if grep -Eq '^(cortex-m|cortex-m-rt|rp2040-boot2) v' <<< "$rp_deps"; then
        echo "Unexpected ARM or RP2040 dependency in $chip RISC-V build" >&2
        exit 1
    fi
done

cargo test --manifest-path ./embassy-stm32/Cargo.toml --no-default-features --features stm32f429vg,time-driver-any,exti,single-bank,low-power,jiff,test
cargo test --manifest-path ./embassy-stm32/Cargo.toml --no-default-features --features stm32f429vg,time-driver-any,exti,dual-bank,test
cargo test --manifest-path ./embassy-stm32/Cargo.toml --no-default-features --features stm32f732ze,time-driver-any,exti,test
cargo test --manifest-path ./embassy-stm32/Cargo.toml --no-default-features --features stm32f769ni,time-driver-any,exti,single-bank,test
cargo test --manifest-path ./embassy-stm32/Cargo.toml --no-default-features --features stm32f769ni,time-driver-any,exti,dual-bank,test
cargo test --manifest-path ./embassy-stm32/Cargo.toml --no-default-features --features stm32f103c8,time-driver-any,exti,test --lib usb::
cargo test --manifest-path ./embassy-stm32/Cargo.toml --no-default-features --features stm32l476rg,time-driver-any,exti,test --lib lcd::

cargo test --manifest-path ./embassy-net-adin1110/Cargo.toml
cargo test --manifest-path ./embassy-net-adin1110/Cargo.toml --no-default-features --features tc6,packetmeta-id
cargo test --manifest-path ./embassy-crypto-rustcrypto/Cargo.toml --features all
cargo test --manifest-path ./embassy-crypto-rand/Cargo.toml
cargo test --manifest-path ./embassy-usb-dfu/Cargo.toml --features dfu
cargo test --manifest-path ./embassy-usb/Cargo.toml --features block-device-driver
cargo test --manifest-path ./embassy-usb-synopsys-otg/Cargo.toml
cargo test --manifest-path ./hazard3-net-async/Cargo.toml --features tcp,dhcpv4,medium-ethernet,ipv6
