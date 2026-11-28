#![cfg_attr(feature = "nightly", feature(impl_trait_in_assoc_type))]

#[hazard3_executor::task]
async fn foo(_x: &u32) {}

fn main() {}
