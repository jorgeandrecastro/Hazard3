# hazard3-futures

Utilities for working with futures, compatible with `no_std` and not using `alloc`.
Optimized for code size and suitable for embedded systems.

This crate is a fork of [`embassy-futures`](https://github.com/embassy-rs/embassy/tree/main/embassy-futures)
maintained as part of the [Hazard3](https://github.com/jorgeandrecastro/Hazard3) ecosystem.

- Future combinators, like [`join`](join) and [`select`](select)
- Utilities to use `async` without a fully fledged executor: [`block_on`](block_on::block_on) and [`yield_now`](yield_now::yield_now).

## Migration

Replace the `embassy-futures` dependency with the new package and update Rust
imports from `embassy_futures` to `hazard3_futures`:

```toml
[dependencies]
hazard3-futures = "0.1"
```

Version 0.1.0 is the first planned release of this fork; check crates.io for
availability. The `defmt` and `log` features remain available.

## Interoperability

Futures from this crate can run on any executor.

## Acknowledgements

Based on the Embassy project, copyright Embassy project contributors, under the
MIT OR Apache-2.0 licenses. The original license notices are retained in
`LICENSE-MIT` and `LICENSE-APACHE`. This fork is not affiliated with or
endorsed by Embassy.
