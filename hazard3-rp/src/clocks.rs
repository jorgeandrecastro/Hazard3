//! # Clock configuration for RP235x microcontrollers.
//!
//! # Clock Configuration API
//!
//! This module provides both high-level convenience functions and low-level manual
//! configuration options for the RP235x clock system.
//!
//! ## High-Level Convenience Functions
//!
//! For most users, these functions provide an easy way to configure clocks:
//!
//! - `ClockConfig::system_freq(150_000_000)` - Set system clock to a specific frequency
//! - `ClockConfig::crystal(12_000_000)` - Default configuration with 12MHz crystal giving 150MHz system clock
//!
//! ## Manual Configuration
//!
//! For advanced users who need precise control:
//!
//! ```rust,ignore
//! // Start with default configuration and customize it
//! let mut config = ClockConfig::default();
//!
//! // Set custom PLL parameters
//! config.xosc = Some(XoscConfig {
//!     hz: 12_000_000,
//!     sys_pll: Some(PllConfig {
//!         refdiv: 1,
//!         fbdiv: 200,
//!         post_div1: 5,
//!         post_div2: 2,
//!     }),
//!     // ... other fields
//! });
//!
//! // Set voltage for overclocking
//! config.core_voltage = CoreVoltage::V1_15;
//! ```
//!
//! ## Examples
//!
//! ### Standard 150MHz configuration
//! ```rust,ignore
//! let config = ClockConfig::crystal(12_000_000);
//! ```
//!
//! Or using the default configuration:
//! ```rust,ignore
//! let config = ClockConfig::default();
//! ```
//!
//! ### Overclock to 200MHz
//! ```rust,ignore
//! let config = ClockConfig::system_freq(200_000_000);
//! ```
//!
//! ### Manual configuration for advanced scenarios
//! ```rust,ignore
//! use hazard3_rp::clocks::{ClockConfig, XoscConfig, PllConfig, CoreVoltage};
//!
//! // Start with defaults and customize
//! let mut config = ClockConfig::default();
//! config.core_voltage = CoreVoltage::V1_15;
//! // Set other parameters as needed...
//! ```

use core::marker::PhantomData;
use core::sync::atomic::{AtomicU32, Ordering};

use pac::clocks::vals::*;

use crate::gpio::Flex;
use crate::{Peri, pac, reset};

// VCO range from 750 MHz to 1600 MHz (RP2350 datasheet, section 8.6.1).
static VCO_MIN: u32 = 750_000_000;
static VCO_MAX: u32 = 1_600_000_000;

// NOTE: all gpin handling is commented out for future reference.
// gpin is not usually safe to use during the boot init() call, so it won't
// be very useful until we have runtime clock reconfiguration. once this
// happens we can resurrect the commented-out gpin bits.

/// Clock error types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ClockError {
    /// PLL failed to lock within the timeout period.
    PllLockTimedOut,
    /// Could not find valid PLL parameters for system clock.
    InvalidPllParameters,
    /// Reading the core voltage failed due to an unexpected value in the register.
    UnexpectedCoreVoltageRead,
}

struct Clocks {
    xosc: AtomicU32,
    sys: AtomicU32,
    reference: AtomicU32,
    pll_sys: AtomicU32,
    pll_usb: AtomicU32,
    usb: AtomicU32,
    adc: AtomicU32,
    // See above re gpin handling being commented out
    // gpin0: AtomicU32,
    // gpin1: AtomicU32,
    rosc: AtomicU32,
    peri: AtomicU32,
}

static CLOCKS: Clocks = Clocks {
    xosc: AtomicU32::new(0),
    sys: AtomicU32::new(0),
    reference: AtomicU32::new(0),
    pll_sys: AtomicU32::new(0),
    pll_usb: AtomicU32::new(0),
    usb: AtomicU32::new(0),
    adc: AtomicU32::new(0),
    // See above re gpin handling being commented out
    // gpin0: AtomicU32::new(0),
    // gpin1: AtomicU32::new(0),
    rosc: AtomicU32::new(0),
    peri: AtomicU32::new(0),
};

/// Peripheral clock sources.
#[repr(u8)]
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum PeriClkSrc {
    /// SYS.
    Sys = ClkPeriCtrlAuxsrc::ClkSys as _,
    /// PLL SYS.
    PllSys = ClkPeriCtrlAuxsrc::ClksrcPllSys as _,
    /// PLL USB.
    PllUsb = ClkPeriCtrlAuxsrc::ClksrcPllUsb as _,
    /// ROSC.
    Rosc = ClkPeriCtrlAuxsrc::RoscClksrcPh as _,
    /// XOSC.
    Xosc = ClkPeriCtrlAuxsrc::XoscClksrc as _,
    // See above re gpin handling being commented out
    // Gpin0 = ClkPeriCtrlAuxsrc::CLKSRC_GPIN0 as _ ,
    // Gpin1 = ClkPeriCtrlAuxsrc::CLKSRC_GPIN1 as _ ,
}

/// Core voltage regulator settings.
///
/// The voltage regulator can be configured for different output voltages.
/// Higher voltages allow for higher clock frequencies but increase power consumption and heat.
///
/// **Note**: The maximum voltage is 1.30V, unless unlocked by setting unless the voltage limit
/// is disabled using the disable_voltage_limit field in the vreg_ctrl register. For lack of practical use at this
/// point in time, this is not implemented here. So the maximum voltage in this enum is 1.30V for now.
#[cfg(feature = "_rp235x")]
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum CoreVoltage {
    /// 0.55V
    V0_55 = 0b00000,
    /// 0.60V
    V0_60 = 0b00001,
    /// 0.65V
    V0_65 = 0b00010,
    /// 0.70V
    V0_70 = 0b00011,
    /// 0.75V
    V0_75 = 0b00100,
    /// 0.80V
    V0_80 = 0b00101,
    /// 0.85V
    V0_85 = 0b00110,
    /// 0.90V
    V0_90 = 0b00111,
    /// 0.95V
    V0_95 = 0b01000,
    /// 1.00V
    V1_00 = 0b01001,
    /// 1.05V
    V1_05 = 0b01010,
    /// 1.10V - Default voltage level
    V1_10 = 0b01011,
    /// 1.15V
    V1_15 = 0b01100,
    /// 1.20V
    V1_20 = 0b01101,
    /// 1.25V
    V1_25 = 0b01110,
    /// 1.30V
    V1_30 = 0b01111,
}

