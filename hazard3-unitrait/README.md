# hazard3-unitrait

> **Migration in progress:** `hazard3-unitrait` replaces the formerly published `unitrait-riscv`.
> See the project repository for the publication status of the Hazard3 packages.
> Rust library names and upstream license and attribution notices are retained.

[![Crates.io](https://img.shields.io/crates/v/hazard3-unitrait.svg)](https://crates.io/crates/hazard3-unitrait)
[![docs.rs](https://docs.rs/hazard3-unitrait/badge.svg)](https://docs.rs/hazard3-unitrait)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20or%20Apache--2.0-blue.svg)](https://opensource.org/licenses/MIT)

> **Fork for RISC-V.** `hazard3-unitrait` is an independent, **unmodified copy** of `unitrait` (unreleased 2.0.0 revision `bb96601` of [embassy-rs/unitrait](https://github.com/embassy-rs/unitrait)),
> maintained by [Jorge Andre Castro](https://github.com/jorgeandrecastro) at <https://github.com/jorgeandrecastro/embassy-riscv>.
> It was originally republished under another name because [`hazard3-rp`](https://crates.io/crates/hazard3-rp) needs these unreleased changes and crates.io does not accept git dependencies.
> The library name is still `unitrait`; use it with `package = "hazard3-unitrait"` in your `Cargo.toml`.
> Use the related Hazard3 crates together to avoid duplicate/incompatible copies in your build.

Traits with a single global implementation, resolved at link time.

A "unitrait" is a trait that has exactly one implementation across the whole
crate tree. It can be called from anywhere in the tree without carrying
generic parameters or instances around, without `dyn`, and produces link-time
errors if there is no implementation or more than one.

This is achieved by dispatching calls through `extern "Rust"` functions: the
defining crate declares (and calls) functions by symbol name, and the
implementing crate exports them. The linker matches them up. Callers reach
them through a *dispatch type*, a unit struct with one method per trait method
that also implements the trait itself.

The use case is allowing pluggalbe "drivers" for foundational, process-wide facilities where generics would be too viral and `dyn` too costly. For example, it's used for the `embassy-time` driver and
the `embassy-executor` pender and trace hooks.

## Example

Define a unitrait in the crate that _calls_ the functionality:

```rust
unitrait::unitrait! {
    /// A driver for the frobnicator.
    pub trait Driver {
        /// Returns the current frobnication level.
        #[symbol = "_frob_level"]
        fn level() -> u32;
    }

    /// The global frobnicator.
    pub struct Frob;

    /// Set the global frobnicator driver.
    macro frob_driver_impl(path = $crate);
}
```

This expands to:

- The trait, exactly as written.
- The dispatch type `Frob`, a unit struct with one inherent method per trait
  method (here `Frob::level() -> u32`) that calls the global implementation
  through the extern symbol. `Frob` also implements `Driver`, so generic code
  written against the trait accepts it.
- A macro (here `frob_driver_impl!`) that implementor crates use to register
  a type as the global implementation.

Implement it in exactly one crate in the tree:

```rust,ignore
struct MyDriver;

impl frob::Driver for MyDriver {
    fn level() -> u32 { 42 }
}

frob::frob_driver_impl!(MyDriver);
```

Now `frob::Frob::level()` works from any crate in the tree. Since methods live
on the dispatch type, several unitraits in one module may share method names.

## Symbol names

Symbol names must be unique across the whole program, so prefix them with your
crate's name and version. Instead of writing each one out, a
`#[symbol_prefix = "..."]` attribute on the trait derives them all:

```rust,ignore
unitrait::unitrait! {
    #[symbol_prefix = "_frob_v1"]
    pub trait Driver {
        /// Uses `_frob_v1_level`.
        fn level() -> u32;

        /// Overridden, so it uses `_frob_legacy_reset`.
        #[symbol = "_frob_legacy_reset"]
        fn reset();
    }

    pub struct Frob;

    macro frob_driver_impl(path = $crate);
}
```

Methods derive `PREFIX_method_name`, and opaque associated types derive
`PREFIX_TypeName_drop` and `PREFIX_TypeName_clone`. A `#[symbol]`,
`#[drop_symbol]` or `#[clone_symbol]` attribute overrides one derived name.
Without a prefix, every symbol must be written out explicitly.

## Opaque associated types

A unitrait may declare _opaque_ associated types, letting callers hold
implementation-defined values and pass them back in, despite not knowing the
implementation's types:

```rust,ignore
unitrait::unitrait! {
    pub trait HashDriver {
        /// Opaque storage for the implementation's hash state.
        #[opaque(size = 128, align = 16)]
        #[drop_symbol = "_hash_context_drop"]
        pub type Context: Drop;

        #[symbol = "_hash_init"]
        fn init() -> Self::Context;

        #[symbol = "_hash_update"]
        fn update(ctx: &mut Self::Context, data: &[u8]);
    }

    pub struct Hash;

    macro hash_impl(path = $crate);
}
```

Callers see `HashContext` (named after the dispatch type plus the associated
type), an opaque type with the declared size and alignment, returned by
`Hash::init()`; the implementation sets `type Context` to its actual state
type, and the implementation macro checks at compile time that it fits. A trait may declare any number of opaque types, and
methods may use each as `Self::Name`, `&Self::Name`, `&mut Self::Name`,
`Pin<&Self::Name>` or `Pin<&mut Self::Name>` in any parameter, and return one
by value. These may also be nested by value inside `Option`, `Result`, tuples
and arrays, as in `Option<Self::Name>` or `Result<(Self::Name, u32), E>`; there,
`Option`, `Result` and `Pin` always mean the `core` types. Opaque values are only obtainable from methods returning one, so they
always hold initialized state — the dispatch methods are safe.

An opaque type with a `Drop` bound gets a `Drop` impl, which drops the
implementation's value in place through the extern symbol named by its
`#[drop_symbol = "..."]` attribute. One without has no drop glue at all, and the
implementation macro checks at compile time that the implementation's associated
type needs no dropping either.

Since the caller can't see what type the implementation picked, an opaque type
implements no auto trait by default. Marker traits are opted into by declaring
them as bounds on the associated type, which the compiler then enforces on the
implementation:

```rust,ignore
/// Sendable between threads, but not shareable.
#[opaque(size = 128, align = 16)]
#[drop_symbol = "_hash_context_drop"]
pub type Context: Send + Drop;
```

`Send`, `Sync`, `Unpin`, `UnwindSafe`, `RefUnwindSafe`, `Copy` and `Clone` are
supported, alongside `Drop`. `Copy` is mutually exclusive with both `Clone` and
`Drop`. Like `Drop`, `Clone` dispatches through a symbol of its own, named by a
`#[clone_symbol = "..."]` attribute, since only the implementation can duplicate
a value of a type the caller can't see.

See the documentation of the `unitrait!` macro for the full details.

## Acknowledgements

This crate is a derivative work of the [Embassy](https://embassy.dev) project ([embassy-rs](https://github.com/embassy-rs)),
Copyright (c) Embassy project contributors, used under the MIT OR Apache-2.0 licenses. The original copyright and license notices are
kept (see `LICENSE-MIT` and `LICENSE-APACHE`). Embassy and its authors are not affiliated with, and do not endorse, this fork.
Thank you to the Embassy team and contributors for their work.
