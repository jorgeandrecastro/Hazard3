# hazard3-net-driver-channel

> **Published replacement:** `hazard3-net-driver-channel` replaces the formerly published `embassy-net-driver-channel-riscv`.
> See the project repository for the publication status of the Hazard3 packages.
> Upstream license and attribution notices are retained.

This fork uses `hazard3-sync` and `hazard3-net-driver` to integrate
drivers such as [`hazard3-cyw43`](../hazard3-cyw43/) with
[`hazard3-net-async`](../hazard3-net-async/). The package name is
`hazard3-net-driver-channel`, and the Rust library name is
`hazard3_net_driver_channel`. The license remains MIT OR Apache-2.0.

This crate is derived from [Embassy's `embassy-net-driver-channel`](https://github.com/embassy-rs/embassy/tree/main/embassy-net-driver-channel).
All credit for the original work goes to Embassy and its contributors. This
independent fork focuses on RISC-V and is not affiliated with or endorsed by Embassy.

This crate provides a higher-level toolkit for implementing network drivers
used by `hazard3-net-async`. It sits on top of the forked
[`hazard3-net-driver`](../hazard3-net/hazard3-net-driver/) `Driver` trait.

The `hazard3_net_driver::Driver` trait is polling-based. To implement it directly,
you must write the packet receive/transmit state machines by hand and hook up
the `Waker`s provided by the network stack to the right interrupt handlers.

With `hazard3_net_driver_channel` you get a channel-like interface instead.
The intended usage is to spawn a driver task in the background, passing
packets between the hardware and the channels shared with `hazard3-net-async`.

## Migration to 0.2.0

The Rust library is now `hazard3_net_driver_channel` instead of `embassy_net_driver_channel`. Update imports and macro paths; this is a source-breaking change. Add the package directly without a `package` alias:

```toml
[dependencies]
hazard3-net-driver-channel = "0.2"
```

Use compatible new versions of the other Hazard3 crates together. These versions are pending publication; older published versions may use different library names.

## A note about deadlocks

When implementing a driver using this crate, it might be tempting to write it in the most straightforward way:

```rust,ignore
loop {
    // Wait for either..
    match select(
        // ... the chip signaling an interrupt, indicating a packet is available to receive, or
        irq_pin.wait_for_low(),
        // ... a packet to send appearing, i.e. the network stack wants to send a packet
        tx_chan.tx(),
    ).await {
        Either::First(_) => {
            // a packet is ready to be received!
            let mut buf = rx_chan.rx_buf().await; // wait for an empty buffer from the stack
            buf.set_len(buf.capacity());
            let n = receive_packet_over_spi(&mut buf).await;
            buf.set_len(n);
            rx_chan.rx(buf).await;
        }
        Either::Second(buf) => {
            // a packet is ready to be sent!
            send_packet_over_spi(&buf).await;
        }
    }
}
```

However, this code has a latent deadlock bug. The symptom is it can hang at `rx_chan.rx_buf().await` under load.

The reason is that, under load, both the TX and RX queues can get full at the
same time. When this happens, the network stack task stalls trying to send
because the TX queue is full, therefore it stops processing packets in the RX
queue and stops handing out empty buffers. Your driver task also stalls waiting
for an empty buffer, therefore it stops processing packets in the TX queue.

The fix is to make sure to always service the TX queue while you're waiting for an empty buffer. For example, select on either "a packet to send is available" or "INT is low AND there is an empty buffer":

```rust,ignore
loop {
    // Wait for either..
    match select(
        async {
            // ... the chip signaling an interrupt, indicating a packet is available to receive
            irq_pin.wait_for_low().await;
            // *AND* an empty buffer to receive it into...
            rx_chan.rx_buf().await
        },
        // ... or a packet to send appearing, i.e. the network stack wants to send a packet
        tx_chan.tx(),
    ).await {
        Either::First(mut buf) => {
            // a packet is ready to be received!
            buf.set_len(buf.capacity());
            let n = receive_packet_over_spi(&mut buf).await;
            buf.set_len(n);
            rx_chan.rx(buf).await;
        }
        Either::Second(buf) => {
            // a packet is ready to be sent!
            send_packet_over_spi(&buf).await;
        }
    }
}
```

## Examples

The RISC-V driver in this repository that uses this crate is:

- [`hazard3-cyw43`](../hazard3-cyw43/) for WiFi and Bluetooth on CYW43xx chips, including
  RP2350 support through [`hazard3-cyw43-pio`](../hazard3-cyw43-pio/).

The upstream Embassy repository contains additional channel-based drivers for
other hardware, but they are not maintained or necessarily compilable targets
of this RISC-V fork.

## Dependency selection

Use the Hazard3 packages together. In `Cargo.toml`, select this crate and the
forked network stack directly:

```toml
[dependencies]
hazard3-net-async = { version = "0.6" }
hazard3-net-driver-channel = { version = "0.2" }
hazard3-sync = { version = "0.2" }
hazard3-net-driver = { version = "0.2" }
```

The Rust crate names are `hazard3_net_async`, `hazard3_net_driver_channel`,
`hazard3_sync`, and `hazard3_net_driver`. Do not mix these packages with incompatible
copies of upstream `embassy-net`, `embassy-sync`, or `xarxa-driver`.

## Interoperability

This crate can run with executors supported by the RISC-V fork and does not
depend on a particular executor implementation.

## Development

Install the RISC-V target if needed:

```sh
rustup target add riscv32imac-unknown-none-elf
```

Run these commands from the `hazard3-net-driver-channel` directory to check
and document the crate for RISC-V:

```sh
cargo check --target riscv32imac-unknown-none-elf
```

Generate the documentation:

```sh
cargo doc \
    --target riscv32imac-unknown-none-elf \
    --no-deps \
    --document-private-items
```
