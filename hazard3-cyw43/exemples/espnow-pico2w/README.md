# ESP-NOW — Raspberry Pi Pico 2 W ↔ ESP32-C3

Bidirectional ESP-NOW communication between a Raspberry Pi Pico 2 W (RP2350 + CYW43439) and an ESP32-C3 Mini, implemented in Rust using Embassy and ESP-HAL.

This example demonstrates cross-platform wireless communication between two independent RISC-V embedded systems using different Wi-Fi radio stacks.

## Overview

The project implements a bidirectional PING/PONG communication link.

| Device | Role | Platform |
|---|---|---|
| Raspberry Pi Pico 2 W | PING receiver / PONG responder | RP2350 + CYW43439 |
| ESP32-C3 Mini | PING initiator / PONG receiver | ESP32-C3 |

Both devices operate on Wi-Fi channel 1.

### Architecture

```text
+---------------------------+
|       ESP32-C3 Mini       |
|                           |
|        ESP-HAL            |
|        ESP-Radio          |
|        ESP-RTOS           |
|                           |
|      PING + SEQUENCE      |
|             |             |
+-------------|-------------+
              |
              | ESP-NOW
              | Wi-Fi Channel 1
              |
+-------------v-------------+
|      Raspberry Pi Pico 2 W|
|                           |
|         RP2350            |
|         Embassy           |
|         CYW43439          |
|         hazard3-cyw43     |
|         hazard3-cyw43-pio |
|                           |
|       PING PARSER         |
|       PONG GENERATOR      |
|             |             |
|          SSD1306          |
|       OLED DEBUG          |
+-------------|-------------+
              |
              +---- PONG ----> ESP32-C3
```

## Hardware

### Raspberry Pi Pico 2 W

- Raspberry Pi Pico 2 W
- RP2350 microcontroller
- CYW43439 Wi-Fi/Bluetooth radio
- SSD1306 OLED, 128 × 64, I2C (optional)

### ESP32-C3

- ESP32-C3 Mini development board
- Integrated 2.4 GHz Wi-Fi radio

No external RF connection is required. Both boards use their integrated antennas.

### OLED connection

The Pico 2 W example uses an SSD1306 display.

| SSD1306 | Pico 2 W |
|---|---|
| VCC | 3V3 |
| GND | GND |
| SDA | GPIO 4 |
| SCL | GPIO 5 |

I2C address: `0x3C`

## Software architecture

### Raspberry Pi Pico 2 W

The Pico 2 W firmware uses:

- Rust `no_std`
- Embassy executor
- `hazard3-cyw43`
- `hazard3-cyw43-pio`
- RP2350 PIO SPI transport
- DMA
- SSD1306 OLED driver

Responsibilities:

1. Initialize the RP2350.
2. Initialize the CYW43439.
3. Load the Wi-Fi firmware, CLM and NVRAM.
4. Configure Wi-Fi channel 1.
5. Enable Action Frame reception.
6. Parse incoming PING frames.
7. Extract the sequence number.
8. Construct a PONG response.
9. Transmit the response.
10. Update OLED diagnostics and statistics.

### ESP32-C3 Mini

The ESP32-C3 firmware uses:

- Rust `no_std`
- ESP-HAL
- ESP-Radio
- ESP-RTOS
- Embassy Time utilities

Responsibilities:

1. Initialize the ESP32-C3.
2. Configure the heap.
3. Start ESP-RTOS.
4. Initialize the Wi-Fi radio.
5. Initialize ESP-NOW.
6. Configure channel 1.
7. Generate PING messages.
8. Transmit messages.
9. Receive PONG responses.
10. Validate sequence numbers.
11. Measure round-trip time.
12. Calculate transmission statistics.

## ESP-NOW protocol

The application uses a PING/PONG message format.

### PING

```text
Offset    Size    Description
--------------------------------
0         4       ASCII "PING"
4         4       Sequence number
--------------------------------
Total:    8 bytes
```

The sequence number is encoded as a little-endian `u32`.

### PONG

```text
Offset    Size    Description
--------------------------------
0         4       ASCII "PONG"
4         4       Sequence number
--------------------------------
Total:    8 bytes
```

The PONG response carries the original sequence number.

This allows the initiator to associate a response with its corresponding transmission.

### Action Frame encapsulation

The Pico 2 W implementation handles Vendor-Specific Action Frames.

