#![cfg_attr(feature = "nightly", feature(impl_trait_in_assoc_type))]

struct Foo<'a>(&'a ());

#[hazard3_executor::task]
async extern "C" fn task() {}

fn main() {}
