//! Interrupt handling for RISC-V devices using the Hazard3 `Xh3irq` extension (RP2350).
//!
//! Hazard3 has no NVIC/PLIC. External interrupts are all routed to the machine external
//! interrupt (`mcause` = 11) and are managed through custom CSRs, accessed as "windowed arrays":
//!
//! | CSR      | addr  | purpose                                   | window            |
//! |----------|-------|-------------------------------------------|-------------------|
//! | `meiea`  | 0xbe0 | external interrupt enable                 | 16 IRQs / window  |
//! | `meipa`  | 0xbe1 | external interrupt pending (read-only)    | 16 IRQs / window  |
//! | `meifa`  | 0xbe2 | external interrupt force (software pend)  | 16 IRQs / window  |
//! | `meipra` | 0xbe3 | external interrupt priority (4 bits each) | 4 IRQs / window   |
//! | `meinext`| 0xbe4 | highest-priority pending IRQ number       | -                 |
//!
//! For the array CSRs, bits 4:0 are the window index and bits 31:16 are the window data.
//!
//! NOTE: written from the Hazard3 / RP2350 documentation, verify against the datasheet
//! (section "Hazard3 Xh3irq") if you see odd behaviour.
use core::arch::asm;
use core::mem;
use core::sync::atomic::{Ordering, compiler_fence};

use critical_section::CriticalSection;

// Window read: `csrrs` with only the index bits set leaves the data bits untouched and returns
// the data of the selected window in bits 31:16.
macro_rules! read_window {
    ($csr:literal, $index:expr) => {{
        let v: u32;
        unsafe { asm!(concat!("csrrs {0}, ", $csr, ", {1}"), out(reg) v, in(reg) $index, options(nostack)) };
        v >> 16
    }};
}

#[inline(always)]
fn window16(irq: u16) -> u32 {
    (irq / 16) as u32
}

#[inline(always)]
fn bit16(irq: u16) -> u32 {
    1u32 << (irq % 16)
}

/// Enable an external interrupt (meiea).
#[inline]
pub fn irq_enable(irq: u16) {
    compiler_fence(Ordering::SeqCst);
    let v = window16(irq) | (bit16(irq) << 16);
    unsafe { asm!("csrs 0xbe0, {0}", in(reg) v, options(nostack)) };
}

/// Disable an external interrupt (meiea).
#[inline]
pub fn irq_disable(irq: u16) {
    let v = window16(irq) | (bit16(irq) << 16);
    unsafe { asm!("csrc 0xbe0, {0}", in(reg) v, options(nostack)) };
    compiler_fence(Ordering::SeqCst);
}

/// Check if an external interrupt is enabled (meiea).
#[inline]
pub fn irq_is_enabled(irq: u16) -> bool {
    read_window!("0xbe0", window16(irq)) & bit16(irq) != 0
}

/// Check if an external interrupt is pending (meipa).
#[inline]
pub fn irq_is_pending(irq: u16) -> bool {
    read_window!("0xbe1", window16(irq)) & bit16(irq) != 0
}

/// Force an interrupt pending from software (meifa).
#[inline]
pub fn irq_pend(irq: u16) {
    let v = window16(irq) | (bit16(irq) << 16);
    unsafe { asm!("csrs 0xbe2, {0}", in(reg) v, options(nostack)) };
}

/// Clear the software-forced pending state (meifa).
#[inline]
pub fn irq_unpend(irq: u16) {
    let v = window16(irq) | (bit16(irq) << 16);
    unsafe { asm!("csrc 0xbe2, {0}", in(reg) v, options(nostack)) };
}

/// Get the priority of an interrupt (meipra, 4 bits per IRQ).
#[inline]
pub fn irq_get_priority(irq: u16) -> Priority {
    let data = read_window!("0xbe3", (irq / 4) as u32);
    let shift = 4 * (irq % 4) as u32;
    Priority::from(((data >> shift) & 0xf) as u8)
}

/// Set the priority of an interrupt, with an already-acquired critical section.
///
/// This is a read-modify-write of a 16-bit window, so it must not be interrupted.
#[inline]
pub fn irq_set_priority_with_cs(_cs: CriticalSection, irq: u16, prio: Priority) {
    let idx = (irq / 4) as u32;
    let shift = 4 * (irq % 4) as u32;
    let mut data = read_window!("0xbe3", idx);
    data = (data & !(0xf << shift)) | ((u8::from(prio) as u32) << shift);
    let v = idx | (data << 16);
    unsafe { asm!("csrw 0xbe3, {0}", in(reg) v, options(nostack)) };
}

/// Set the priority of an interrupt.
#[inline]
pub fn irq_set_priority(irq: u16, prio: Priority) {
    critical_section::with(|cs| irq_set_priority_with_cs(cs, irq, prio))
}