impl CoreVoltage {
    /// Get the recommended Brown-Out Detection (BOD) setting for this voltage.
    /// Sets the BOD threshold to approximately 80% of the core voltage.
    fn recommended_bod(self) -> u8 {
        match self {
            CoreVoltage::V0_55 => 0b00001, // 0.516V (~94% of 0.55V)
            CoreVoltage::V0_60 => 0b00010, // 0.559V (~93% of 0.60V)
            CoreVoltage::V0_65 => 0b00011, // 0.602V (~93% of 0.65V)
            CoreVoltage::V0_70 => 0b00011, // 0.602V (~86% of 0.70V)
            CoreVoltage::V0_75 => 0b00100, // 0.645V (~86% of 0.75V)
            CoreVoltage::V0_80 => 0b00101, // 0.688V (~86% of 0.80V)
            CoreVoltage::V0_85 => 0b00110, // 0.731V (~86% of 0.85V)
            CoreVoltage::V0_90 => 0b00110, // 0.731V (~81% of 0.90V)
            CoreVoltage::V0_95 => 0b00111, // 0.774V (~81% of 0.95V)
            CoreVoltage::V1_00 => 0b01000, // 0.817V (~82% of 1.00V)
            CoreVoltage::V1_05 => 0b01000, // 0.817V (~78% of 1.05V)
            CoreVoltage::V1_10 => 0b01001, // 0.860V (~78% of 1.10V), the default
            CoreVoltage::V1_15 => 0b01001, // 0.860V (~75% of 1.15V)
            CoreVoltage::V1_20 => 0b01010, // 0.903V (~75% of 1.20V)
            CoreVoltage::V1_25 => 0b01010, // 0.903V (~72% of 1.25V)
            CoreVoltage::V1_30 => 0b01011, // 0.946V (~73% of 1.30V)
                                            // all others: 0.946V (see CoreVoltage: we do not support setting Voltages higher than 1.30V at this point)
        }
    }
}

/// Clock configuration.
#[non_exhaustive]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ClockConfig {
    /// Ring oscillator configuration.
    pub rosc: Option<RoscConfig>,
    /// External oscillator configuration.
    pub xosc: Option<XoscConfig>,
    /// Reference clock configuration.
    pub ref_clk: RefClkConfig,
    /// System clock configuration.
    pub sys_clk: SysClkConfig,
    /// Peripheral clock source configuration.
    pub peri_clk_src: Option<PeriClkSrc>,
    /// USB clock configuration.
    pub usb_clk: Option<UsbClkConfig>,
    /// ADC clock configuration.
    pub adc_clk: Option<AdcClkConfig>,
    /// Core voltage scaling. Defaults to 1.10V.
    pub core_voltage: CoreVoltage,
    /// Voltage stabilization delay in microseconds.
    /// If not set, defaults will be used based on voltage level.
    pub voltage_stabilization_delay_us: Option<u32>,
    // See above re gpin handling being commented out
    // gpin0: Option<(u32, Gpin<'static, AnyPin>)>,
    // gpin1: Option<(u32, Gpin<'static, AnyPin>)>,
}

impl Default for ClockConfig {
    /// Creates a minimal default configuration with safe values.
    ///
    /// This configuration uses the ring oscillator (ROSC) as the clock source
    /// and sets minimal defaults that guarantee a working system. It's intended
    /// as a starting point for manual configuration.
    ///
    /// Most users should use one of the more specific configuration functions:
    /// - `ClockConfig::crystal()` - Standard configuration with external crystal
    /// - `ClockConfig::rosc()` - Configuration using only the internal oscillator
    /// - `ClockConfig::system_freq()` - Configuration for a specific system frequency
    fn default() -> Self {
        Self {
            rosc: None,
            xosc: None,
            ref_clk: RefClkConfig {
                src: RefClkSrc::Rosc,
                div: 1,
            },
            sys_clk: SysClkConfig {
                src: SysClkSrc::Rosc,
                div_int: 1,
                div_frac: 0,
            },
            peri_clk_src: None,
            usb_clk: None,
            adc_clk: None,
            core_voltage: CoreVoltage::V1_10,
            voltage_stabilization_delay_us: None,
            // See above re gpin handling being commented out
            // gpin0: None,
            // gpin1: None,
        }
    }
}

impl ClockConfig {
    /// Clock configuration derived from external crystal.
    ///
    /// This uses default settings for most parameters, suitable for typical use cases.
    /// For manual control of PLL parameters, use `new_manual()` or modify the struct fields directly.
    pub fn crystal(crystal_hz: u32) -> Self {
        Self {
            rosc: Some(RoscConfig {
                hz: 6_500_000,
                range: RoscRange::Medium,
                drive_strength: [0; 8],
                div: 16,
            }),
            xosc: Some(XoscConfig {
                hz: crystal_hz,
                sys_pll: Some(PllConfig {
                    refdiv: 1,
                    fbdiv: 125,
                    post_div1: 5,
                    post_div2: 2,
                }),
                usb_pll: Some(PllConfig {
                    refdiv: 1,
                    fbdiv: 120,
                    post_div1: 6,
                    post_div2: 5,
                }),
                delay_multiplier: 128,
            }),
            ref_clk: RefClkConfig {
                src: RefClkSrc::Xosc,
                div: 1,
            },
            sys_clk: SysClkConfig {
                src: SysClkSrc::PllSys,
                div_int: 1,
                div_frac: 0,
            },
            peri_clk_src: Some(PeriClkSrc::Sys),
            // CLK USB = PLL USB (48MHz) / 1 = 48MHz
            usb_clk: Some(UsbClkConfig {
                src: UsbClkSrc::PllUsb,
                div: 1,
                phase: 0,
            }),
            // CLK ADC = PLL USB (48MHZ) / 1 = 48MHz
            adc_clk: Some(AdcClkConfig {
                src: AdcClkSrc::PllUsb,
                div: 1,
                phase: 0,
            }),
            core_voltage: CoreVoltage::V1_10, // Use hardware default (1.10V)
            voltage_stabilization_delay_us: None,
            // See above re gpin handling being commented out
            // gpin0: None,
            // gpin1: None,
        }
    }

    /// Clock configuration from internal oscillator.
    pub fn rosc() -> Self {
        Self {
            rosc: Some(RoscConfig {
                hz: 140_000_000,
                range: RoscRange::High,
                drive_strength: [0; 8],
                div: 1,
            }),
            xosc: None,
            ref_clk: RefClkConfig {
                src: RefClkSrc::Rosc,
                div: 1,
            },
            sys_clk: SysClkConfig {
                src: SysClkSrc::Rosc,
                div_int: 1,
                div_frac: 0,
            },
            peri_clk_src: Some(PeriClkSrc::Rosc),
            usb_clk: None,
            // CLK ADC = ROSC (140MHz) / 3 ≅ 48MHz
            adc_clk: Some(AdcClkConfig {
                src: AdcClkSrc::Rosc,
                div: 3,
                phase: 0,
            }),
            core_voltage: CoreVoltage::V1_10, // Use hardware default (1.10V)
            voltage_stabilization_delay_us: None,
            // See above re gpin handling being commented out
            // gpin0: None,
            // gpin1: None,
        }
    }

