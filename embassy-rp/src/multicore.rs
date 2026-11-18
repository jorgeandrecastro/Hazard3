//! Multicore support
//!
//! This module handles setup of the 2nd cpu core on the rp2040, which we refer to as core1.
//! It provides functionality for setting up the stack, and starting core1.
//!
//! The entrypoint for core1 can be any function that never returns, including closures.
//!
//! Enable the `critical-section-impl` feature in embassy-rp when sharing data across cores using
//! the `embassy-sync` primitives and `CriticalSectionRawMutex`.
//!
//! # Usage
//!
//! ```no_run
//! use embassy_rp::multicore::Stack;
//! use static_cell::StaticCell;
//! use embassy_executor::Executor;
//! use core::ptr::addr_of_mut;
//!
//! static mut CORE1_STACK: Stack<4096> = Stack::new();
//! static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
//! static EXECUTOR1: StaticCell<Executor> = StaticCell::new();
//!
//! # // workaround weird error: `main` function not found in crate `rust_out`
//! # let _ = ();
//!
//! #[embassy_executor::task]
//! async fn core0_task() {
//!     // ...
//! }
//!
//! #[embassy_executor::task]
//! async fn core1_task() {
//!     // ...
//! }
//!
//! #[cortex_m_rt::entry]
//! fn main() -> ! {
//!     let p = embassy_rp::init(Default::default());
//!
//!     embassy_rp::multicore::spawn_core1(p.CORE1, unsafe { &mut *addr_of_mut!(CORE1_STACK) }, move || {
//!         let executor1 = EXECUTOR1.init(Executor::new());
//!         executor1.run(|spawner| spawner.spawn(core1_task().unwrap()));
//!     });
//!
//!     let executor0 = EXECUTOR0.init(Executor::new());
//!     executor0.run(|spawner| spawner.spawn(core0_task().unwrap()))
//! }
//! ```

use core::mem::ManuallyDrop;
use core::sync::atomic::{AtomicBool, Ordering, compiler_fence};

#[cfg(all(feature = "rt", target_arch = "arm", any(feature = "rp2040", feature = "_rp235x")))]
use cortex_m::interrupt::InterruptNumber;
#[cfg(all(feature = "rt", target_arch = "arm", any(feature = "rp2040", feature = "_rp235x")))]
use cortex_m::peripheral::NVIC;

use crate::interrupt::InterruptExt;
use crate::peripherals::CORE1;
use crate::{Peri, install_stack_guard, interrupt, pac};

const PAUSE_TOKEN: u32 = 0xDEADBEEF;
const RESUME_TOKEN: u32 = !0xDEADBEEF;
#[cfg(all(feature = "rt", any(feature = "rp2040", feature = "_rp235x")))]
pub(crate) const PEND_IRQ_TOKEN: u32 = 0xCAFE0000;

static IS_CORE1_INIT: AtomicBool = AtomicBool::new(false);

/// Represents a particular CPU core (SIO_CPUID)
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(u8)]
pub enum CoreId {
    /// Core 0
    Core0 = 0x0,
    /// Core 1
    Core1 = 0x1,
}

/// Gets which core we are currently executing from
pub fn current_core() -> CoreId {
    if pac::SIO.cpuid().read() == 0 {
        CoreId::Core0
    } else {
        CoreId::Core1
    }
}

#[inline(always)]
unsafe fn core1_setup(stack_bottom: *mut usize) {
    if install_stack_guard(stack_bottom).is_err() {
        // currently only happens if the MPU was already set up, which
        // would indicate that the core is already in use from outside
        // embassy, somehow. trap if so since we can't deal with that.
        crate::cpu::udf();
    }

    #[cfg(feature = "_rp235x")]
    crate::enable_actlr_extexclall();
}

/// Data type for a properly aligned stack of N bytes
#[repr(C, align(32))]
pub struct Stack<const SIZE: usize> {
    /// Memory to be used for the stack
    pub mem: [u8; SIZE],
}

impl<const SIZE: usize> Default for Stack<SIZE> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const SIZE: usize> Stack<SIZE> {
    /// Construct a stack of length SIZE, initialized to 0
    pub const fn new() -> Stack<SIZE> {
        Stack { mem: [0_u8; SIZE] }
    }
}

