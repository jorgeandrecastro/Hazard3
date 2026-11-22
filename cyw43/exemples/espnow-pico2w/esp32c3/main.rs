
#![no_std]
#![no_main]

extern crate alloc;

use embassy_executor::Spawner;
use embassy_time::{Duration, Instant, Timer};

use esp_alloc as _;
use esp_backtrace as _;
use esp_println::println;

use esp_hal::{
    clock::CpuClock,
    interrupt::software::SoftwareInterruptControl,
    ram,
    timer::timg::TimerGroup,
};

use esp_radio::esp_now::BROADCAST_ADDRESS;

esp_bootloader_esp_idf::esp_app_desc!();

// ============================================================
// CONFIGURATION
// ============================================================

const CHANNEL: u8 = 1;

const INTERVAL_MS: u64 = 1000;
const POLL_INTERVAL_MS: u64 = 10;
const PONG_TIMEOUT_MS: u64 = 500;

const PAYLOAD_SIZE: usize = 8;

const TEST_PACKETS: u32 = 10_000;
const REPORT_INTERVAL: u32 = 100;

// RTT histogram: 256 buckets x 100 us.
const RTT_BUCKET_WIDTH_US: u64 = 100;
const RTT_BUCKETS: usize = 256;

// ============================================================
// PAYLOAD BUILDER
//
// PING + sequence(u32 LE)
// ============================================================

fn build_ping(sequence: u32) -> [u8; PAYLOAD_SIZE] {
    let mut payload = [0u8; PAYLOAD_SIZE];

    payload[0..4].copy_from_slice(b"PING");
    payload[4..8].copy_from_slice(&sequence.to_le_bytes());

    payload
}

// ============================================================
// PONG PARSER
//
// PONG + sequence(u32 LE)
// ============================================================

fn parse_pong(data: &[u8]) -> Option<u32> {
    if data.len() != PAYLOAD_SIZE {
        return None;
    }

    if &data[0..4] != b"PONG" {
        return None;
    }

    Some(u32::from_le_bytes([
        data[4],
        data[5],
        data[6],
        data[7],
    ]))
}

// ============================================================
// STATISTICS
// ============================================================

struct Statistics {
    tx_attempts: u32,
    tx_packets: u32,
    tx_errors: u32,

    rx_pongs: u32,
    invalid_pongs: u32,
    unexpected_pongs: u32,
    timeouts: u32,

    rtt_min_us: u64,
    rtt_max_us: u64,
    rtt_total_us: u64,

    rtt_histogram: [u32; RTT_BUCKETS],
}

impl Statistics {
    const fn new() -> Self {
        Self {
            tx_attempts: 0,
            tx_packets: 0,
            tx_errors: 0,

            rx_pongs: 0,
            invalid_pongs: 0,
            unexpected_pongs: 0,
            timeouts: 0,

            rtt_min_us: u64::MAX,
            rtt_max_us: 0,
            rtt_total_us: 0,

            rtt_histogram: [0; RTT_BUCKETS],
        }
    }

    fn record_rtt(&mut self, rtt_us: u64) {
        self.rx_pongs = self.rx_pongs.saturating_add(1);

        self.rtt_min_us = self.rtt_min_us.min(rtt_us);
        self.rtt_max_us = self.rtt_max_us.max(rtt_us);

        self.rtt_total_us =
            self.rtt_total_us.saturating_add(rtt_us);

        let bucket = (rtt_us / RTT_BUCKET_WIDTH_US) as usize;
        let bucket = bucket.min(RTT_BUCKETS - 1);

        self.rtt_histogram[bucket] =
            self.rtt_histogram[bucket].saturating_add(1);
    }

    fn average_rtt(&self) -> u64 {
        if self.rx_pongs == 0 {
            return 0;
        }

        self.rtt_total_us / self.rx_pongs as u64
    }