    /// Configure clocks derived from an external crystal with specific system frequency.
    ///
    /// This function calculates optimal PLL parameters to achieve the requested system
    /// frequency. This only works for the usual 12MHz crystal. In case a different crystal is used,
    /// You will have to set the PLL parameters manually.
    ///
    /// # Arguments
    ///
    /// * `sys_freq_hz` - The desired system clock frequency in Hz
    ///
    /// # Returns
    ///
    /// A ClockConfig configured to achieve the requested system frequency using the
    /// the usual 12Mhz crystal, or an error if no valid parameters can be found.
    ///
    /// # Note on core voltage:
    ///
    /// **For RP235x**:
    /// At this point in time there is no official manufacturer endorsement for running the chip on other core voltages and/or other clock speeds than the defaults.
    /// Using this function is experimental and may not work as expected or even damage the chip.
    ///
    /// # Returns
    ///
    /// A Result containing either the configured ClockConfig or a ClockError.
    pub fn system_freq(hz: u32) -> Result<Self, ClockError> {
        // Start with the standard configuration from crystal()
        const DEFAULT_CRYSTAL_HZ: u32 = 12_000_000;
        let mut config = Self::crystal(DEFAULT_CRYSTAL_HZ);

        if hz == 150_000_000 {
            return Ok(config);
        }

        // Find optimal PLL parameters for the requested frequency
        let sys_pll_params = find_pll_params(DEFAULT_CRYSTAL_HZ, hz).ok_or(ClockError::InvalidPllParameters)?;

        // Replace the sys_pll configuration with our custom parameters
        if let Some(xosc) = &mut config.xosc {
            xosc.sys_pll = Some(sys_pll_params);
        }

        #[cfg(feature = "_rp235x")]
        #[allow(clippy::match_single_binding)]
        {
            config.core_voltage = match hz {
                // There is no official support for running the chip on other core voltages and/or other clock speeds than the defaults.
                // So for now we have not way of knowing what the voltage should be. Change this if the manufacturer provides more information.
                _ => CoreVoltage::V1_10, // Use default voltage (V1_10)
            };
        }

        Ok(config)
    }
}

/// ROSC freq range.
#[repr(u16)]
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum RoscRange {
    /// Low range.
    Low = pac::rosc::vals::FreqRange::Low.to_bits(),
    /// Medium range (1.33x low)
    Medium = pac::rosc::vals::FreqRange::Medium.to_bits(),
    /// High range (2x low)
    High = pac::rosc::vals::FreqRange::High.to_bits(),
    /// Too high. Should not be used.
    TooHigh = pac::rosc::vals::FreqRange::Toohigh.to_bits(),
}

/// On-chip ring oscillator configuration.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct RoscConfig {
    /// Final frequency of the oscillator, after the divider has been applied.
    /// The oscillator has a nominal frequency of 6.5MHz at medium range with
    /// divider 16 and all drive strengths set to 0, other values should be
    /// measured in situ.
    pub hz: u32,
    /// Oscillator range.
    pub range: RoscRange,
    /// Drive strength for oscillator.
    pub drive_strength: [u8; 8],
    /// Output divider.
    pub div: u16,
}

/// Crystal oscillator configuration.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct XoscConfig {
    /// Final frequency of the oscillator.
    pub hz: u32,
    /// Configuring PLL for the system clock.
    pub sys_pll: Option<PllConfig>,
    /// Configuring PLL for the USB clock.
    pub usb_pll: Option<PllConfig>,
    /// Multiplier for the startup delay.
    pub delay_multiplier: u32,
}

/// PLL configuration.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct PllConfig {
    /// Reference divisor.
    pub refdiv: u8,
    /// Feedback divisor.
    pub fbdiv: u16,
    /// Output divisor 1.
    pub post_div1: u8,
    /// Output divisor 2.
    pub post_div2: u8,
}

impl PllConfig {
    /// Calculate the output frequency for this PLL configuration
    /// given an input frequency.
    pub fn output_frequency(&self, input_hz: u32) -> u32 {
        let ref_freq = input_hz / self.refdiv as u32;
        let vco_freq = ref_freq * self.fbdiv as u32;
        vco_freq / ((self.post_div1 * self.post_div2) as u32)
    }

    /// Check if this PLL configuration is valid for the given input frequency.
    pub fn is_valid(&self, input_hz: u32) -> bool {
        // Check divisor constraints
        if self.refdiv < 1 || self.refdiv > 63 {
            return false;
        }
        if self.fbdiv < 16 || self.fbdiv > 320 {
            return false;
        }
        if self.post_div1 < 1 || self.post_div1 > 7 {
            return false;
        }
        if self.post_div2 < 1 || self.post_div2 > 7 {
            return false;
        }
        if self.post_div2 > self.post_div1 {
            return false;
        }

        // Calculate reference frequency
        let ref_freq = input_hz / self.refdiv as u32;

        // Check reference frequency range
        if !(5_000_000..=800_000_000).contains(&ref_freq) {
            return false;
        }

        // Calculate VCO frequency
        let vco_freq = ref_freq * self.fbdiv as u32;

        // Check VCO frequency range
        vco_freq >= VCO_MIN && vco_freq <= VCO_MAX
    }
}

/// Reference clock config.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct RefClkConfig {
    /// Reference clock source.
    pub src: RefClkSrc,
    /// Reference clock divider.
    pub div: u8,
}

/// Reference clock source.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum RefClkSrc {
    /// XOSC.
    Xosc,
    /// ROSC.
    Rosc,
    /// PLL USB.
    PllUsb,
    // See above re gpin handling being commented out
    // Gpin0,
    // Gpin1,
}

/// SYS clock source.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SysClkSrc {
    /// REF.
    Ref,
    /// PLL SYS.
    PllSys,
    /// PLL USB.
    PllUsb,
    /// ROSC.
    Rosc,
    /// XOSC.
    Xosc,
    // See above re gpin handling being commented out
    // Gpin0,
    // Gpin1,
}

/// SYS clock config.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct SysClkConfig {
    /// SYS clock source.
    pub src: SysClkSrc,
    pub div_int: u16,
    /// SYS clock fraction.
    pub div_frac: u16,
}

/// USB clock source.
#[repr(u8)]
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum UsbClkSrc {
    /// PLL USB.
    PllUsb = ClkUsbCtrlAuxsrc::ClksrcPllUsb as _,
    /// PLL SYS.
    PllSys = ClkUsbCtrlAuxsrc::ClksrcPllSys as _,
    /// ROSC.
    Rosc = ClkUsbCtrlAuxsrc::RoscClksrcPh as _,
    /// XOSC.
    Xosc = ClkUsbCtrlAuxsrc::XoscClksrc as _,
    // See above re gpin handling being commented out
    // Gpin0 = ClkUsbCtrlAuxsrc::CLKSRC_GPIN0 as _ ,
    // Gpin1 = ClkUsbCtrlAuxsrc::CLKSRC_GPIN1 as _ ,
}

/// USB clock config.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct UsbClkConfig {
    /// USB clock source.
    pub src: UsbClkSrc,
    /// USB clock divider.
    pub div: u8,
    /// USB clock phase.
    pub phase: u8,
}

