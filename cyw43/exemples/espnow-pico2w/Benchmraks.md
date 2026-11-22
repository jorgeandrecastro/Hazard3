# ESP-NOW Benchmarks

Performance measurements for bidirectional ESP-NOW communication between a Raspberry Pi Pico 2 W and an ESP32-C3 Mini.

## 1. Test configuration

| Parameter | Configuration |
|---|---|
| Initiator | ESP32-C3 Mini |
| Responder | Raspberry Pi Pico 2 W |
| Initiator MCU | ESP32-C3 |
| Responder MCU | RP2350 |
| Wi-Fi radio (initiator) | ESP32-C3 integrated radio |
| Wi-Fi radio (responder) | CYW43439 |
| Communication | ESP-NOW |
| Wi-Fi channel | 1 |
| Message type | PING / PONG |
| Payload size | 8 bytes |
| Sequence number | `u32` little-endian |
| Measurement clock | ESP32-C3 `embassy_time::Instant` |

## 2. Benchmark methodology

The ESP32-C3 acts as the communication initiator.

The experiment follows this sequence:

1. Generate a new sequence number.
2. Construct an 8-byte PING payload.
3. Record the transmission timestamp.
4. Submit the PING using ESP-NOW.
5. Wait for an incoming PONG.
6. Validate the PONG payload.
7. Extract the returned sequence number.
8. Match the received sequence against the pending transmission.
9. Record the reception timestamp.
10. Calculate the round-trip time.
11. Update the statistics.

The Pico 2 W receives the PING, validates its contents and generates a PONG containing the same sequence number.

### RTT calculation

The round-trip time is calculated as:

\[
RTT = t_{\text{PONG received}} - t_{\text{PING submitted}}
\]

The implementation uses microseconds:

```rust
let rtt_us = Instant::now()
    .as_micros()
    .saturating_sub(pending_timestamp_us);
```

This measurement includes:

- Radio transmission time
- Receiver processing
- PONG generation
- Response transmission
- Initiator reception and processing

It is an application-level round-trip measurement, not a one-way propagation measurement.

## 3. Initial experimental results

The following results come from an actual execution of the benchmark.

<details>
<summary>Experimental run: 1,000 packets</summary>

| Metric | Result |
|---|---:|
| TX attempts | 1,000 |
| TX submitted | 1,000 |
| TX errors | 0 |
| Matched PONGs | 1,000 |
| Invalid PONGs | 0 |
| Unexpected PONGs | 0 |
| Timeouts | 0 |
| Success rate | 100.0% |
| Timeout rate | 0.0% |
| Elapsed time | 1,007 seconds |

</details>

### RTT measurements

| Metric | Result |
|---|---:|
| Minimum RTT | 12,648 µs |
| Maximum RTT | 196,103 µs |
| Average RTT | 13,732 µs |
| P50 RTT | ~12,850 µs |
| P95 RTT | ~14,250 µs |
| P99 RTT | ~23,050 µs |

### Summary

The observed run achieved:

- 1,000 successfully matched PONG responses.
- No invalid PONG payloads.
- No unexpected sequence numbers.
- No transmission errors.
- No timeouts.

The average RTT was approximately 13.732 ms.

The maximum RTT was significantly higher than the P99, indicating occasional latency spikes.

## 4. Interpretation

### Packet delivery

The measured application-level success rate was:

\[
SuccessRate =
\frac{N_{\text{matched PONG}}}
{N_{\text{TX attempts}}}
\times 100
\]

For the observed run:

\[
SuccessRate =
\frac{1000}{1000}\times100
=100\%
\]

No timeout or invalid response was observed.

This result applies to this specific run and should not be interpreted as a guarantee of 100% delivery under all operating conditions.

### Latency

The average RTT was approximately 13.732 ms.

The median RTT was approximately 12.850 ms.

The difference between the average and median indicates that occasional slower exchanges influenced the average.

The maximum RTT of 196.103 ms is an important observation and should not be discarded when evaluating latency stability.

### Percentiles

The reported percentiles provide additional information about latency distribution.

| Percentile | Interpretation |
|---|---|
| P50 | Median RTT |
| P95 | 95% of observations are at or below this value |
| P99 | 99% of observations are at or below this value |

The reported percentile values are approximate values from the experimental output.

## 5. Benchmark limitations

The current experiment has several limitations.

### Measurement scope

The RTT measurement includes software and radio processing on both devices.

It is not a pure PHY-layer latency measurement.

### Hardware dependence

Results can vary according to:

- Physical distance between boards
- Antenna orientation
- RF interference
- Wi-Fi channel occupancy
- Firmware versions
- CPU workload
- Interrupt scheduling
- RTOS scheduling
- Radio power configuration

### Sample size

The documented execution contains 1,000 transmissions.

A 10,000-packet benchmark remains a separate validation target.

### TX submission versus delivery

A successful TX submission only confirms that the local API accepted the transmission operation.

A matching PONG provides application-level evidence that the request reached the responder and that its response returned successfully.

## 6. Reproducibility

For comparable measurements, keep the following parameters consistent:

| Parameter | Recommended setting |
|---|---|
| Wi-Fi channel | 1 |
| Payload size | 8 bytes |
| PING interval | 1,000 ms |
| PONG timeout | 500 ms |
| RX polling interval | 10 ms |
| Sequence size | 32-bit |
| Measurement unit | Microseconds |

Record the following information for each execution:

- Board revisions
- Firmware commit hashes
- Rust toolchain version
- Crate versions
- Wi-Fi channel
- Number of transmission attempts
- Total execution time
- RTT metrics
- Error counters

## 7. Future experiments

The following experiments can extend the current benchmark.

| Experiment | Objective |
|---|---|
| 10,000 packets | Larger sample size |
| 100,000 packets | Long-duration reliability |
| Different payload sizes | Payload-size impact |
| Different distances | Range and signal quality |
| Different channels | RF interference impact |
| Different transmission intervals | Throughput and congestion |
| CPU load variation | Scheduling impact |
| Repeated runs | Statistical reproducibility |

These are proposed experiments, not completed measurements.

## 8. Conclusion

The initial experiment demonstrates successful bidirectional ESP-NOW communication between two independent RISC-V embedded platforms.

The tested implementation achieved:

- 1,000 matched PONG responses.
- 100% application-level success rate during the observed run.
- Zero transmission errors.
- Zero invalid responses.
- Zero timeouts.
- 13.732 ms average round-trip time.

The experiment validates the communication architecture and provides a baseline for future performance and reliability measurements.

---

**Measurement status:** Initial experimental validation 1,000 packets.

**Platform:** Raspberry Pi Pico 2 W (RP2350 / CYW43439) ↔ ESP32-C3 Mini.