    // Approximate percentile using histogram.
    fn percentile(&self, percentile: u32) -> u64 {
        if self.rx_pongs == 0 {
            return 0;
        }

        let rank = (
            self.rx_pongs as u64 * percentile as u64 + 99
        ) / 100;

        let mut cumulative = 0u64;

        for (index, count) in self.rtt_histogram.iter().enumerate() {
            cumulative += *count as u64;

            if cumulative >= rank {
                return index as u64 * RTT_BUCKET_WIDTH_US
                    + RTT_BUCKET_WIDTH_US / 2;
            }
        }

        (RTT_BUCKETS as u64 - 1) * RTT_BUCKET_WIDTH_US
            + RTT_BUCKET_WIDTH_US / 2
    }

    fn loss_percent_x100(&self) -> u32 {
        if self.tx_packets == 0 {
            return 0;
        }

        (
            self.timeouts as u64 * 10_000
                / self.tx_packets as u64
        ) as u32
    }

    fn success_percent_x100(&self) -> u32 {
        if self.tx_packets == 0 {
            return 0;
        }

        (
            self.rx_pongs as u64 * 10_000
                / self.tx_packets as u64
        ) as u32
    }
}

// ============================================================
// MAC FORMAT
// ============================================================

fn print_mac(mac: [u8; 6]) {
    println!(
        "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
        mac[0],
        mac[1],
        mac[2],
        mac[3],
        mac[4],
        mac[5],
    );
}

// ============================================================
// STATISTICS REPORT
// ============================================================

fn print_statistics(stats: &Statistics) {
    println!();
    println!("================================");
    println!(" ESP-NOW LINK STATISTICS");
    println!("================================");

    println!(
        "[STAT] TX ATTEMPTS: {}",
        stats.tx_attempts
    );

    println!(
        "[STAT] TX SUBMITTED: {}",
        stats.tx_packets
    );

    println!(
        "[STAT] TX ERRORS: {}",
        stats.tx_errors
    );

    println!(
        "[STAT] RX MATCHED PONGS: {}",
        stats.rx_pongs
    );

    println!(
        "[STAT] INVALID PONGS: {}",
        stats.invalid_pongs
    );

    println!(
        "[STAT] UNEXPECTED PONGS: {}",
        stats.unexpected_pongs
    );

    println!(
        "[STAT] TIMEOUTS: {}",
        stats.timeouts
    );

    println!(
        "[STAT] SUCCESS RATE: {}.{}%",
        stats.success_percent_x100() / 100,
        stats.success_percent_x100() % 100
    );

    println!(
        "[STAT] TIMEOUT RATE: {}.{}%",
        stats.loss_percent_x100() / 100,
        stats.loss_percent_x100() % 100
    );

    if stats.rx_pongs > 0 {
        println!();
        println!("---------- RTT METRICS ----------");

        println!(
            "[RTT] MIN: {} us",
            stats.rtt_min_us
        );

        println!(
            "[RTT] MAX: {} us",
            stats.rtt_max_us
        );

        println!(
            "[RTT] AVG: {} us",
            stats.average_rtt()
        );

        println!(
            "[RTT] P50: ~{} us",
            stats.percentile(50)
        );

        println!(
            "[RTT] P95: ~{} us",
            stats.percentile(95)
        );

        println!(
            "[RTT] P99: ~{} us",
            stats.percentile(99)
        );
    } else {
        println!("[RTT] NO VALID MEASUREMENTS");
    }

    println!("================================");
}

// ============================================================
// MAIN
// ============================================================