/// ADC clock source.
#[repr(u8)]
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum AdcClkSrc {
    /// PLL USB.
    PllUsb = ClkAdcCtrlAuxsrc::ClksrcPllUsb as _,
    /// PLL SYS.
    PllSys = ClkAdcCtrlAuxsrc::ClksrcPllSys as _,
    /// ROSC.
    Rosc = ClkAdcCtrlAuxsrc::RoscClksrcPh as _,
    /// XOSC.
    Xosc = ClkAdcCtrlAuxsrc::XoscClksrc as _,
    // See above re gpin handling being commented out
    // Gpin0 = ClkAdcCtrlAuxsrc::CLKSRC_GPIN0 as _ ,
    // Gpin1 = ClkAdcCtrlAuxsrc::CLKSRC_GPIN1 as _ ,
}

/// ADC clock config.
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AdcClkConfig {
    /// ADC clock source.
    pub src: AdcClkSrc,
    /// ADC clock divider.
    pub div: u8,
    /// ADC clock phase.
    pub phase: u8,
}

/// Find valid PLL parameters (refdiv, fbdiv, post_div1, post_div2) for a target output frequency
/// based on the input frequency.
///
/// This function searches for the best PLL configuration to achieve the requested target frequency
/// while staying within the VCO frequency range of 750MHz to 1600MHz. It prioritizes stability
/// over exact frequency matching by using larger divisors where possible.
///
/// # Parameters
///
/// * `input_hz`: The input frequency in Hz, typically the crystal frequency
/// * `target_hz`: The desired output frequency in Hz
///
/// # Returns
///
/// * `Some(PllConfig)` if valid parameters were found
/// * `None` if no valid parameters could be found for the requested combination
///
/// # Example
///
/// ```rust,ignore
/// // Find parameters for 133MHz system clock from 12MHz crystal
/// let pll_params = find_pll_params(12_000_000, 133_000_000).unwrap();
/// ```
fn find_pll_params(input_hz: u32, target_hz: u32) -> Option<PllConfig> {
    // Fixed reference divider for system PLL
    const PLL_SYS_REFDIV: u8 = 1;

    // Calculate reference frequency
    let reference_freq = input_hz as u64 / PLL_SYS_REFDIV as u64;

    // Start from highest fbdiv for better stability (like SDK does)
    for fbdiv in (16..=320).rev() {
        let vco_freq = reference_freq * fbdiv;

        // Check VCO frequency is within valid range
        if vco_freq < VCO_MIN as u64 || vco_freq > VCO_MAX as u64 {
            continue;
        }

        // Try all possible postdiv combinations starting from larger values
        // (more conservative/stable approach)
        for post_div1 in (1..=7).rev() {
            for post_div2 in (1..=post_div1).rev() {
                let out_freq = vco_freq / (post_div1 * post_div2);

                // Check if we get the exact target frequency without remainder
                if out_freq == target_hz as u64 && vco_freq.is_multiple_of(post_div1 * post_div2) {
                    return Some(PllConfig {
                        refdiv: PLL_SYS_REFDIV,
                        fbdiv: fbdiv as u16,
                        post_div1: post_div1 as u8,
                        post_div2: post_div2 as u8,
                    });
                }
            }
        }
    }

    // If we couldn't find an exact match, find the closest match
    let mut best_config = None;
    let mut min_diff = u32::MAX;

    for fbdiv in (16..=320).rev() {
        let vco_freq = reference_freq * fbdiv;

        if vco_freq < VCO_MIN as u64 || vco_freq > VCO_MAX as u64 {
            continue;
        }

        for post_div1 in (1..=7).rev() {
            for post_div2 in (1..=post_div1).rev() {
                let out_freq = (vco_freq / (post_div1 * post_div2) as u64) as u32;
                let diff = out_freq.abs_diff(target_hz);

                // If this is closer to the target, save it
                if diff < min_diff {
                    min_diff = diff;
                    best_config = Some(PllConfig {
                        refdiv: PLL_SYS_REFDIV,
                        fbdiv: fbdiv as u16,
                        post_div1: post_div1 as u8,
                        post_div2: post_div2 as u8,
                    });
                }
            }
        }
    }

    // Return the closest match if we found one
    best_config
}