The custom frame builder uses:

| Field | Size |
|---|---:|
| Category | 1 byte |
| OUI | 3 bytes |
| Random field | 4 bytes |
| Element ID | 1 byte |
| Element length | 1 byte |
| Vendor OUI | 3 bytes |
| Type | 1 byte |
| Version | 1 byte |
| Payload | Variable |

The OUI configured in the example is `18:FE:34`.

The ESP-NOW payload itself is 8 bytes.

## Configuration

Both firmwares must use the same Wi-Fi channel.

```rust
const CHANNEL: u8 = 1;
```

The ESP32-C3 uses a broadcast destination for the initial communication test.

The Pico 2 W responds using the configured broadcast Action Frame transmission.

## Building

The two firmware applications have independent build configurations.

### Raspberry Pi Pico 2 W

Install the required target:

```bash
rustup target add riscv32imac-unknown-none-elf
```

In the Pico 2 W firmware directory, configure `.cargo/config.toml`:

```toml
[build]
target = "riscv32imac-unknown-none-elf"

[target.riscv32imac-unknown-none-elf]
rustflags = [
    "-C", "link-arg=--nmagic",
    "-C", "link-arg=-Tmemory.x",
    "-C", "link-arg=-Tlink.x",
    "-C", "link-arg=-Tembassy_rp_irq.x",
    "-C", "link-arg=-Tdefmt.x",
]
```

The external `embassy-ssd1306` dependency still uses the former
`embassy-time-riscv` package; this example therefore includes both time packages
until that driver is migrated.

Build from the Pico 2 W firmware directory:

```bash
cd hazard3-cyw43/exemples/espnow-pico2w/pico2w

cargo build --release \
    --target riscv32imac-unknown-none-elf
```

Use the target and runner configuration appropriate to the RP2350 RISC-V firmware and the selected linker.

### ESP32-C3 Mini

Install the target:

```bash
rustup target add riscv32imc-unknown-none-elf
```

Build:

```bash
cd firmware/esp32c3

cargo build --release \
    --target riscv32imc-unknown-none-elf
```

## Running the experiment

1. Flash the Pico 2 W firmware.
2. Connect the SSD1306 display if available.
3. Flash the ESP32-C3 firmware.
4. Configure both boards to channel 1.
5. Start the Pico 2 W receiver.
6. Start the ESP32-C3 transmitter.
7. Observe the PING/PONG exchange.
8. Monitor RTT and transmission statistics.

The Pico 2 W OLED provides diagnostic information without requiring access to a debug probe.

## OLED diagnostics

The Pico 2 W firmware provides several diagnostic screens.

| Screen | Information |
|---|---|
| System | CYW43439, RP2350 and radio status |
| Link statistics | RX packets, valid PINGs, TX replies and errors |
| Last frame | RSSI, frame length and sequence |
| Source | Last transmitting MAC address |
| Debug | Parsing status, last TX result and diagnostic counters |

The display refreshes periodically.

## Benchmarking

The ESP32-C3 initiator measures the round-trip time between a PING transmission and the corresponding PONG reception.

The following metrics are collected:

- TX attempts
- TX submitted
- TX errors
- Matched PONG responses
- Invalid PONG responses
- Unexpected responses
- Timeouts
- Minimum RTT
- Maximum RTT
- Average RTT
- P50 RTT
- P95 RTT
- P99 RTT

See [BENCHMARKS.md](BENCHMARKS.md) for the experimental results and methodology.

## Limitations

- The example currently targets the Raspberry Pi Pico 2 W and ESP32-C3.
- Both devices must operate on the same Wi-Fi channel.
- The experiment uses broadcast transmission.
- Results depend on radio conditions, firmware versions, scheduler activity and measurement methodology.
- TX submission does not independently prove successful reception. The matched PONG is the application-level confirmation.
- The benchmark results are specific to the tested hardware and software configuration.

## Related projects

- [hazard3-cyw43](https://github.com/jorgeandrecastro/cyw43-riscv)
- [Embassy](https://github.com/embassy-rs/embassy)
- [esp-hal](https://github.com/esp-rs/esp-hal)
- [esp-radio](https://github.com/esp-rs/esp-radio)

## License

This project is an independent implementation example and is not affiliated with or endorsed by Raspberry Pi, Espressif or the Embassy project.