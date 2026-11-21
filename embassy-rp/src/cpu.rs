//! CPU primitives for RISC-V (Hazard3).

#![allow(dead_code)]

mod imp {
    use core::arch::asm;

    #[inline(always)]
    pub fn nop() {
        unsafe { asm!("nop") }
    }

    /// Busy-wait for *at least* `cycles` cycles (each iteration takes >= 1 cycle).
    #[inline(always)]
    pub fn delay(cycles: u32) {
        let mut i = 0;
        while i < cycles {
            nop();
            i += 1;
        }
    }

    /// Hazard3 `h3.unblock` (Xh3power): wake harts blocked with `h3.block`.
    /// Encoded as `slt x0, x0, x1`, same as pico-sdk's `__sev()`.
    #[inline(always)]
    pub fn sev() {
        unsafe { asm!("slt x0, x0, x1") }
    }

    /// Hazard3 `h3.block` (Xh3power): sleep until `h3.unblock` or an interrupt.
    /// Encoded as `slt x0, x0, x0`, same as pico-sdk's `__wfe()`.
    #[inline(always)]
    pub fn wfe() {
        unsafe { asm!("slt x0, x0, x0") }
    }

    #[inline(always)]
    pub fn udf() -> ! {
        unsafe { asm!("unimp", options(noreturn)) }
    }

    /// True if machine interrupts are enabled (mstatus.MIE, bit 3).
    #[inline(always)]
    pub fn interrupts_enabled() -> bool {
        let mstatus: usize;
        unsafe { asm!("csrr {0}, mstatus", out(reg) mstatus) };
        mstatus & (1 << 3) != 0
    }

    /// Clear mstatus.MIE. (No `nomem`: acts as a compiler barrier for critical sections.)
    #[inline(always)]
    pub fn interrupt_disable() {
        unsafe { asm!("csrci mstatus, 8") }
    }

    /// Set mstatus.MIE.
    #[inline(always)]
    pub unsafe fn interrupt_enable() {
        asm!("csrsi mstatus, 8")
    }

    /// Trap vector base of the current core (mtvec).
    #[inline(always)]
    pub fn vector_table_base() -> u32 {
        let v: usize;
        unsafe { asm!("csrr {0}, mtvec", out(reg) v) };
        v as u32
    }

    /// Enable machine external interrupts on the current core: mie.MEIE (bit 11) and mstatus.MIE.
    ///
    /// riscv-rt leaves both cleared at boot, so without this no IRQ ever fires.
    #[inline(always)]
    pub unsafe fn enable_external_irq() {
        asm!("csrs mie, {0}", in(reg) (1usize << 11));
        interrupt_enable();
    }
}

pub(crate) use imp::*;
