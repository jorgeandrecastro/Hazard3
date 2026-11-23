
#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::bind_interrupts;
use embassy_rp::dma;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::i2c::{
    Config as I2cConfig,
    I2c,
    InterruptHandler as I2cInterruptHandler,
};
use embassy_rp::mode::Async;
use embassy_rp::peripherals::{DMA_CH0, DMA_CH1, I2C0, PIO0};
use embassy_rp::pio::{
    InterruptHandler as PioInterruptHandler,
    Pio,
};
use embassy_rp::time::Hertz;

use embassy_time::{Duration, Ticker};
use embassy_futures::select::{select, Either};

use cyw43_pio::{PioSpi, DEFAULT_CLOCK_DIVIDER};
use cyw43_setup::{CLM, FW, NVRAM};

use embassy_ssd1306::Ssd1306;
use static_cell::StaticCell;

use rp2350_riscv_linker as _;
use defmt_rtt as _;
use panic_halt as _;

// ============================================================
// CONFIGURATION
// ============================================================

const CHANNEL: u8 = 1;
const OLED_REFRESH_MS: u64 = 4000;

const ESP_NOW_CATEGORY: u8 = 127;
const ESP_NOW_ELEMENT_ID: u8 = 221;
const ESP_NOW_TYPE: u8 = 0x04;

const OUI: [u8; 3] = [0x18, 0xFE, 0x34];

const BROADCAST: [u8; 6] = [0xFF; 6];

const PAGE_COUNT: u8 = 5;

// ============================================================
// INTERRUPTS
// ============================================================

bind_interrupts!(struct Irqs {
    I2C0_IRQ => I2cInterruptHandler<I2C0>;

    PIO0_IRQ_0 => PioInterruptHandler<PIO0>;

    DMA_IRQ_0 =>
        dma::InterruptHandler<DMA_CH0>,
        dma::InterruptHandler<DMA_CH1>;
});

// ============================================================
// CYW43 RUNNER
// ============================================================

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<
        'static,
        cyw43::SpiBus<
            Output<'static>,
            PioSpi<'static, PIO0, 0>,
        >,
        cyw43::Cyw43439,
    >,
) -> ! {
    runner.run().await
}

// ============================================================
// FRAME BUILDER
// ============================================================

fn build_esp_now_frame(
    output: &mut [u8; 265],
    version: u8,
    payload: &[u8],
) -> Option<usize> {

    if payload.len() > 250 {
        return None;
    }

    output[0] = ESP_NOW_CATEGORY;

    output[1..4].copy_from_slice(&OUI);

    output[4..8].copy_from_slice(&[
        0x12, 0x34, 0x56, 0x78,
    ]);

    output[8] = ESP_NOW_ELEMENT_ID;

    output[9] = (5 + payload.len()) as u8;

    output[10..13].copy_from_slice(&OUI);

    output[13] = ESP_NOW_TYPE;

    output[14] = version;

    output[15..15 + payload.len()]
        .copy_from_slice(payload);

    Some(15 + payload.len())
}

// ============================================================
// FRAME PARSER
//
// ESP-NOW body:
// [0]       Category
// [1..4]    OUI
// [4..8]    Random
// [8]       Element ID
// [9]       Length
// [10..13]  OUI
// [13]      Type
// [14]      Version
// [15..19]  PING
// [19..23]  Sequence
// ============================================================

fn parse_ping(data: &[u8]) -> Option<(u8, u32)> {

    if data.len() < 23 {
        return None;
    }

    if data[0] != ESP_NOW_CATEGORY {
        return None;
    }

    if data[1..4] != OUI {
        return None;
    }

    if data[8] != ESP_NOW_ELEMENT_ID {
        return None;
    }

    if data[10..13] != OUI {
        return None;
    }

    if data[13] != ESP_NOW_TYPE {
        return None;
    }

    let version = data[14];

    let element_len = data[9] as usize;

    if element_len < 13 {
        return None;
    }

    let element_end = 10 + element_len;

    if element_end > data.len() {
        return None;
    }

    let payload = &data[15..element_end];

    if payload.len() < 8 {
        return None;
    }

    if &payload[0..4] != b"PING" {
        return None;
    }

    let sequence = u32::from_le_bytes([
        payload[4],
        payload[5],
        payload[6],
        payload[7],
    ]);

    Some((version, sequence))
}

// ============================================================
// INTEGER FORMAT
// ============================================================

fn format_u32(mut value: u32) -> ([u8; 10], usize) {

    let mut buffer = [0u8; 10];

    if value == 0 {
        buffer[0] = b'0';
        return (buffer, 1);
    }

    let mut pos = 10;

    while value > 0 {
        pos -= 1;
        buffer[pos] = b'0' + (value % 10) as u8;
        value /= 10;
    }

    let len = 10 - pos;

    let mut result = [0u8; 10];

    result[..len].copy_from_slice(&buffer[pos..]);

    (result, len)
}