/// safety: must be called exactly once at bootup
pub(crate) unsafe fn init(config: ClockConfig) {
    // Reset everything except:
    // - QSPI (we're using it to run this code!)
    // - PLLs (it may be suicide if that's what's clocking us)
    // - USB, SYSCFG (breaks usb-to-swd on core1)
    // - RTC (else there would be no more time...)
    let mut peris = reset::ALL_PERIPHERALS;
    peris.set_io_qspi(false);
    // peris.set_io_bank0(false); // might be suicide if we're clocked from gpin
    peris.set_pads_qspi(false);
    peris.set_pll_sys(false);
    peris.set_pll_usb(false);
    peris.set_usbctrl(false);
    peris.set_syscfg(false);
    //peris.set_rtc(false);
    reset::reset(peris);

    // Disable resus that may be enabled from previous software
    let c = pac::CLOCKS;
    c.clk_sys_resus_ctrl()
        .write_value(pac::clocks::regs::ClkSysResusCtrl(0));

    // Before we touch PLLs, switch sys and ref cleanly away from their aux sources.
    c.clk_sys_ctrl().modify(|w| w.set_src(ClkSysCtrlSrc::ClkRef));
    #[cfg(feature = "_rp235x")]
    while c.clk_sys_selected().read() != pac::clocks::regs::ClkSysSelected(1) {}
    c.clk_ref_ctrl().modify(|w| w.set_src(ClkRefCtrlSrc::RoscClksrcPh));
    #[cfg(feature = "_rp235x")]
    while c.clk_ref_selected().read() != pac::clocks::regs::ClkRefSelected(1) {}

    // Reset the PLLs
    let mut peris = reset::Peripherals(0);
    peris.set_pll_sys(true);
    peris.set_pll_usb(true);
    reset::reset(peris);
    reset::unreset_wait(peris);

    // See above re gpin handling being commented out
    // let gpin0_freq = config.gpin0.map_or(0, |p| {
    //     core::mem::forget(p.1);
    //     p.0
    // });
    // CLOCKS.gpin0.store(gpin0_freq, Ordering::Relaxed);
    // let gpin1_freq = config.gpin1.map_or(0, |p| {
    //     core::mem::forget(p.1);
    //     p.0
    // });
    // CLOCKS.gpin1.store(gpin1_freq, Ordering::Relaxed);

    let rosc_freq = match config.rosc {
        Some(config) => configure_rosc(config),
        None => 0,
    };
    CLOCKS.rosc.store(rosc_freq, Ordering::Relaxed);

    // Set Core Voltage, if we have config for it and we're not using the default
    {
        let voltage = config.core_voltage;

        let vreg = pac::POWMAN;

        let current_vsel = vreg.vreg().read().vsel();
        let target_vsel = voltage as u8;

        // If the target voltage is different from the current one, we need to change it
        if target_vsel != current_vsel {
            {
                // For rp235x changes to the voltage regulator are protected by a password, see datasheet section 6.4 Power Management (POWMAN) Registers
                // The password is "5AFE" (0x5AFE), it must be set in the top 16 bits of the registers
                // Unlock the VREG control interface, this cannot be locked after being unlocked
                vreg.vreg_ctrl().modify(|w| {
                    w.0 = (w.0 & 0x0000FFFF) | (0x5AFE << 16); // Password
                    w.set_unlock(true);
                });
                // Set the regulator to the target voltage
                vreg.vreg().modify(|w| {
                    w.0 = (w.0 & 0x0000FFFF) | (0x5AFE << 16); // Password
                    w.set_vsel(target_vsel);
                });
            }

            // Wait for the voltage to stabilize. Use the provided delay or default based on voltage
            let settling_time_us = config.voltage_stabilization_delay_us.unwrap_or({
                match voltage {
                    CoreVoltage::V1_15 => 1000,                                           // 1ms for 1.15V
                    CoreVoltage::V1_20 | CoreVoltage::V1_25 | CoreVoltage::V1_30 => 2000, // 2ms for higher voltages
                    _ => 0,                                                               // no delay for all others
                }
            });

            if settling_time_us != 0 {
                // Delay in microseconds, using the ROSC frequency to calculate cycles
                let cycles_per_us = rosc_freq / 1_000_000;
                let delay_cycles = settling_time_us * cycles_per_us;
                crate::cpu::delay(delay_cycles);
            }

            vreg.bod().write(|w| {
                w.0 = (w.0 & 0x0000FFFF) | (0x5AFE << 16); // Set the password
                w.set_vsel(voltage.recommended_bod());
                w.set_en(true); // Enable brownout detection
            });
        }
    }

    let (xosc_freq, pll_sys_freq, pll_usb_freq) = match config.xosc {
        Some(config) => {
            // start XOSC
            start_xosc(config.hz, config.delay_multiplier);

            let pll_sys_freq = match config.sys_pll {
                Some(sys_pll_config) => match configure_pll(pac::PLL_SYS, config.hz, sys_pll_config) {
                    Ok(freq) => freq,
                    Err(e) => panic!("Failed to configure PLL_SYS: {:?}", e),
                },
                None => 0,
            };
            let pll_usb_freq = match config.usb_pll {
                Some(usb_pll_config) => match configure_pll(pac::PLL_USB, config.hz, usb_pll_config) {
                    Ok(freq) => freq,
                    Err(e) => panic!("Failed to configure PLL_USB: {:?}", e),
                },
                None => 0,
            };

            (config.hz, pll_sys_freq, pll_usb_freq)
        }
        None => (0, 0, 0),
    };

    CLOCKS.xosc.store(xosc_freq, Ordering::Relaxed);
    CLOCKS.pll_sys.store(pll_sys_freq, Ordering::Relaxed);
    CLOCKS.pll_usb.store(pll_usb_freq, Ordering::Relaxed);

    let (ref_src, ref_aux, clk_ref_freq) = {
        use ClkRefCtrlAuxsrc as Aux;
        use ClkRefCtrlSrc as Src;
        let div = config.ref_clk.div as u32;
        assert!((1..=4).contains(&div));
        match config.ref_clk.src {
            RefClkSrc::Xosc => (Src::XoscClksrc, Aux::ClksrcPllUsb, xosc_freq / div),
            RefClkSrc::Rosc => (Src::RoscClksrcPh, Aux::ClksrcPllUsb, rosc_freq / div),
            RefClkSrc::PllUsb => (Src::ClksrcClkRefAux, Aux::ClksrcPllUsb, pll_usb_freq / div),
            // See above re gpin handling being commented out
            // RefClkSrc::Gpin0 => (Src::ClksrcClkRefAux, Aux::CLKSRC_GPIN0, gpin0_freq / div),
            // RefClkSrc::Gpin1 => (Src::ClksrcClkRefAux, Aux::CLKSRC_GPIN1, gpin1_freq / div),
        }
    };
    assert!(clk_ref_freq != 0);
    CLOCKS.reference.store(clk_ref_freq, Ordering::Relaxed);
    c.clk_ref_ctrl().write(|w| {
        w.set_src(ref_src);
        w.set_auxsrc(ref_aux);
    });
    #[cfg(feature = "_rp235x")]
    while c.clk_ref_selected().read() != pac::clocks::regs::ClkRefSelected(1 << ref_src as u32) {}
    c.clk_ref_div().write(|w| {
        w.set_int(config.ref_clk.div);
    });

    // Configure tick generator on the 2350
    #[cfg(feature = "_rp235x")]
    {
        let cycle_count = clk_ref_freq / 1_000_000;

        pac::TICKS.timer0_cycles().write(|w| w.0 = cycle_count);
        pac::TICKS.timer0_ctrl().write(|w| w.set_enable(true));

        pac::TICKS.watchdog_cycles().write(|w| w.0 = cycle_count);
        pac::TICKS.watchdog_ctrl().write(|w| w.set_enable(true));
    }

    let (sys_src, sys_aux, clk_sys_freq) = {
        use ClkSysCtrlAuxsrc as Aux;
        use ClkSysCtrlSrc as Src;
        let (src, aux, freq) = match config.sys_clk.src {
            SysClkSrc::Ref => (Src::ClkRef, Aux::ClksrcPllSys, clk_ref_freq),
            SysClkSrc::PllSys => (Src::ClksrcClkSysAux, Aux::ClksrcPllSys, pll_sys_freq),
            SysClkSrc::PllUsb => (Src::ClksrcClkSysAux, Aux::ClksrcPllUsb, pll_usb_freq),
            SysClkSrc::Rosc => (Src::ClksrcClkSysAux, Aux::RoscClksrc, rosc_freq),
            SysClkSrc::Xosc => (Src::ClksrcClkSysAux, Aux::XoscClksrc, xosc_freq),
            // See above re gpin handling being commented out
            // SysClkSrc::Gpin0 => (Src::ClksrcClkSysAux, Aux::CLKSRC_GPIN0, gpin0_freq),
            // SysClkSrc::Gpin1 => (Src::ClksrcClkSysAux, Aux::CLKSRC_GPIN1, gpin1_freq),
        };
        let div = config.sys_clk.div_int as u64 * 256 + config.sys_clk.div_frac as u64;
        (src, aux, ((freq as u64 * 256) / div) as u32)
    };
    assert!(clk_sys_freq != 0);
    CLOCKS.sys.store(clk_sys_freq, Ordering::Relaxed);
    if sys_src != ClkSysCtrlSrc::ClkRef {
        c.clk_sys_ctrl().write(|w| w.set_src(ClkSysCtrlSrc::ClkRef));
        #[cfg(feature = "_rp235x")]
        while c.clk_sys_selected().read() != pac::clocks::regs::ClkSysSelected(1 << ClkSysCtrlSrc::ClkRef as u32) {}
    }
    c.clk_sys_ctrl().write(|w| {
        w.set_auxsrc(sys_aux);
        w.set_src(sys_src);
    });

    #[cfg(feature = "_rp235x")]
    while c.clk_sys_selected().read() != pac::clocks::regs::ClkSysSelected(1 << sys_src as u32) {}

    c.clk_sys_div().write(|w| {
        w.set_int(config.sys_clk.div_int);
        w.set_frac(config.sys_clk.div_frac);
    });

    let mut peris = reset::ALL_PERIPHERALS;

    if let Some(src) = config.peri_clk_src {
        c.clk_peri_ctrl().write(|w| {
            w.set_enable(true);
            w.set_auxsrc(ClkPeriCtrlAuxsrc::from_bits(src as _));
        });
        let peri_freq = match src {
            PeriClkSrc::Sys => clk_sys_freq,
            PeriClkSrc::PllSys => pll_sys_freq,
            PeriClkSrc::PllUsb => pll_usb_freq,
            PeriClkSrc::Rosc => rosc_freq,
            PeriClkSrc::Xosc => xosc_freq,
            // See above re gpin handling being commented out
            // PeriClkSrc::Gpin0 => gpin0_freq,
            // PeriClkSrc::Gpin1 => gpin1_freq,
        };
        assert!(peri_freq != 0);
        CLOCKS.peri.store(peri_freq, Ordering::Relaxed);
    } else {
        peris.set_spi0(false);
        peris.set_spi1(false);
        peris.set_uart0(false);
        peris.set_uart1(false);
        CLOCKS.peri.store(0, Ordering::Relaxed);
    }

    if let Some(conf) = config.usb_clk {
        c.clk_usb_div().write(|w| w.set_int(conf.div));
        c.clk_usb_ctrl().write(|w| {
            w.set_phase(conf.phase);
            w.set_enable(true);
            w.set_auxsrc(ClkUsbCtrlAuxsrc::from_bits(conf.src as _));
        });
        let usb_freq = match conf.src {
            UsbClkSrc::PllUsb => pll_usb_freq,
            UsbClkSrc::PllSys => pll_sys_freq,
            UsbClkSrc::Rosc => rosc_freq,
            UsbClkSrc::Xosc => xosc_freq,
            // See above re gpin handling being commented out
            // UsbClkSrc::Gpin0 => gpin0_freq,
            // UsbClkSrc::Gpin1 => gpin1_freq,
        };
        assert!(usb_freq != 0);
        assert!(conf.div >= 1 && conf.div <= 4);
        CLOCKS.usb.store(usb_freq / conf.div as u32, Ordering::Relaxed);
    } else {
        peris.set_usbctrl(false);
        CLOCKS.usb.store(0, Ordering::Relaxed);
    }

    if let Some(conf) = config.adc_clk {
        c.clk_adc_div().write(|w| w.set_int(conf.div));
        c.clk_adc_ctrl().write(|w| {
            w.set_phase(conf.phase);
            w.set_enable(true);
            w.set_auxsrc(ClkAdcCtrlAuxsrc::from_bits(conf.src as _));
        });
        let adc_in_freq = match conf.src {
            AdcClkSrc::PllUsb => pll_usb_freq,
            AdcClkSrc::PllSys => pll_sys_freq,
            AdcClkSrc::Rosc => rosc_freq,
            AdcClkSrc::Xosc => xosc_freq,
            // See above re gpin handling being commented out
            // AdcClkSrc::Gpin0 => gpin0_freq,
            // AdcClkSrc::Gpin1 => gpin1_freq,
        };
        assert!(adc_in_freq != 0);
        assert!(conf.div >= 1 && conf.div <= 4);
        CLOCKS.adc.store(adc_in_freq / conf.div as u32, Ordering::Relaxed);
    } else {
        peris.set_adc(false);
        CLOCKS.adc.store(0, Ordering::Relaxed);
    }

    // rp235x specific clocks
    #[cfg(feature = "_rp235x")]
    {
        // TODO hstx clock
        peris.set_hstx(false);
    }

    // Peripheral clocks should now all be running
    reset::unreset_wait(peris);
}

