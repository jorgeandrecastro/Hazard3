#![no_std]
#![allow(clippy::new_without_default)]
#![allow(unsafe_op_in_unsafe_fn)]
#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

// This mod MUST go first, so that the others see its macros.
pub(crate) mod fmt;

pub mod atomic_ring_buffer;
mod macros;
mod peripheral;
pub use peripheral::{Peri, PeripheralType};

#[cfg(all(feature = "cortex-m", feature = "riscv"))]
compile_error!("features `cortex-m` and `riscv` of embassy-hal-internal are mutually exclusive");

#[cfg(feature = "cortex-m")]
pub mod interrupt;

#[cfg(feature = "riscv")]
#[path = "interrupt_riscv.rs"]
pub mod interrupt;