#![cfg_attr(feature = "nightly", feature(impl_trait_in_assoc_type))]

#[hazard3_executor::task]
async fn foo<'a>(_x: &'a u32) {}

fn main() {}
