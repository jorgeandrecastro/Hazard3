# cyw43-pio-riscv

RISC-V fork of Embassy's PIO driver for the CYW43 half-duplex SPI bus on the
RP2350, including the Pico 2 W. It uses `embassy-rp-riscv` and `cyw43-riscv`;
it does not support RP2040 or Arm.

The package name changes, but the Rust library name and API remain `cyw43_pio`.
Use the matching forked packages together:

```toml
[dependencies]
cyw43 = { package = "cyw43-riscv", version = "0.1" }
cyw43-pio = { package = "cyw43-pio-riscv", version = "0.1" }
embassy-rp = { package = "embassy-rp-riscv", version = "0.2", features = ["rp235xa"] }
```

Select `rp235xb` instead for an RP2350B. The current `PioSpi::new` takes
separate TX and RX DMA channels and a clock divider scaled by 256, as in the
in-repository upstream-derived implementation. The fork uses the upstream
MIT OR Apache-2.0 licenses.

## Development

Run these commands from the repository root. Install the RISC-V target if needed:

```sh
rustup target add riscv32imac-unknown-none-elf
```

Check the integration with the HAL, CYW43 driver and RISC-V networking stack:

```sh
# RP2350A
cargo check --manifest-path cyw43-pio/Cargo.toml --test compatibility --target riscv32imac-unknown-none-elf --features embassy-rp/rp235xa

# RP2350B with defmt
cargo check --manifest-path cyw43-pio/Cargo.toml --test compatibility --target riscv32imac-unknown-none-elf --features embassy-rp/rp235xb,defmt
```

Generate the documentation:

```sh
cargo doc \
    --manifest-path cyw43-pio/Cargo.toml \
    --target riscv32imac-unknown-none-elf \
    --features embassy-rp/rp235xa \
    --no-deps \
    --document-private-items
```

Package and publish with the RP2350A target configuration:

```sh
cargo package --manifest-path cyw43-pio/Cargo.toml \
    --target riscv32imac-unknown-none-elf \
    --features embassy-rp/rp235xa
cargo publish --manifest-path cyw43-pio/Cargo.toml \
    --target riscv32imac-unknown-none-elf \
    --features embassy-rp/rp235xa
```

## Credits

This crate is derived from [Embassy's `cyw43-pio`](https://github.com/embassy-rs/embassy/tree/main/cyw43-pio).
All credit for the original work goes to Embassy and its contributors. This
independent fork focuses on RISC-V and is not affiliated with or endorsed by Embassy.