/// Number of the highest-priority pending enabled interrupt (meinext, no update), if any.
#[inline]
pub fn irq_next() -> Option<u16> {
    let v: u32;
    unsafe { asm!("csrr {0}, 0xbe4", out(reg) v, options(nostack)) };
    if v & 0x8000_0000 != 0 {
        None
    } else {
        Some(((v >> 2) & 0x1ff) as u16)
    }
}

/// Generate a standard `mod interrupt` for a HAL (RISC-V version).
///
/// Besides the `interrupt` module, with `rt` enabled this also generates:
/// - a weak default handler for every interrupt (overridden by `bind_interrupts!` or a
///   `#[unsafe(no_mangle)] extern "C" fn <IRQ>()` of your own),
/// - a dispatch table indexed by IRQ number,
/// - the `MachineExternal` trap handler (symbol expected by `riscv-rt`) that reads `meinext`
///   and calls the right handler.
#[macro_export]
macro_rules! interrupt_mod {
    ($($irqs:ident),* $(,)?) => {
        /// Interrupt definitions.
        pub mod interrupt {
            pub use $crate::interrupt::Priority;
            pub use crate::pac::Interrupt::*;
            pub use crate::pac::Interrupt;

            /// Operations on interrupts.
            ///
            /// Defined here (and not in `embassy-hal-internal`) because it is implemented for the
            /// PAC's `Interrupt` type, which the orphan rule only allows for a trait local to the HAL.
            pub unsafe trait InterruptExt: Copy {
                /// IRQ number.
                fn number(self) -> u16;

                /// Enable the interrupt.
                #[inline]
                unsafe fn enable(self) {
                    $crate::interrupt::irq_enable(self.number())
                }

                /// Disable the interrupt.
                #[inline]
                fn disable(self) {
                    $crate::interrupt::irq_disable(self.number())
                }

                /// Check if interrupt is enabled.
                #[inline]
                fn is_enabled(self) -> bool {
                    $crate::interrupt::irq_is_enabled(self.number())
                }

                /// Check if interrupt is pending.
                #[inline]
                fn is_pending(self) -> bool {
                    $crate::interrupt::irq_is_pending(self.number())
                }

                /// Set interrupt pending.
                #[inline]
                fn pend(self) {
                    $crate::interrupt::irq_pend(self.number())
                }

                /// Unset interrupt pending.
                #[inline]
                fn unpend(self) {
                    $crate::interrupt::irq_unpend(self.number())
                }

                /// Get the priority of the interrupt.
                #[inline]
                fn get_priority(self) -> Priority {
                    $crate::interrupt::irq_get_priority(self.number())
                }

                /// Set the interrupt priority.
                #[inline]
                fn set_priority(self, prio: Priority) {
                    $crate::interrupt::irq_set_priority(self.number(), prio)
                }

                /// Set the interrupt priority with an already-acquired critical section.
                #[inline]
                fn set_priority_with_cs(self, cs: critical_section::CriticalSection, prio: Priority) {
                    $crate::interrupt::irq_set_priority_with_cs(cs, self.number(), prio)
                }
            }

            unsafe impl InterruptExt for Interrupt {
                #[inline]
                fn number(self) -> u16 {
                    self as u16
                }
            }

            /// Type-level interrupt infrastructure.
            ///
            /// This module contains one *type* per interrupt. This is used for checking at compile time that
            /// the interrupts are correctly bound to HAL drivers.
            ///
            /// As an end user, you shouldn't need to use this module directly. Use the [`crate::bind_interrupts!`] macro
            /// to bind interrupts, and the [`crate::interrupt`] module to manually register interrupt handlers and manipulate
            /// interrupts directly (pending/unpending, enabling/disabling, setting the priority, etc...)
            pub mod typelevel {
                use super::InterruptExt;

                trait SealedInterrupt {}

                /// Type-level interrupt.
                ///
                /// This trait is implemented for all typelevel interrupt types in this module.
                pub trait Interrupt: SealedInterrupt {
                    /// Interrupt enum variant.
                    ///
                    /// This allows going from typelevel interrupts (one type per interrupt) to
                    /// non-typelevel interrupts (a single `Interrupt` enum type, with one variant per interrupt).
                    const IRQ: super::Interrupt;

                    /// Enable the interrupt.
                    #[inline]
                    unsafe fn enable() {
                        Self::IRQ.enable()
                    }

                    /// Disable the interrupt.
                    #[inline]
                    fn disable() {
                        Self::IRQ.disable()
                    }

                    /// Check if interrupt is enabled.
                    #[inline]
                    fn is_enabled() -> bool {
                        Self::IRQ.is_enabled()
                    }

                    /// Check if interrupt is pending.
                    #[inline]
                    fn is_pending() -> bool {
                        Self::IRQ.is_pending()
                    }

                    /// Set interrupt pending.
                    #[inline]
                    fn pend() {
                        Self::IRQ.pend()
                    }

                    /// Unset interrupt pending.
                    #[inline]
                    fn unpend() {
                        Self::IRQ.unpend()
                    }

                    /// Get the priority of the interrupt.
                    #[inline]
                    fn get_priority() -> crate::interrupt::Priority {
                        Self::IRQ.get_priority()
                    }

                    /// Set the interrupt priority.
                    #[inline]
                    fn set_priority(prio: crate::interrupt::Priority) {
                        Self::IRQ.set_priority(prio)
                    }

                    /// Set the interrupt priority with an already-acquired critical section
                    #[inline]
                    fn set_priority_with_cs(cs: critical_section::CriticalSection, prio: crate::interrupt::Priority) {
                        Self::IRQ.set_priority_with_cs(cs, prio)
                    }
                }

                $(
                    #[allow(non_camel_case_types)]
                    #[doc=stringify!($irqs)]
                    #[doc=" typelevel interrupt."]
                    pub enum $irqs {}
                    impl SealedInterrupt for $irqs{}
                    impl Interrupt for $irqs {
                        const IRQ: super::Interrupt = super::Interrupt::$irqs;
                    }
                )*

                /// Interrupt handler trait.
                ///
                /// Drivers that need to handle interrupts implement this trait.
                /// The user must ensure `on_interrupt()` is called every time the interrupt fires.
                /// Drivers must use use [`Binding`] to assert at compile time that the user has done so.
                pub trait Handler<I: Interrupt> {
                    /// Interrupt handler function.
                    ///
                    /// Must be called every time the `I` interrupt fires, synchronously from
                    /// the interrupt handler context.
                    ///
                    /// # Safety
                    ///
                    /// This function must ONLY be called from the interrupt handler for `I`.
                    unsafe fn on_interrupt();
                }

                /// Compile-time assertion that an interrupt has been bound to a handler.
                ///
                /// For the vast majority of cases, you should use the `bind_interrupts!`
                /// macro instead of writing `unsafe impl`s of this trait.
                ///
                /// # Safety
                ///
                /// By implementing this trait, you are asserting that you have arranged for `H::on_interrupt()`
                /// to be called every time the `I` interrupt fires.
                ///
                /// This allows drivers to check bindings at compile-time.
                pub unsafe trait Binding<I: Interrupt, H: Handler<I>>: Copy {}
            }
        }

        // ---- Vector dispatch (RISC-V has a single machine external interrupt vector) ----
        #[cfg(feature = "riscv")]
        #[doc(hidden)]
        mod __embassy_irq_dispatch {
            // Handlers not defined by anyone are resolved at link time by `PROVIDE(<IRQ> = __embassy_default_irq)`
            // lines (see `embassy_rp_irq.x`), the same way cortex-m-rt's `device.x` works.
            // They cannot be weak symbols defined here: this crate defines some handlers itself
            // (timer, SIO FIFO) and a weak and a strong definition in the same crate conflict.

            /// Called when an interrupt fires that nobody bound a handler to.
            #[unsafe(no_mangle)]
            unsafe extern "C" fn __embassy_default_irq() {
                // An enabled interrupt without handler is a bug: hang so it is noticed.
                loop {
                    core::hint::spin_loop();
                }
            }

            unsafe extern "C" {
                $( fn $irqs(); )*
            }

            // Dispatch table, indexed by IRQ number (RP2350 has 52 IRQs; 64 leaves room).
            static IRQ_TABLE: [unsafe extern "C" fn(); 64] = {
                let mut t = [__embassy_default_irq as unsafe extern "C" fn(); 64];
                $( t[crate::pac::Interrupt::$irqs as usize] = $irqs; )*
                t
            };

            /// Machine external interrupt: symbol looked up by `riscv-rt`'s trap handler.
            ///
            /// Reads `meinext` (without update) and runs the handler of the highest-priority pending
            /// interrupt. If more are pending, `mip.MEIP` stays set and the trap is taken again.
            #[unsafe(no_mangle)]
            unsafe extern "C" fn MachineExternal() {
                if let Some(irq) = $crate::interrupt::irq_next() {
                    (IRQ_TABLE[irq as usize])();
                }
            }
        }
    };
}

impl From<u8> for Priority {
    fn from(priority: u8) -> Self {
        // SAFETY: masked to 4 bits, all 16 values are valid variants.
        unsafe { mem::transmute(priority & 0xf) }
    }
}

impl From<Priority> for u8 {
    fn from(p: Priority) -> Self {
        p as u8
    }
}

/// The interrupt priority level (Hazard3: 4 bits, 0 = lowest, 15 = highest).
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
#[allow(missing_docs)]
pub enum Priority {
    P0 = 0,
    P1 = 1,
    P2 = 2,
    P3 = 3,
    P4 = 4,
    P5 = 5,
    P6 = 6,
    P7 = 7,
    P8 = 8,
    P9 = 9,
    P10 = 10,
    P11 = 11,
    P12 = 12,
    P13 = 13,
    P14 = 14,
    P15 = 15,
}