#[esp_rtos::main]
async fn main(_spawner: Spawner) -> ! {
    esp_println::logger::init_logger_from_env();

    println!();
    println!("================================");
    println!(" ESP32-C3 MINI");
    println!(" ESP-NOW STRESS TEST");
    println!("================================");

    // --------------------------------------------------------
    // HAL
    // --------------------------------------------------------

    let config = esp_hal::Config::default()
        .with_cpu_clock(CpuClock::max());

    let peripherals = esp_hal::init(config);

    // --------------------------------------------------------
    // HEAP
    // --------------------------------------------------------

    esp_alloc::heap_allocator!(
        #[ram(reclaimed)]
        size: 64 * 1024
    );

    esp_alloc::heap_allocator!(
        size: 36 * 1024
    );

    println!("[INIT] HEAP READY");

    // --------------------------------------------------------
    // ESP-RTOS
    // --------------------------------------------------------

    let timg0 = TimerGroup::new(peripherals.TIMG0);

    let sw_int =
        SoftwareInterruptControl::new(
            peripherals.SW_INTERRUPT
        );

    esp_rtos::start(
        timg0.timer0,
        sw_int.software_interrupt0,
    );

    println!("[INIT] RTOS READY");

    // --------------------------------------------------------
    // WIFI RADIO
    // --------------------------------------------------------

    println!("[INIT] INITIALIZING WIFI...");

    let (_wifi_controller, interfaces) =
        esp_radio::wifi::new(
            peripherals.WIFI,
            Default::default(),
        )
        .expect("WiFi initialization failed");

    println!("[INIT] WIFI RADIO READY");

    // --------------------------------------------------------
    // ESP-NOW
    // --------------------------------------------------------

    let mut esp_now = interfaces.esp_now;

    esp_now
        .set_channel(CHANNEL)
        .expect("ESP-NOW channel configuration failed");

    let version = esp_now
        .version()
        .expect("ESP-NOW version query failed");

    println!("[INIT] ESP-NOW READY");
    println!("[INIT] VERSION: {}", version);
    println!("[INIT] CHANNEL: {}", CHANNEL);

    println!("[INIT] DESTINATION: BROADCAST");
    println!("[INIT] RX ENABLED");
    println!("[INIT] TX ENABLED");

    // --------------------------------------------------------
    // TEST CONFIGURATION
    // --------------------------------------------------------

    println!();
    println!("================================");
    println!(" STRESS TEST CONFIGURATION");
    println!("================================");

    println!("[TEST] PACKETS: {}", TEST_PACKETS);
    println!("[TEST] INTERVAL: {} ms", INTERVAL_MS);
    println!("[TEST] TIMEOUT: {} ms", PONG_TIMEOUT_MS);
    println!("[TEST] CHANNEL: {}", CHANNEL);

    // --------------------------------------------------------
    // STATISTICS
    // --------------------------------------------------------

    let mut stats = Statistics::new();

    let mut sequence: u32 = 0;

    let mut pending_sequence: Option<u32> = None;

    let mut pending_timestamp_us: u64 = 0;

    let test_start = Instant::now();

    // --------------------------------------------------------
    // MAIN TEST LOOP
    // --------------------------------------------------------

    while stats.tx_attempts < TEST_PACKETS {
        sequence = sequence.wrapping_add(1);

        stats.tx_attempts =
            stats.tx_attempts.saturating_add(1);

        let cycle_start = Instant::now();

        let payload = build_ping(sequence);

        // ================================================
        // TRANSMIT PING
        // ================================================

        let tx_timestamp = Instant::now().as_micros();

        match esp_now
            .send_async(
                &BROADCAST_ADDRESS,
                &payload,
            )
            .await
        {
            Ok(()) => {
                stats.tx_packets =
                    stats.tx_packets.saturating_add(1);

                pending_sequence = Some(sequence);
                pending_timestamp_us = tx_timestamp;

                if sequence == 1
                    || sequence % REPORT_INTERVAL == 0
                {
                    println!();
                    println!(
                        "[TX] PING #{}",
                        sequence
                    );

                    println!("[TX] SUBMITTED");
                }
            }

            Err(error) => {
                stats.tx_errors =
                    stats.tx_errors.saturating_add(1);

                pending_sequence = None;

                println!(
                    "[TX] ERROR: {:?}",
                    error
                );
            }
        }

        // ================================================
        // RECEIVE WINDOW
        // ================================================

        loop {
            let mut received_any = false;

            // --------------------------------------------
            // DRAIN RX QUEUE
            // --------------------------------------------

            while let Some(received) = esp_now.receive() {
                received_any = true;

                let data = received.data();

                let source = received.info.src_address;

                println!();
                println!("[RX] ESP-NOW FRAME");

                print_mac(source);

                println!(
                    "[RX] LENGTH: {}",
                    data.len()
                );

                // ----------------------------------------
                // PARSE PONG
                // ----------------------------------------

                match parse_pong(data) {
                    Some(received_sequence) => {
                        match pending_sequence {
                            Some(expected_sequence)
                                if received_sequence == expected_sequence =>
                            {
                                let now_us =
                                    Instant::now().as_micros();

                                let rtt_us =
                                    now_us.saturating_sub(
                                        pending_timestamp_us
                                    );

                                stats.record_rtt(rtt_us);

                                pending_sequence = None;

                                if sequence == 1
                                    || sequence % REPORT_INTERVAL == 0
                                {
                                    println!(
                                        "[RX] VALID PONG: {}",
                                        received_sequence
                                    );

                                    println!("[LINK] MATCHED");

                                    println!(
                                        "[RTT] {} us",
                                        rtt_us
                                    );
                                }
                            }

                            Some(expected_sequence) => {
                                stats.unexpected_pongs =
                                    stats.unexpected_pongs.saturating_add(1);

                                println!(
                                    "[LINK] SEQUENCE MISMATCH"
                                );

                                println!(
                                    "[LINK] EXPECTED: {}",
                                    expected_sequence
                                );

                                println!(
                                    "[LINK] RECEIVED: {}",
                                    received_sequence
                                );
                            }

                            None => {
                                stats.unexpected_pongs =
                                    stats.unexpected_pongs.saturating_add(1);

                                println!(
                                    "[LINK] UNEXPECTED PONG: {}",
                                    received_sequence
                                );
                            }
                        }
                    }

                    None => {
                        stats.invalid_pongs =
                            stats.invalid_pongs.saturating_add(1);

                        println!("[RX] INVALID PONG");
                    }
                }
            }

            // --------------------------------------------
            // TIMEOUT
            // --------------------------------------------

            if let Some(expected_sequence) = pending_sequence {
                let elapsed_us =
                    Instant::now()
                        .as_micros()
                        .saturating_sub(pending_timestamp_us);

                if elapsed_us >= PONG_TIMEOUT_MS * 1000 {
                    stats.timeouts =
                        stats.timeouts.saturating_add(1);

                    pending_sequence = None;

                    println!();
                    println!(
                        "[TIMEOUT] SEQUENCE: {}",
                        expected_sequence
                    );
                }
            }

            // --------------------------------------------
            // END OF CYCLE
            // --------------------------------------------

            let elapsed_ms =
                Instant::now()
                    .duration_since(cycle_start)
                    .as_millis();

            if elapsed_ms >= INTERVAL_MS {
                break;
            }

            if received_any {
                Timer::after_millis(1).await;
            } else {
                Timer::after(
                    Duration::from_millis(POLL_INTERVAL_MS)
                ).await;
            }
        }

        // ================================================
        // PERIODIC REPORT
        // ================================================

        if stats.tx_attempts % REPORT_INTERVAL == 0 {
            println!();

            println!(
                "========== PROGRESS: {}/{} ==========",
                stats.tx_attempts,
                TEST_PACKETS
            );

            print_statistics(&stats);

            let elapsed =
                Instant::now()
                    .duration_since(test_start)
                    .as_secs();

            println!(
                "[TEST] ELAPSED: {} seconds",
                elapsed
            );
        }
    }

    // ====================================================
    // FINAL REPORT
    // ====================================================

    println!();
    println!("################################");
    println!(" ESP-NOW STRESS TEST COMPLETED");
    println!("################################");

    print_statistics(&stats);

    let total_seconds =
        Instant::now()
            .duration_since(test_start)
            .as_secs();

    println!(
        "[FINAL] DURATION: {} seconds",
        total_seconds
    );

    println!(
        "[FINAL] TX ATTEMPTS: {}",
        stats.tx_attempts
    );

    println!(
        "[FINAL] MATCHED PONGS: {}",
        stats.rx_pongs
    );

    println!("################################");

    // Keep firmware alive after test.
    loop {
        Timer::after(Duration::from_secs(10)).await;
    }
}