fn format_i32(value: i32) -> ([u8; 12], usize) {

    let mut result = [0u8; 12];

    let negative = value < 0;

    let (digits, len) = format_u32(value.unsigned_abs());

    let offset = if negative {
        result[0] = b'-';
        1
    } else {
        0
    };

    result[offset..offset + len]
        .copy_from_slice(&digits[..len]);

    (result, offset + len)
}

// ============================================================
// MAC FORMAT
// ============================================================

fn format_mac(mac: [u8; 6]) -> [u8; 17] {

    const HEX: &[u8; 16] = b"0123456789ABCDEF";

    let mut output = [0u8; 17];

    let mut pos = 0;

    for (i, byte) in mac.iter().enumerate() {

        output[pos] = HEX[(byte >> 4) as usize];
        output[pos + 1] = HEX[(byte & 0x0F) as usize];

        pos += 2;

        if i != 5 {
            output[pos] = b':';
            pos += 1;
        }
    }

    output
}

// ============================================================
// DEBUG STATE
// ============================================================

struct Statistics {
    rx_count: u32,
    valid_ping_count: u32,
    invalid_frame_count: u32,
    tx_count: u32,
    tx_error_count: u32,

    last_rssi: i32,
    last_len: usize,
    last_channel: u8,
    last_mac: [u8; 6],
    last_sequence: u32,

    last_parse_ok: bool,
    last_tx_ok: bool,
}

impl Statistics {
    const fn new() -> Self {
        Self {
            rx_count: 0,
            valid_ping_count: 0,
            invalid_frame_count: 0,
            tx_count: 0,
            tx_error_count: 0,

            last_rssi: 0,
            last_len: 0,
            last_channel: CHANNEL,
            last_mac: [0; 6],
            last_sequence: 0,

            last_parse_ok: false,
            last_tx_ok: false,
        }
    }
}

// ============================================================
// OLED RENDERER
// ============================================================

async fn update_oled(
    oled: &mut Ssd1306<I2c<'static, Async>>,
    page: u8,
    stats: &Statistics,
) {

    oled.clear();

    match page {

        // ====================================================
        // PAGE 1 - SYSTEM
        // ====================================================

        0 => {

            oled.draw_str(0, 0, b"ESP-NOW MONITOR");
            oled.draw_str(0, 1, b"----------------");

            oled.draw_str(0, 2, b"CYW43439: READY");
            oled.draw_str(0, 3, b"RP2350: READY");

            oled.draw_str(0, 4, b"RADIO: ACTIVE");
            oled.draw_str(0, 5, b"CHANNEL: 1");

            oled.draw_str(0, 6, b"MODE: RX/TX");
            oled.draw_str(0, 7, b"SYSTEM OK");
        }

        // ====================================================
        // PAGE 2 - STATISTICS
        // ====================================================

        1 => {

            oled.draw_str(0, 0, b"LINK STATISTICS");
            oled.draw_str(0, 1, b"----------------");

            oled.draw_str(0, 2, b"RX PACKETS:");

            let (rx, len) = format_u32(stats.rx_count);
            oled.draw_str(85, 2, &rx[..len]);

            oled.draw_str(0, 3, b"VALID PING:");

            let (valid, len) = format_u32(stats.valid_ping_count);
            oled.draw_str(85, 3, &valid[..len]);

            oled.draw_str(0, 4, b"INVALID:");

            let (invalid, len) = format_u32(stats.invalid_frame_count);
            oled.draw_str(85, 4, &invalid[..len]);

            oled.draw_str(0, 5, b"TX REPLIES:");

            let (tx, len) = format_u32(stats.tx_count);
            oled.draw_str(85, 5, &tx[..len]);

            oled.draw_str(0, 6, b"TX ERRORS:");

            let (errors, len) = format_u32(stats.tx_error_count);
            oled.draw_str(85, 6, &errors[..len]);

            if stats.rx_count > 0 {
                oled.draw_str(0, 7, b"LINK: ACTIVE");
            } else {
                oled.draw_str(0, 7, b"LINK: WAITING");
            }
        }

        // ====================================================
        // PAGE 3 - LAST FRAME
        // ====================================================

        2 => {

            oled.draw_str(0, 0, b"LAST FRAME");
            oled.draw_str(0, 1, b"----------------");

            oled.draw_str(0, 2, b"RSSI:");

            let (rssi, len) = format_i32(stats.last_rssi);

            oled.draw_str(55, 2, &rssi[..len]);
            oled.draw_str(100, 2, b"dBm");

            oled.draw_str(0, 3, b"LENGTH:");

            let (size, len) = format_u32(stats.last_len as u32);

            oled.draw_str(65, 3, &size[..len]);
            oled.draw_str(95, 3, b"BYTES");

            oled.draw_str(0, 5, b"CHANNEL:");

            let (channel, len) = format_u32(stats.last_channel as u32);
            oled.draw_str(75, 5, &channel[..len]);

            oled.draw_str(0, 6, b"SEQUENCE:");

            let (seq, len) = format_u32(stats.last_sequence);
            oled.draw_str(75, 6, &seq[..len]);

            if stats.last_parse_ok {
                oled.draw_str(0, 7, b"PARSE: VALID");
            } else {
                oled.draw_str(0, 7, b"PARSE: INVALID");
            }
        }

        // ====================================================
        // PAGE 4 - SOURCE MAC
        // ====================================================

        3 => {

            oled.draw_str(0, 0, b"LAST TX SOURCE");
            oled.draw_str(0, 1, b"----------------");

            let mac = format_mac(stats.last_mac);

            oled.draw_str(0, 3, &mac[0..8]);
            oled.draw_str(0, 4, &mac[9..17]);

            oled.draw_str(0, 6, b"ESP32-C3");

            if stats.rx_count > 0 {
                oled.draw_str(0, 7, b"LINK: RX OK");
            } else {
                oled.draw_str(0, 7, b"NO SOURCE");
            }
        }

        // ====================================================
        // PAGE 5 - DEBUG
        // ====================================================

        _ => {

            oled.draw_str(0, 0, b"ESP-NOW DEBUG");
            oled.draw_str(0, 1, b"----------------");

            oled.draw_str(0, 2, b"VALID PING:");

            let (valid, len) = format_u32(stats.valid_ping_count);
            oled.draw_str(85, 2, &valid[..len]);

            oled.draw_str(0, 3, b"INVALID:");

            let (invalid, len) = format_u32(stats.invalid_frame_count);
            oled.draw_str(85, 3, &invalid[..len]);

            oled.draw_str(0, 4, b"TX ERRORS:");

            let (errors, len) = format_u32(stats.tx_error_count);
            oled.draw_str(85, 4, &errors[..len]);

            oled.draw_str(0, 5, b"PARSE:");

            if stats.last_parse_ok {
                oled.draw_str(55, 5, b"VALID");
            } else {
                oled.draw_str(55, 5, b"INVALID");
            }

            oled.draw_str(0, 6, b"LAST TX:");

            if stats.last_tx_ok {
                oled.draw_str(55, 6, b"SUCCESS");
            } else {
                oled.draw_str(55, 6, b"FAILED");
            }

            oled.draw_str(0, 7, b"DIAGNOSTICS");
        }
    }

    oled.flush()
        .await
        .expect("OLED flush failed");
}