fn configure_rosc(config: RoscConfig) -> u32 {
    let p = pac::ROSC;

    p.freqa().write(|w| {
        w.set_passwd(pac::rosc::vals::Passwd::Pass);
        w.set_ds0(config.drive_strength[0]);
        w.set_ds1(config.drive_strength[1]);
        w.set_ds2(config.drive_strength[2]);
        w.set_ds3(config.drive_strength[3]);
    });

    p.freqb().write(|w| {
        w.set_passwd(pac::rosc::vals::Passwd::Pass);
        w.set_ds4(config.drive_strength[4]);
        w.set_ds5(config.drive_strength[5]);
        w.set_ds6(config.drive_strength[6]);
        w.set_ds7(config.drive_strength[7]);
    });

    p.div().write(|w| {
        w.set_div(pac::rosc::vals::Div::from_bits(
            config.div + pac::rosc::vals::Div::Pass.to_bits(),
        ));
    });

    p.ctrl().write(|w| {
        w.set_enable(pac::rosc::vals::Enable::Enable);
        w.set_freq_range(pac::rosc::vals::FreqRange::from_bits(config.range as u16));
    });

    config.hz
}

/// ROSC clock frequency.
pub fn rosc_freq() -> u32 {
    CLOCKS.rosc.load(Ordering::Relaxed)
}

/// XOSC clock frequency.
pub fn xosc_freq() -> u32 {
    CLOCKS.xosc.load(Ordering::Relaxed)
}

// See above re gpin handling being commented out
// pub fn gpin0_freq() -> u32 {
//     CLOCKS.gpin0.load(Ordering::Relaxed)
// }
// pub fn gpin1_freq() -> u32 {
//     CLOCKS.gpin1.load(Ordering::Relaxed)
// }

/// PLL SYS clock frequency.
pub fn pll_sys_freq() -> u32 {
    CLOCKS.pll_sys.load(Ordering::Relaxed)
}

/// PLL USB clock frequency.
pub fn pll_usb_freq() -> u32 {
    CLOCKS.pll_usb.load(Ordering::Relaxed)
}

/// SYS clock frequency.
pub fn clk_sys_freq() -> u32 {
    CLOCKS.sys.load(Ordering::Relaxed)
}

/// REF clock frequency.
pub fn clk_ref_freq() -> u32 {
    CLOCKS.reference.load(Ordering::Relaxed)
}

/// Peripheral clock frequency.
pub fn clk_peri_freq() -> u32 {
    CLOCKS.peri.load(Ordering::Relaxed)
}

/// USB clock frequency.
pub fn clk_usb_freq() -> u32 {
    CLOCKS.usb.load(Ordering::Relaxed)
}

/// ADC clock frequency.
pub fn clk_adc_freq() -> u32 {
    CLOCKS.adc.load(Ordering::Relaxed)
}

