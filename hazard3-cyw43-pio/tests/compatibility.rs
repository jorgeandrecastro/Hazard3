#![no_std]
#![no_main]

use core::panic::PanicInfo;

use cyw43::SpiBusCyw43;
use cyw43_pio::PioSpi;
use embassy_net::driver::Driver;
use embassy_rp::peripherals::PIO0;

fn assert_spi_bus<T: SpiBusCyw43>() {}
fn assert_network_driver<T: Driver>() {}

#[unsafe(no_mangle)]
pub extern "C" fn main() {
    assert_spi_bus::<PioSpi<'static, PIO0, 0>>();
    assert_network_driver::<cyw43::NetDriver<'static>>();
}

#[panic_handler]
fn panic(_info: &PanicInfo<'_>) -> ! {
    loop {}
}