// ============================================================
// MAIN
// ============================================================

#[embassy_executor::main]
async fn main(spawner: Spawner) {

    defmt::info!("================================");
    defmt::info!("PICO 2 W - ESP-NOW BIDIRECTIONAL");
    defmt::info!("================================");

    // --------------------------------------------------------
    // RP2350
    // --------------------------------------------------------

    let p = embassy_rp::init(Default::default());

    // --------------------------------------------------------
    // CYW43439
    // --------------------------------------------------------

    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);

    let mut pio = Pio::new(p.PIO0, Irqs);

    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        DEFAULT_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        dma::Channel::new(p.DMA_CH0, Irqs),
        dma::Channel::new(p.DMA_CH1, Irqs),
    );

    static STATE: StaticCell<cyw43::State> = StaticCell::new();

    let state = STATE.init(cyw43::State::new());

    let (_net_device, mut control, runner) = cyw43::new(
        state,
        pwr,
        spi,
        &FW,
        &NVRAM,
    )
    .await;

    spawner.spawn(
        cyw43_task(runner)
            .expect("CYW43 task spawn failed"),
    );

    control.init(CLM, cyw43::Config::default()).await;

    defmt::info!("CYW43439 READY");

    // --------------------------------------------------------
    // OLED
    // --------------------------------------------------------

    let mut i2c_config = I2cConfig::default();
    i2c_config.frequency = Hertz(400_000);

    let i2c = I2c::new(
        p.I2C0,
        p.PIN_5,
        p.PIN_4,
        Irqs,
        i2c_config,
    );

    let mut oled = Ssd1306::new(i2c, 0x3C);

    oled.init().await.expect("OLED init failed");

    oled.clear();
    oled.draw_str(0, 0, b"ESP-NOW MONITOR");
    oled.draw_str(0, 2, b"INITIALIZING...");
    oled.flush().await.expect("OLED flush failed");

    // --------------------------------------------------------
    // RADIO
    // --------------------------------------------------------

    control.set_channel(CHANNEL).await;

    defmt::info!("RADIO CHANNEL: {}", CHANNEL);

    // --------------------------------------------------------
    // ACTION FRAME RECEIVER
    // --------------------------------------------------------

    let mut receiver = control.listen_action_frames();

    defmt::info!("ACTION FRAME RX ENABLED");
    defmt::info!("WAITING FOR ESP32-C3");

    // --------------------------------------------------------
    // STATISTICS
    // --------------------------------------------------------

    let mut stats = Statistics::new();

    let mut page: u8 = 0;

    let mut ticker = Ticker::every(
        Duration::from_millis(OLED_REFRESH_MS)
    );

    update_oled(&mut oled, page, &stats).await;

    // --------------------------------------------------------
    // MAIN EVENT LOOP
    // --------------------------------------------------------

    loop {

        match select(
            receiver.next(),
            ticker.next(),
        ).await {

            // ================================================
            // RX EVENT
            // ================================================

            Either::First(Some(frame)) => {

                stats.rx_count = stats.rx_count.saturating_add(1);

                stats.last_rssi = frame.rssi;
                stats.last_len = frame.data().len();
                stats.last_channel = frame.channel;
                stats.last_mac = frame.src;

                defmt::info!("================================");
                defmt::info!("RX PACKET: {}", stats.rx_count);

                defmt::info!(
                    "SOURCE: {:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                    frame.src[0],
                    frame.src[1],
                    frame.src[2],
                    frame.src[3],
                    frame.src[4],
                    frame.src[5]
                );

                defmt::info!("RSSI: {} dBm", frame.rssi);
                defmt::info!("CHANNEL: {}", frame.channel);
                defmt::info!("LENGTH: {}", frame.data().len());
                defmt::info!("TRUNCATED: {}", frame.truncated);

                defmt::info!(
                    "BODY: {=[u8]:02x}",
                    frame.data()
                );

                // --------------------------------------------
                // PARSE PING
                // --------------------------------------------

                if let Some((version, sequence)) =
                    parse_ping(frame.data())
                {

                    stats.valid_ping_count =
                        stats.valid_ping_count.saturating_add(1);

                    stats.last_parse_ok = true;
                    stats.last_sequence = sequence;

                    defmt::info!("VALID ESP-NOW PING");
                    defmt::info!("SEQUENCE: {}", sequence);

                    // ----------------------------------------
                    // BUILD PONG
                    // ----------------------------------------

                    let mut payload = [0u8; 8];

                    payload[0..4].copy_from_slice(b"PONG");

                    payload[4..8].copy_from_slice(
                        &sequence.to_le_bytes()
                    );

                    let mut response = [0u8; 265];

                    if let Some(response_len) =
                        build_esp_now_frame(
                            &mut response,
                            version,
                            &payload,
                        )
                    {

                        // Broadcast response for initial test.

                        match control.send_action_frame(
                            BROADCAST,
                            BROADCAST,
                            CHANNEL,
                            &response[..response_len],
                        ).await {

                            Ok(()) => {

                                stats.tx_count =
                                    stats.tx_count.saturating_add(1);

                                stats.last_tx_ok = true;

                                defmt::info!("PONG SUBMITTED");
                                defmt::info!("SEQUENCE: {}", sequence);
                            }

                            Err(error) => {

                                stats.tx_error_count =
                                    stats.tx_error_count.saturating_add(1);

                                stats.last_tx_ok = false;

                                defmt::error!(
                                    "PONG TX ERROR: {:?}",
                                    error
                                );
                            }
                        }
                    }

                } else {

                    stats.invalid_frame_count =
                        stats.invalid_frame_count.saturating_add(1);

                    stats.last_parse_ok = false;

                    defmt::warn!("INVALID ACTION FRAME");

                    // Show first bytes for diagnosis.
                    let data = frame.data();

                    if data.len() >= 4 {
                        defmt::info!(
                            "HEADER: {:02X} {:02X} {:02X} {:02X}",
                            data[0], data[1], data[2], data[3]
                        );
                    }
                }

                // --------------------------------------------
                // LED HEARTBEAT
                // --------------------------------------------

                control.gpio_set(0, true).await;
                control.gpio_set(0, false).await;
            }

            // ================================================
            // OLED REFRESH
            // ================================================

            Either::Second(_) => {

                page = (page + 1) % PAGE_COUNT;

                defmt::info!(
                    "OLED PAGE: {}",
                    page
                );

                update_oled(
                    &mut oled,
                    page,
                    &stats,
                ).await;
            }

            // ================================================
            // RECEIVER CLOSED
            // ================================================

            Either::First(None) => {

                defmt::error!("ACTION FRAME RECEIVER CLOSED");

                oled.clear();
                oled.draw_str(0, 0, b"RX ERROR");
                oled.draw_str(0, 2, b"RECEIVER CLOSED");

                oled.flush().await.expect("OLED flush failed");

                break;
            }
        }
    }
}
