
#![no_std]
#![allow(non_camel_case_types)]
#![doc = include_str!("../README.md")]

#[cfg(not(feature = "rp235x"))]
compile_error!("You must enable the `rp235x` Cargo feature.");

#[cfg(feature = "rp235x")]
#[path = "./rp235x/mod.rs"]
mod inner;

#[cfg(feature = "rp235x")]
pub use inner::*;
