# cyw43-riscv

RISC-V fork of the Embassy CYW43 WiFi and Bluetooth driver, based on
[Infineon/wifi-host-driver](https://github.com/Infineon/wifi-host-driver). The
package is named `cyw43-riscv`, while the Rust library name remains `cyw43`.
It uses the forked time, synchronization and network channel crates and is
licensed under MIT OR Apache-2.0.

Upstream lists the following boards; this fork targets RISC-V and the RP2350
PIO transport is provided by `cyw43-pio-riscv`:

- Raspberry Pi Pico 2 W (RP2350A)
- Pimoroni Pico Plus 2 W (RP2350B)
- Any board with Raspberry Pi RM2 radio module.
- Any board with the CYW43439 chip, and possibly others if the protocol is similar enough.

## Features

Working:

- WiFi support
    - Station mode (joining an AP).
    - AP mode with open, WPA2, WPA3, and WPA2/WPA3 transition-mode security. WPA3 requires compatible CYW43 firmware and client support.
    - Scanning
    - Sending and receiving Ethernet frames.
    - Using the default MAC address.
    - [`embassy-net`](https://embassy.dev) integration.
    - RP2350 PIO driver for the nonstandard half-duplex SPI used in the Pico 2 W.
    - Using IRQ for device events, no busy polling.
    - GPIO support (for LED on the Pico W).
- Bluetooth support
    - Bluetooth Classic + LE HCI commands.
    - Concurrent operation with WiFi.
    - Implements the [bt-hci](https://crates.io/crates/bt-hci) controller traits.
    - Works with the [TrouBLE](https://github.com/embassy-rs/trouble) bluetooth LE stack. Check its repo for examples using `cyw43`.

The repository's upstream Arm examples are not maintained for this fork.
See the [`cyw43-pio-riscv` README](../cyw43-pio/README.md) for dependency
selection and the RISC-V compatibility check.

## Development

Run these commands from the `cyw43` directory. Install the RISC-V target if needed:

```sh
rustup target add riscv32imac-unknown-none-elf
```

Check the driver for RISC-V, including Bluetooth and firmware logging:

```sh
cargo check --target riscv32imac-unknown-none-elf
cargo check --target riscv32imac-unknown-none-elf --features bluetooth,defmt,firmware-logs
```

Run the host tests:

```sh
cargo test --features bluetooth
```

Generate the documentation:

```sh
cargo doc \
    --target riscv32imac-unknown-none-elf \
    --features bluetooth,defmt,firmware-logs \
    --no-deps \
    --document-private-items
```

Publish `embassy-net-driver-channel-riscv` first, then package and publish this
crate for RISC-V:

```sh
cargo package --target riscv32imac-unknown-none-elf
cargo publish --target riscv32imac-unknown-none-elf
```

## Credits

This crate is derived from [Embassy's `cyw43`](https://github.com/embassy-rs/embassy/tree/main/cyw43).
All credit for the original work goes to Embassy and its contributors. This
independent fork focuses on RISC-V and is not affiliated with or endorsed by Embassy.
