#![cfg_attr(feature = "nightly", feature(impl_trait_in_assoc_type))]
#![deny(unused_unsafe)]

use std::mem;

#[hazard3_executor::task]
async fn safe() {}

#[hazard3_executor::task]
async unsafe fn not_safe() {}

#[cfg(not(all(feature = "platform-std", feature = "executor-thread")))]
#[unsafe(export_name = "__pender")]
fn pender(_: *mut ()) {
    // The test doesn't link if we don't include this.
    // We never call this anyway.
}

fn main() {
    let _forget_me = safe();
    // SAFETY: not_safe has not safety preconditions
    let _forget_me2 = unsafe { not_safe() };

    mem::forget(_forget_me);
    mem::forget(_forget_me2);
}