#[cfg(all(feature = "rt", target_arch = "arm", any(feature = "rp2040", feature = "_rp235x")))]
#[derive(Clone, Copy)]
struct Irq(u16);
#[cfg(all(feature = "rt", target_arch = "arm", any(feature = "rp2040", feature = "_rp235x")))]
unsafe impl InterruptNumber for Irq {
    fn number(self) -> u16 {
        self.0
    }
}

/// Pend an interrupt on the current core (used to wake interrupt executors from the other core).
#[cfg(all(feature = "rt", any(feature = "rp2040", feature = "_rp235x")))]
#[inline(always)]
unsafe fn pend_irq(n: u16) {
    #[cfg(all(target_arch = "arm", feature = "rp2040"))]
    NVIC::pend(Irq(n));

    #[cfg(all(target_arch = "arm", feature = "_rp235x"))]
    {
        let mut nvic: NVIC = core::mem::transmute(());
        nvic.request(Irq(n));
    }

    #[cfg(not(target_arch = "arm"))]
    embassy_hal_internal::interrupt::irq_pend(n);
}

/// Body of the SIO FIFO interrupt, shared by all chips/architectures.
#[cfg(all(feature = "rt", any(feature = "rp2040", feature = "_rp235x")))]
#[inline(always)]
unsafe fn fifo_irq_handler() {
    let sio = pac::SIO;
    // Clear IRQ
    sio.fifo().st().write(|w| w.set_wof(false));

    while sio.fifo().st().read().vld() {
        let fifo_read = fifo_read_wfe();
        if fifo_read == PAUSE_TOKEN {
            // Pause CORE1 execution and disable interrupts
            crate::cpu::interrupt_disable();
            // Signal to CORE0 that execution is paused
            fifo_write(PAUSE_TOKEN);
            // Wait for `resume` signal from CORE0
            while fifo_read_wfe() != RESUME_TOKEN {
                crate::cpu::nop();
            }
            crate::cpu::interrupt_enable();
            // Signal to CORE0 that execution is resumed
            fifo_write(RESUME_TOKEN);
        } else if fifo_read & 0xFFFF0000 == PEND_IRQ_TOKEN {
            // Pend the IRQ to wake up interrupt executors.
            pend_irq((fifo_read & 0xFFFF) as u16);
        }
    }
}

#[cfg(all(feature = "rt", feature = "rp2040"))]
#[interrupt]
unsafe fn SIO_IRQ_PROC1() {
    fifo_irq_handler()
}

#[cfg(all(feature = "rt", feature = "_rp235x", target_arch = "arm"))]
#[interrupt]
unsafe fn SIO_IRQ_FIFO() {
    fifo_irq_handler()
}

// RISC-V: no `#[interrupt]` attribute, the handler is a plain exported symbol picked up by the
// dispatch table generated by `interrupt_mod!`.
#[cfg(all(feature = "rt", feature = "_rp235x", not(target_arch = "arm")))]
#[unsafe(no_mangle)]
unsafe extern "C" fn SIO_IRQ_FIFO() {
    fifo_irq_handler()
}

// RISC-V: core 1 is started by the bootrom with only `sp` set (and nothing else), so we can't pass
// arguments in registers. `spawn_core1` pushes them on the new stack and this trampoline pops them:
//   [sp+0]  = pointer to the entry closure   -> a0
//   [sp+4]  = stack bottom                   -> a1
//   [sp+8]  = address of `core1_startup`     -> a2
//   [sp+12] = mtvec of core 0                -> mtvec
// It also sets up `gp`, which riscv-rt only does on core 0.
#[cfg(not(target_arch = "arm"))]
core::arch::global_asm!(
    r#"
    .section .text.__core1_trampoline, "ax"
    .global __core1_trampoline
    .type __core1_trampoline, @function
__core1_trampoline:
    .option push
    .option norelax
    la gp, __global_pointer$
    .option pop
    lw a0, 0(sp)
    lw a1, 4(sp)
    lw a2, 8(sp)
    lw a3, 12(sp)
    addi sp, sp, 16
    csrw mtvec, a3
    jr a2
    .size __core1_trampoline, . - __core1_trampoline
"#
);

#[cfg(not(target_arch = "arm"))]
unsafe extern "C" {
    fn __core1_trampoline();
}

