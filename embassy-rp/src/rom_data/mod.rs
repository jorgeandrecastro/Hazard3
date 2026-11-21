#![cfg_attr(
    feature = "_rp235x",
    doc = r"
Functions and data from the RPI Bootrom.

From [Section 5.4](https://rptl.io/rp2350-datasheet#section_bootrom) of the
RP2350 datasheet:

> Whilst some ROM space is dedicated to the implementation of the boot
> sequence and USB/UART boot interfaces, the bootrom also contains public
> functions that provide useful RP2350 functionality that may be useful for
> any code or runtime running on the device
"
)]

#[cfg_attr(feature = "_rp235x", path = "rp235x.rs")]
mod inner;
pub use inner::*;
