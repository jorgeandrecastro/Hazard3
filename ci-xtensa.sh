#!/bin/bash

set -eo pipefail

export RUSTFLAGS=-Dwarnings
export DEFMT_LOG=trace,embassy_hal_internal=debug,embassy_net_esp_hosted=debug,cyw43=info,cyw43_pio=info,xarxa=info
export RUSTUP_TOOLCHAIN=esp
if [[ -z "${CARGO_TARGET_DIR}" ]]; then
    export CARGO_TARGET_DIR=target_ci
fi

cargo embassy-devtool build --group xtensa

for features in "" "generic-queue-8"; do
    cargo check --release --manifest-path hazard3-time-queue-utils/Cargo.toml \
        --target xtensa-esp32s2-none-elf -Z build-std=core,alloc --features "$features"
done

for features in "" "defmt" "log"; do
    cargo check --release --manifest-path hazard3-futures/Cargo.toml \
        --target xtensa-esp32s2-none-elf -Z build-std=core,alloc --features "$features"
done