/// Spawn a function on this core
pub fn spawn_core1<F, const SIZE: usize>(_core1: Peri<'static, CORE1>, stack: &'static mut Stack<SIZE>, entry: F)
where
    F: FnOnce() -> ! + Send + 'static,
{
    // Common body of core 1 startup, once `entry` and `stack_bottom` are known.
    unsafe fn core1_body<F: FnOnce() -> !>(entry: *mut ManuallyDrop<F>, stack_bottom: *mut usize) -> ! {
        core1_setup(stack_bottom);

        let entry = ManuallyDrop::take(&mut *entry);

        // make sure the preceding read doesn't get reordered past the following fifo write
        compiler_fence(Ordering::SeqCst);

        // Signal that it's safe for core 0 to get rid of the original value now.
        fifo_write(1);

        IS_CORE1_INIT.store(true, Ordering::Release);

        // RISC-V: interrupts are off at boot on this core, turn them on (no-op on ARM).
        crate::cpu::enable_external_irq();

        // Enable fifo interrupt on CORE1 for `pause` functionality.
        #[cfg(feature = "rp2040")]
        interrupt::SIO_IRQ_PROC1.enable();
        #[cfg(feature = "_rp235x")]
        interrupt::SIO_IRQ_FIFO.enable();

        // Enable FPU
        #[cfg(all(feature = "_rp235x", has_fpu, target_arch = "arm"))]
        {
            let p = cortex_m::Peripherals::steal();
            p.SCB.cpacr.modify(|cpacr| cpacr | (3 << 20) | (3 << 22));
        }

        entry()
    }

    // The first two ignored `u64` parameters are there to take up all of the registers,
    // which means that the rest of the arguments are taken from the stack,
    // where we're able to put them from core 0.
    #[cfg(target_arch = "arm")]
    extern "C" fn core1_startup<F: FnOnce() -> !>(
        _: u64,
        _: u64,
        entry: *mut ManuallyDrop<F>,
        stack_bottom: *mut usize,
    ) -> ! {
        unsafe { core1_body(entry, stack_bottom) }
    }

    // RISC-V: arguments arrive in a0/a1, loaded by `__core1_trampoline`.
    #[cfg(not(target_arch = "arm"))]
    extern "C" fn core1_startup<F: FnOnce() -> !>(entry: *mut ManuallyDrop<F>, stack_bottom: *mut usize) -> ! {
        unsafe { core1_body(entry, stack_bottom) }
    }

    // Reset the core
    let psm = pac::PSM;
    psm.frce_off().modify(|w| w.set_proc1(true));
    while !psm.frce_off().read().proc1() {
        crate::cpu::nop();
    }
    psm.frce_off().modify(|w| w.set_proc1(false));

    // The ARM AAPCS ABI requires 8-byte stack alignment (RISC-V ilp32: 16 bytes).
    // #[align] on `struct Stack` ensures the bottom is aligned, but the top could still be
    // unaligned if the user chooses a stack size that's not multiple of 8 (16).
    // So, we round down to the next multiple.
    #[cfg(target_arch = "arm")]
    let stack_words = stack.mem.len() / 8 * 2;
    #[cfg(not(target_arch = "arm"))]
    let stack_words = stack.mem.len() / 16 * 4;
    let mem = unsafe { core::slice::from_raw_parts_mut(stack.mem.as_mut_ptr() as *mut usize, stack_words) };

    // Set up the stack
    let mut stack_ptr = unsafe { mem.as_mut_ptr().add(mem.len()) };

    // We don't want to drop this, since it's getting moved to the other core.
    let mut entry = ManuallyDrop::new(entry);

    let vector_table = crate::cpu::vector_table_base();

    // Push the arguments to `core1_startup` onto the stack.
    #[cfg(target_arch = "arm")]
    unsafe {
        // Push `stack_bottom`.
        stack_ptr = stack_ptr.sub(1);
        stack_ptr.cast::<*mut usize>().write(mem.as_mut_ptr());

        // Push `entry`.
        stack_ptr = stack_ptr.sub(1);
        stack_ptr.cast::<*mut ManuallyDrop<F>>().write(&mut entry);
    }
    // RISC-V: 4 words popped by `__core1_trampoline` (see layout above).
    #[cfg(not(target_arch = "arm"))]
    unsafe {
        stack_ptr = stack_ptr.sub(4);
        stack_ptr.add(0).cast::<*mut ManuallyDrop<F>>().write(&mut entry);
        stack_ptr.add(1).write(mem.as_mut_ptr() as usize);
        stack_ptr.add(2).write(core1_startup::<F> as *const () as usize);
        stack_ptr.add(3).write(vector_table as usize);
    }

    // Make sure the compiler does not reorder the stack writes after to after the
    // below FIFO writes, which would result in them not being seen by the second
    // core.
    //
    // From the compiler perspective, this doesn't guarantee that the second core
    // actually sees those writes. However, we know that the RP2040 doesn't have
    // memory caches, and writes happen in-order.
    compiler_fence(Ordering::Release);

    // Where core 1 starts executing: the startup function itself on ARM, the trampoline on RISC-V.
    #[cfg(target_arch = "arm")]
    let entry_point = core1_startup::<F> as *const () as usize;
    #[cfg(not(target_arch = "arm"))]
    let entry_point = __core1_trampoline as *const () as usize;

    // After reset, core 1 is waiting to receive commands over FIFO.
    // This is the sequence to have it jump to some code.
    let cmd_seq = [0, 0, 1, vector_table as usize, stack_ptr as usize, entry_point];

    let mut seq = 0;
    let mut fails = 0;
    loop {
        let cmd = cmd_seq[seq] as u32;
        if cmd == 0 {
            fifo_drain();
            crate::cpu::sev();
        }
        fifo_write(cmd);

        let response = fifo_read();
        if cmd == response {
            seq += 1;
        } else {
            seq = 0;
            fails += 1;
            if fails > 16 {
                // The second core isn't responding, and isn't going to take the entrypoint
                panic!("CORE1 not responding");
            }
        }
        if seq >= cmd_seq.len() {
            break;
        }
    }

    // Wait until the other core has copied `entry` before returning.
    fifo_read();

    // Enable fifo interrupt on CORE0 for `pend irq` functionality.
    #[cfg(all(feature = "rp2040", feature = "executor-interrupt"))]
    unsafe {
        interrupt::SIO_IRQ_PROC1.enable()
    };
    #[cfg(all(feature = "_rp235x", feature = "executor-interrupt"))]
    unsafe {
        interrupt::SIO_IRQ_FIFO.enable()
    };
}

