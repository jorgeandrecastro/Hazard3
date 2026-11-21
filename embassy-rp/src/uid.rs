//! Unique ID (UID)
//!
//! The ID is read once, on first use, and cached.
//!
//! The chip ID is read from OTP, the same source used by the bootrom USB serial number.
//!
//! Infallible, so if the ID cannot be read, return a fixed value (`0xEE`), also as per `pico-sdk`.

use embassy_sync::lazy_lock::LazyLock;

const UNAVAILABLE: u8 = 0xEE;

struct Uid {
    bytes: [u8; 8],
    hex: [u8; 16],
}

static UID: LazyLock<Uid> = LazyLock::new(|| {
    let bytes = read_uid();

    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut hex = [0u8; 16];
    for (idx, v) in bytes.iter().enumerate() {
        let lo = v & 0x0f;
        let hi = (v & 0xf0) >> 4;
        hex[idx * 2] = HEX[hi as usize];
        hex[idx * 2 + 1] = HEX[lo as usize];
    }

    Uid { bytes, hex }
});

#[cfg(feature = "_rp235x")]
fn read_uid() -> [u8; 8] {
    match crate::otp::get_chipid() {
        Ok(id) => id.to_be_bytes(),
        Err(_) => [UNAVAILABLE; 8],
    }
}

/// Get this device's unique 64-bit ID.
pub fn uid() -> &'static [u8; 8] {
    &UID.get().bytes
}

/// Get this device's unique 64-bit ID as a string of 16 hexadecimal ASCII digits.
pub fn uid_hex() -> &'static str {
    unsafe { core::str::from_utf8_unchecked(uid_hex_bytes()) }
}

/// Get this device's unique 64-bit ID, encoded into 16 hexadecimal ASCII bytes.
pub fn uid_hex_bytes() -> &'static [u8; 16] {
    &UID.get().hex
}
