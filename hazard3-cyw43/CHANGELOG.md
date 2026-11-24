# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

- Repository renamed to [`jorgeandrecastro/Hazard3`](https://github.com/jorgeandrecastro/Hazard3).
- Former `*-riscv` releases remain preserved on crates.io but are yanked and are not recommended for new dependency resolution.

- Rename package `cyw43-riscv` to `hazard3-cyw43` while retaining the Rust library name.

## [0.2.1] - 2026-11-22

### Added

- Added the complete ESP-NOW Pico 2 W and ESP32-C3 example under
  [`exemples/espnow-pico2w`](exemples/espnow-pico2w/README.md).
- Added the benchmark report under
  [`exemples/espnow-pico2w/Benchmraks.md`](exemples/espnow-pico2w/Benchmraks.md).

### Packaging

- Excluded `exemples/` from the `cyw43-riscv` package archive so example source
  files and their separate dependencies do not add weight to the published crate
  or to `cyw43-riscv` binaries.

## [0.2.0] - 2026-11-22

### Added

- ESP-NOW bidirectional hardware validation using the CYW43439 on Raspberry Pi Pico 2 W.
- ESP32-C3 Mini interoperability test.
- PING/PONG application protocol with 32-bit sequence numbers.
- Action-frame reception and response transmission.
- Sequence-number validation and response matching.
- Round-trip time (RTT) measurement.
- Packet statistics, invalid-payload detection, and timeout monitoring.
- Hardware benchmark documentation.

### Validated

ESP-NOW communication between:

- Raspberry Pi Pico 2 W (RP2350 + CYW43439).
- ESP32-C3 Mini.

Recorded benchmark results:

| Metric | Result |
|---|---:|
| Exchanges completed | 1,000 |
| TX attempts | 1,000 |
| TX submissions | 1,000 |
| TX errors | 0 |
| Matched PONGs | 1,000 |
| Invalid PONGs | 0 |
| Unexpected PONGs | 0 |
| Timeouts | 0 |
| Success rate | 100.0% |
| Timeout rate | 0.0% |
| Elapsed time | 1,007 seconds |

### RTT benchmark

| Metric | Result |
|---|---:|
| Minimum | 12.648 ms |
| Maximum | 196.103 ms |
| Average | 13.732 ms |
| P50 | ~12.850 ms |
| P95 | ~14.250 ms |
| P99 | ~23.050 ms |

The configured benchmark target was 10,000 exchanges. The recorded results represent the first 1,000 exchanges, not a completed 10,000-packet test.

### Documentation

- Added ESP-NOW experimental validation results to the README.
- Documented the hardware configuration and communication protocol.
- Documented RTT statistics and benchmark limitations.
- Clarified the distinction between the RISC-V fork and the original Embassy project.

## [0.1.0] - 2026-11-22

### Added

- Initial RISC-V fork named `cyw43-riscv`.
- Retained the Rust library name `cyw43`.
- Adopted forked time, synchronization, and network channel crates.
- Added CYW43455 support over SDIO (`Cyw43455`, `new_43455_sdio`).
- Added RISC-V compatibility work for the CYW43 driver.

### Changed

- Breaking: `Control::init` now takes a `Config`, whose `country` field configures radio regulatory rules, including channels and transmit power, instead of always using worldwide rules.
- Exposed `countries` publicly.
- Passing `Config::default()` retains worldwide rules.
- `State` is now `Send`, allowing storage in a `ConstStaticCell`.
- The multicast hardware address filter is managed automatically by `embassy-net`/`xarxa` through the driver trait.
- Removed `Control::add_multicast_address` and `Control::remove_multicast_address`.

### Fixed

- Fixed `reset_core` waiting for the backplane to become busy instead of idle, which caused timeouts on SDIO chips with a Cortex-R4.
- Cleared the scan event mask and subscription on cancellation.
- Improved WiFi responsiveness when the Bluetooth RX queue is full.
- Resumed queued Bluetooth records when queue space becomes available.
- Disassociated before retrying failed or cancelled joins.
- Returned `JoinError::InvalidPassphrase` instead of panicking when joining with an invalid passphrase.
- Preserved host-owned Bluetooth control bits during simultaneous WiFi and Bluetooth operation.

### Bluetooth

- Added `bt-hci-transport` 0.1.0 support.
- Replaced Bluetooth SPI polling with interrupts.
- Added Bluetooth Classic and LE HCI support.

### WiFi

- Added WPA3 SoftAP support.
- Added WPA2/WPA3 transition-mode SoftAP support.

---

## Upstream Embassy CYW43 history

The following entries describe upstream `cyw43` releases and are retained for historical reference. They are not releases of this RISC-V fork.

### Upstream 0.7.0 - 2026-03-10

- Reset WPA security before creating a secure AP.
- Added `get_rssi()` to `Controller`.
- Implemented link state monitoring.
- Always unsubscribe from CYW43 events to fix event leaks.
- Added SDIO support for STM32.
- Allowed specifying NVRAM and improved LWB+ support.
- Improved join handling and error reporting.
- Replaced `num_enum` dependency with an internal macro.
- Updated `bt-hci` to 0.8.0.
- Updated `embedded-io` to 0.7.
- Updated `embassy-sync` to 0.8.0.
- Updated `embassy-net-driver-channel` to 0.4.0.

### Upstream 0.6.0 - 2025-11-27

- Updated documentation for `Control::join()`.
- Updated `bt-hci` to 0.6.0.
- Added error handling to HCI transport implementation.
- Reset WPA security on AP creation.

### Upstream 0.5.0 - 2025-08-28

- Updated `bt-hci` to 0.4.0.

### Upstream 0.4.1 - 2025-08-26 (yanked)

- Updated `bt-hci` to 0.4.0.

### Upstream 0.4.0 - 2025-07-15

- Updated `embassy-sync` to 0.7.0.
- Updated `bt-hci` to 0.3.0.
- Made `State::new` a `const fn`.

### Upstream 0.3.0 - 2025-01-05

- Updated `embassy-time` to 0.4.0.
- Added Bluetooth support.
- Added WPA3 support.
- Expanded WiFi security configuration options.

### Upstream 0.2.0 - 2024-08-05

- Updated to new versions of `embassy-time` and `embassy-sync`.
- Added more fields to the `BssInfo` packet structure.
- Extended the Scan API.
- Reused buffers to reduce stack usage.
- Added MAC address getter to the controller.
- Added support for joining WPA2 networks with a precomputed PSK.
- Added functionality to close a SoftAP.
- Fixed a missing re-export.

### Upstream 0.1.0 - 2024-01-11

- First upstream release.