/// Pause execution on CORE1.
pub fn pause_core1() {
    if IS_CORE1_INIT.load(Ordering::Acquire) {
        fifo_write(PAUSE_TOKEN);
        // Wait for CORE1 to signal it has paused execution.
        while fifo_read() != PAUSE_TOKEN {}
    }
}

/// Resume CORE1 execution.
pub fn resume_core1() {
    if IS_CORE1_INIT.load(Ordering::Acquire) {
        fifo_write(RESUME_TOKEN);
        // Wait for CORE1 to signal it has resumed execution.
        while fifo_read() != RESUME_TOKEN {}
    }
}

// Push a value to the inter-core FIFO, block until space is available
#[inline(always)]
pub(crate) fn fifo_write(value: u32) {
    let sio = pac::SIO;
    // Wait for the FIFO to have enough space
    while !sio.fifo().st().read().rdy() {
        crate::cpu::nop();
    }
    sio.fifo().wr().write_value(value);
    // Fire off an event to the other core.
    // This is required as the other core may be `wfe` (waiting for event)
    crate::cpu::sev();
}

// Pop a value from inter-core FIFO, block until available
#[inline(always)]
fn fifo_read() -> u32 {
    let sio = pac::SIO;
    // Wait until FIFO has data
    while !sio.fifo().st().read().vld() {
        crate::cpu::nop();
    }
    sio.fifo().rd().read()
}

// Pop a value from inter-core FIFO, `wfe` until available
#[inline(always)]
#[allow(unused)]
fn fifo_read_wfe() -> u32 {
    let sio = pac::SIO;
    // Wait until FIFO has data
    while !sio.fifo().st().read().vld() {
        crate::cpu::wfe();
    }
    sio.fifo().rd().read()
}

// Drain inter-core FIFO
#[inline(always)]
fn fifo_drain() {
    let sio = pac::SIO;
    while sio.fifo().st().read().vld() {
        let _ = sio.fifo().rd().read();
    }
}