/// The core voltage of the chip.
///
/// Returns the current core voltage or an error if the voltage register
/// contains an unknown value.
pub fn core_voltage() -> Result<CoreVoltage, ClockError> {
    let vreg = pac::POWMAN;
    let vsel = vreg.vreg().read().vsel();
    match vsel {
        0b00000 => Ok(CoreVoltage::V0_55),
        0b00001 => Ok(CoreVoltage::V0_60),
        0b00010 => Ok(CoreVoltage::V0_65),
        0b00011 => Ok(CoreVoltage::V0_70),
        0b00100 => Ok(CoreVoltage::V0_75),
        0b00101 => Ok(CoreVoltage::V0_80),
        0b00110 => Ok(CoreVoltage::V0_85),
        0b00111 => Ok(CoreVoltage::V0_90),
        0b01000 => Ok(CoreVoltage::V0_95),
        0b01001 => Ok(CoreVoltage::V1_00),
        0b01010 => Ok(CoreVoltage::V1_05),
        0b01011 => Ok(CoreVoltage::V1_10),
        0b01100 => Ok(CoreVoltage::V1_15),
        0b01101 => Ok(CoreVoltage::V1_20),
        0b01110 => Ok(CoreVoltage::V1_25),
        0b01111 => Ok(CoreVoltage::V1_30),
        _ => Err(ClockError::UnexpectedCoreVoltageRead),
    }
}

fn start_xosc(crystal_hz: u32, delay_multiplier: u32) {
    let startup_delay = (((crystal_hz / 1000) * delay_multiplier) + 128) / 256;
    pac::XOSC.startup().write(|w| w.set_delay(startup_delay as u16));
    pac::XOSC.ctrl().write(|w| {
        w.set_freq_range(pac::xosc::vals::CtrlFreqRange::_115mhz);
        w.set_enable(pac::xosc::vals::Enable::Enable);
    });
    while !pac::XOSC.status().read().stable() {}
}

/// PLL (Phase-Locked Loop) configuration
#[inline(always)]
fn configure_pll(p: pac::pll::Pll, input_freq: u32, config: PllConfig) -> Result<u32, ClockError> {
    // Calculate reference frequency
    let ref_freq = input_freq / config.refdiv as u32;

    // Validate PLL parameters
    // Feedback divider (FBDIV) must be between 16 and 320
    assert!(config.fbdiv >= 16 && config.fbdiv <= 320);

    // Post divider 1 (POSTDIV1) must be between 1 and 7
    assert!(config.post_div1 >= 1 && config.post_div1 <= 7);

    // Post divider 2 (POSTDIV2) must be between 1 and 7
    assert!(config.post_div2 >= 1 && config.post_div2 <= 7);

    // Post divider 2 (POSTDIV2) must be less than or equal to post divider 1 (POSTDIV1)
    assert!(config.post_div2 <= config.post_div1);

    // Reference divider (REFDIV) must be between 1 and 63
    assert!(config.refdiv >= 1 && config.refdiv <= 63);

    // Reference frequency (REF_FREQ) must be between 5MHz and 800MHz
    assert!((5_000_000..=800_000_000).contains(&ref_freq));

    // Calculate VCO frequency
    let vco_freq = ref_freq.saturating_mul(config.fbdiv as u32);

    // VCO (Voltage Controlled Oscillator) frequency must be between 750MHz and 1600MHz
    assert!(vco_freq >= VCO_MIN && vco_freq <= VCO_MAX);

    // We follow the SDK's approach to PLL configuration which is:
    // 1. Power down PLL
    // 2. Configure the reference divider
    // 3. Configure the feedback divider
    // 4. Power up PLL and VCO
    // 5. Wait for PLL to lock
    // 6. Configure post-dividers
    // 7. Enable post-divider output

    // 1. Power down PLL before configuration
    p.pwr().write(|w| {
        w.set_pd(true); // Power down the PLL
        w.set_vcopd(true); // Power down the VCO
        w.set_postdivpd(true); // Power down the post divider
        w.set_dsmpd(true); // Disable fractional mode
    });

    // Short delay after powering down
    crate::cpu::delay(10);

    // 2. Configure reference divider first
    p.cs().write(|w| w.set_refdiv(config.refdiv as _));

    // 3. Configure feedback divider
    p.fbdiv_int().write(|w| w.set_fbdiv_int(config.fbdiv));

    // 4. Power up PLL and VCO, but keep post divider powered down during initial lock
    p.pwr().write(|w| {
        w.set_pd(false); // Power up the PLL
        w.set_vcopd(false); // Power up the VCO
        w.set_postdivpd(true); // Keep post divider powered down during initial lock
        w.set_dsmpd(true); // Disable fractional mode (simpler configuration)
    });

    // 5. Wait for PLL to lock with a timeout
    let mut timeout = 1_000_000;
    while !p.cs().read().lock() {
        timeout -= 1;
        if timeout == 0 {
            // PLL failed to lock, return 0 to indicate failure
            return Err(ClockError::PllLockTimedOut);
        }
    }

    // 6. Configure post dividers after PLL is locked
    p.prim().write(|w| {
        w.set_postdiv1(config.post_div1);
        w.set_postdiv2(config.post_div2);
    });

    // 7. Enable the post divider output
    p.pwr().modify(|w| {
        w.set_postdivpd(false); // Power up post divider
    });

    // Final delay to ensure everything is stable
    crate::cpu::delay(100);

    // Calculate and return actual output frequency
    Ok(vco_freq / ((config.post_div1 * config.post_div2) as u32))
}

/// General purpose input clock pin.
pub trait GpinPin: crate::gpio::Pin {
    /// Pin number.
    const NR: usize;
}

macro_rules! impl_gpinpin {
    ($name:ident, $gpin_num:expr) => {
        impl GpinPin for crate::peripherals::$name {
            const NR: usize = $gpin_num;
        }
    };
}

#[cfg(feature = "_rp235x")]
impl_gpinpin!(PIN_12, 0);
#[cfg(feature = "_rp235x")]
impl_gpinpin!(PIN_14, 1);

/// GPCK function select.
#[cfg(feature = "_rp235x")]
const GPCLK_FUNCSEL: u8 = 0x09;

/// General purpose clock input driver.
pub struct Gpin<'d, T: GpinPin> {
    _gpin: Flex<'d>,
    _phantom: PhantomData<T>,
}

impl<'d, T: GpinPin> Gpin<'d, T> {
    /// Create new gpin driver.
    pub fn new(gpin: Peri<'d, T>) -> Self {
        let mut gpin = Flex::new_unconfigured(gpin);
        gpin.set_function(GPCLK_FUNCSEL);
        #[cfg(feature = "_rp235x")]
        gpin.write_pad(|_| {});

        Gpin {
            _gpin: gpin,
            _phantom: PhantomData,
        }
    }
}

/// General purpose clock output pin.
pub trait GpoutPin: crate::gpio::Pin {
    /// Pin number.
    fn number(&self) -> usize;
}

macro_rules! impl_gpoutpin {
    ($name:ident, $gpout_num:expr) => {
        impl GpoutPin for crate::peripherals::$name {
            fn number(&self) -> usize {
                $gpout_num
            }
        }
    };
}

#[cfg(feature = "_rp235x")]
impl_gpoutpin!(PIN_13, 0);
#[cfg(feature = "_rp235x")]
impl_gpoutpin!(PIN_15, 1);

/// Gpout clock source.
#[repr(u8)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum GpoutSrc {
    /// Sys PLL.
    PllSys = ClkGpoutCtrlAuxsrc::ClksrcPllSys as _,
    // See above re gpin handling being commented out
    // Gpin0 = ClkGpoutCtrlAuxsrc::CLKSRC_GPIN0 as _ ,
    // Gpin1 = ClkGpoutCtrlAuxsrc::CLKSRC_GPIN1 as _ ,
    /// USB PLL.
    PllUsb = ClkGpoutCtrlAuxsrc::ClksrcPllUsb as _,
    /// ROSC.
    Rosc = ClkGpoutCtrlAuxsrc::RoscClksrc as _,
    /// XOSC.
    Xosc = ClkGpoutCtrlAuxsrc::XoscClksrc as _,
    /// LPOSC.
    #[cfg(feature = "_rp235x")]
    Lposc = ClkGpoutCtrlAuxsrc::LposcClksrc as _,
    /// SYS.
    Sys = ClkGpoutCtrlAuxsrc::ClkSys as _,
    /// USB.
    Usb = ClkGpoutCtrlAuxsrc::ClkUsb as _,
    /// ADC.
    Adc = ClkGpoutCtrlAuxsrc::ClkAdc as _,
    /// REF.
    Ref = ClkGpoutCtrlAuxsrc::ClkRef as _,
}

/// General purpose clock output driver.
pub struct Gpout<'d, T: GpoutPin> {
    _pin: Flex<'d>,
    number: u8,
    _phantom: PhantomData<T>,
}

impl<'d, T: GpoutPin> Gpout<'d, T> {
    /// Create new general purpose clock output.
    pub fn new(gpout: Peri<'d, T>) -> Self {
        let number = gpout.number() as u8;
        let mut pin = Flex::new_unconfigured(gpout);
        pin.set_function(GPCLK_FUNCSEL);
        #[cfg(feature = "_rp235x")]
        pin.write_pad(|_| {});

        Self {
            _pin: pin,
            number,
            _phantom: PhantomData,
        }
    }

    fn number(&self) -> usize {
        self.number as usize
    }

    /// Set clock divider.
    #[cfg(feature = "_rp235x")]
    pub fn set_div(&self, int: u16, frac: u16) {
        let c = pac::CLOCKS;
        c.clk_gpout_div(self.number()).write(|w| {
            w.set_int(int);
            w.set_frac(frac);
        });
    }

    /// Set clock source.
    pub fn set_src(&self, src: GpoutSrc) {
        let c = pac::CLOCKS;
        c.clk_gpout_ctrl(self.number()).modify(|w| {
            w.set_auxsrc(ClkGpoutCtrlAuxsrc::from_bits(src as _));
        });
    }

    /// Enable clock.
    pub fn enable(&self) {
        let c = pac::CLOCKS;
        c.clk_gpout_ctrl(self.number()).modify(|w| {
            w.set_enable(true);
        });
    }

    /// Disable clock.
    pub fn disable(&self) {
        let c = pac::CLOCKS;
        c.clk_gpout_ctrl(self.number()).modify(|w| {
            w.set_enable(false);
        });
    }

    /// Clock frequency.
    pub fn get_freq(&self) -> u32 {
        let c = pac::CLOCKS;
        let src = c.clk_gpout_ctrl(self.number()).read().auxsrc();

        let base = match src {
            ClkGpoutCtrlAuxsrc::ClksrcPllSys => pll_sys_freq(),
            // See above re gpin handling being commented out
            // ClkGpoutCtrlAuxsrc::CLKSRC_GPIN0 => gpin0_freq(),
            // ClkGpoutCtrlAuxsrc::CLKSRC_GPIN1 => gpin1_freq(),
            ClkGpoutCtrlAuxsrc::ClksrcPllUsb => pll_usb_freq(),
            ClkGpoutCtrlAuxsrc::RoscClksrc => rosc_freq(),
            ClkGpoutCtrlAuxsrc::XoscClksrc => xosc_freq(),
            ClkGpoutCtrlAuxsrc::ClkSys => clk_sys_freq(),
            ClkGpoutCtrlAuxsrc::ClkUsb => clk_usb_freq(),
            ClkGpoutCtrlAuxsrc::ClkAdc => clk_adc_freq(),
            ClkGpoutCtrlAuxsrc::ClkRef => clk_ref_freq(),
            _ => unreachable!(),
        };

        let div = c.clk_gpout_div(self.number()).read();
        let int = if div.int() == 0 { 0xFFFF } else { div.int() } as u64;
        let frac = div.frac() as u64;

        ((base as u64 * 256) / (int * 256 + frac)) as u32
    }
}

impl<'d, T: GpoutPin> Drop for Gpout<'d, T> {
    fn drop(&mut self) {
        self.disable();
    }
}

/// Random number generator based on the ROSC RANDOMBIT register.
///
/// This will not produce random values if the ROSC is stopped or run at some
/// harmonic of the bus frequency. With default clock settings these are not
/// issues.
pub struct RoscRng;

impl RoscRng {
    /// Get a random u8
    pub fn next_u8() -> u8 {
        let random_reg = pac::ROSC.randombit();
        let mut acc = 0;
        for _ in 0..u8::BITS {
            acc <<= 1;
            acc |= random_reg.read().randombit() as u8;
        }
        acc
    }

    /// Get a random u32
    pub fn next_u32(&mut self) -> u32 {
        let mut bytes = [0; 4];
        self.fill_bytes(&mut bytes);
        u32::from_le_bytes(bytes)
    }

    /// Get a random u64
    pub fn next_u64(&mut self) -> u64 {
        let mut bytes = [0; 8];
        self.fill_bytes(&mut bytes);
        u64::from_le_bytes(bytes)
    }

    /// Fill a slice with random bytes
    pub fn fill_bytes(&mut self, dest: &mut [u8]) {
        dest.fill_with(Self::next_u8)
    }
}

impl rand_core_10::TryRng for RoscRng {
    type Error = core::convert::Infallible;

    fn try_next_u32(&mut self) -> Result<u32, Self::Error> {
        Ok(self.next_u32())
    }

    fn try_next_u64(&mut self) -> Result<u64, Self::Error> {
        Ok(self.next_u64())
    }

    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Self::Error> {
        self.fill_bytes(dest);
        Ok(())
    }
}

impl rand_core_10::TryCryptoRng for RoscRng {}
