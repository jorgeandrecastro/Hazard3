//! Architecture-specific code: stack guard, `__pre_init`, ACTLR.
//!
//! ARM (Cortex-M0+/M33) and RISC-V (Hazard3) variants live side by side here,
//! selected with `target_arch`.

// ---------------------------------------------------------------------------
// Stack guard
// ---------------------------------------------------------------------------

/// Installs a stack guard for the CORE0 stack in MPU region 0.
/// Will fail if the MPU is already configured. This function requires
/// a `_stack_end` symbol to be defined by the linker script, and expects
/// `_stack_end` to be located at the lowest address (largest depth) of
/// the stack.
///
/// This method can *only* set up stack guards on the currently
/// executing core. Stack guards for CORE1 are set up automatically,
/// only CORE0 should ever use this.
///
/// On the RISC-V (Hazard3) cores there is no MPU nor MSPLIM: this function is a
/// no-op that always returns `Ok(())`.
///
/// # Usage
///
/// ```no_run
/// use embassy_rp::install_core0_stack_guard;
/// use embassy_executor::{Executor, Spawner};
///
/// #[embassy_executor::main]
/// async fn main(_spawner: Spawner) {
///     // set up by the linker as follows:
///     //
///     //     MEMORY {
///     //       STACK0: ORIGIN = 0x20040000, LENGTH = 4K
///     //     }
///     //
///     //     _stack_end = ORIGIN(STACK0);
///     //     _stack_start = _stack_end + LENGTH(STACK0);
///     //
///     install_core0_stack_guard().expect("MPU already configured");
///     let p = embassy_rp::init(Default::default());
///
///     // ...
/// }
/// ```
#[cfg(target_arch = "arm")]
pub fn install_core0_stack_guard() -> Result<(), ()> {
    unsafe extern "C" {
        static mut _stack_end: usize;
    }
    unsafe { install_stack_guard(core::ptr::addr_of_mut!(_stack_end)) }
}

/// Stack guard for CORE0 (RISC-V: no-op, there is no MPU/MSPLIM on Hazard3).
#[cfg(not(target_arch = "arm"))]
pub fn install_core0_stack_guard() -> Result<(), ()> {
    Ok(())
}

#[cfg(all(feature = "rp2040", target_arch = "arm", not(feature = "_test")))]
#[inline(always)]
pub(crate) unsafe fn install_stack_guard(stack_bottom: *mut usize) -> Result<(), ()> {
    let core = unsafe { cortex_m::Peripherals::steal() };

    // Fail if MPU is already configured
    if core.MPU.ctrl.read() != 0 {
        return Err(());
    }

    // The minimum we can protect is 32 bytes on a 32 byte boundary, so round up which will
    // just shorten the valid stack range a tad.
    let addr = (stack_bottom as u32 + 31) & !31;
    // Mask is 1 bit per 32 bytes of the 256 byte range... clear the bit for the segment we want
    let subregion_select = 0xff ^ (1 << ((addr >> 5) & 7));
    unsafe {
        core.MPU.ctrl.write(5); // enable mpu with background default map
        core.MPU.rbar.write((addr & !0xff) | (1 << 4)); // set address and update RNR
        core.MPU.rasr.write(
            1 // enable region
               | (0x7 << 1) // size 2^(7 + 1) = 256
               | (subregion_select << 8)
               | 0x10000000, // XN = disable instruction fetch; no other bits means no permissions
        );
    }
    Ok(())
}

#[cfg(all(feature = "_rp235x", target_arch = "arm", not(feature = "_test")))]
#[inline(always)]
pub(crate) unsafe fn install_stack_guard(stack_bottom: *mut usize) -> Result<(), ()> {
    // The RP2350 arm cores are cortex-m33 and can use the MSPLIM register to guard the end of stack.
    cortex_m::register::msplim::write(stack_bottom.addr() as u32);

    Ok(())
}

// This is to hack around cortex_m defaulting to ARMv7 when building tests,
// so the compile fails when we try to use ARMv8 peripherals.
#[cfg(all(feature = "_test", target_arch = "arm"))]
#[inline(always)]
pub(crate) unsafe fn install_stack_guard(_stack_bottom: *mut usize) -> Result<(), ()> {
    Ok(())
}

// RISC-V (Hazard3): no MPU nor MSPLIM, so there is no stack guard to install.
#[cfg(not(target_arch = "arm"))]
#[inline(always)]
#[allow(dead_code)]
pub(crate) unsafe fn install_stack_guard(_stack_bottom: *mut usize) -> Result<(), ()> {
    Ok(())
}

// ---------------------------------------------------------------------------
// __pre_init
// ---------------------------------------------------------------------------

// #[pre_init] code converted to ASM
//
// As per [https://docs.rs/cortex-m-rt/0.7.5/cortex_m_rt/#features] - a #[pre_init] macro is also
// provided to run a function before RAM initialisation, but its use is deprecated as it is not
// defined behaviour to execute Rust code before initialisation.
// It is still possible to create a custom pre_init function using assembly.
//
// SIO does not get reset when core0 is reset with either `scb::sys_reset()` or with SWD.
// Since we're using SIO spinlock 31 for the critical-section impl, this causes random
// hangs if we reset in the middle of a CS, because the next boot sees the spinlock
// as locked and waits forever.
//
// See https://github.com/embassy-rs/embassy/issues/1736
// and https://github.com/rp-rs/rp-hal/issues/292
// and https://matrix.to/#/!vhKMWjizPZBgKeknOo:matrix.org/$VfOkQgyf1PjmaXZbtycFzrCje1RorAXd8BQFHTl4d5M
//
// According to Raspberry Pi, this is considered Working As Intended, and not an errata,
// even though this behavior is different from every other ARM chip (sys_reset usually resets
// the *system* as its name implies, not just the current core).
//
// To fix this, reset SIO on boot. We must do this in pre_init because it's unsound to do it
// in `embassy_rp::init`, since the user could've acquired a CS by then. pre_init is guaranteed
// to run before any user code.
//
// A similar thing could happen with PROC1. It is unclear whether it's possible for PROC1
// to stay unreset through a PROC0 reset, so we reset it anyway just in case.
//
// Important info from PSM logic (from Luke Wren in above Matrix thread)
//
//     The logic is, each PSM stage is reset if either of the following is true:
//     - The previous stage is in reset and FRCE_ON is false
//     - FRCE_OFF is true
//
// The PSM order is SIO -> PROC0 -> PROC1.
// So, we have to force-on PROC0 to prevent it from getting reset when resetting SIO.
#[cfg(all(feature = "rt", feature = "rp2040", target_arch = "arm"))]
core::arch::global_asm!(
    r#"
    .syntax unified
    .section .text.__pre_init, "ax"
    .global __pre_init
    .type __pre_init, %function
    .thumb_func
__pre_init:
    // pac::PSM.frce_on().write_and_wait(|w| w.set_proc0(true));
    movs r1, #1
    lsls r1, r1, #15
    ldr r0, =0x40010000
    str r1, [r0]
.Lrp2040_wait_frce_on_set:
    ldr r2, [r0]
    cmp r2, r1
    bne .Lrp2040_wait_frce_on_set

    // pac::PSM.frce_off().write_and_wait(|w| {{
    //     w.set_sio(true);
    //     w.set_proc1(true);
    // }});
    movs r1, #5
    lsls r1, r1, #14
    str r1, [r0, #4]
.Lrp2040_wait_frce_off_set:
    ldr r2, [r0, #4]
    cmp r2, r1
    bne .Lrp2040_wait_frce_off_set

    // pac::PSM.frce_off().write_and_wait(|_| {{}});
    movs r1, #0
    str r1, [r0, #4]
.Lrp2040_wait_frce_off_clear:
    ldr r2, [r0, #4]
    cmp r2, r1
    bne .Lrp2040_wait_frce_off_clear

    // pac::PSM.frce_on().write_and_wait(|_| {{}});
    str r1, [r0]
.Lrp2040_wait_frce_on_clear:
    ldr r2, [r0]
    cmp r2, r1
    bne .Lrp2040_wait_frce_on_clear

    bx lr
    .size __pre_init, . - __pre_init
"#
);

// On RP235x, datasheet says "The FRCE_ON register is a development feature that does nothing in production devices."
// No idea why they removed it. Removing it means we can't use PSM to reset SIO, because it comes before
// PROC0, so we'd need FRCE_ON to prevent resetting ourselves.
//
// So we just unlock the spinlock manually.
//
// We can still use PSM to reset PROC1 since it comes after PROC0 in the state machine.
//
// The bootrom executes redundancy coprocessor (RCP) instructions, which fault unless the RCP is
// enabled in CPACR and its salt registers have been initialized. The bootrom does both on a normal
// boot, but not if a debugger loads the image and jumps to it directly after a reset. Core 1 also
// waits in the bootrom for its salt to become valid before it can be launched. So, like pico-sdk's
// crt0, enable the RCP and initialize the salts if nobody did it yet. This must happen before
// resetting PROC1.
#[cfg(all(feature = "rt", feature = "_rp235x", target_arch = "arm"))]
core::arch::global_asm!(
    r#"
    .syntax unified
    .section .text.__pre_init, "ax"
    .global __pre_init
    .type __pre_init, %function
    .thumb_func
__pre_init:
    // Enable full access to coprocessor 7 (RCP) in CPACR.
    ldr r0, =0xe000ed88
    ldr r1, [r0]
    orr r1, r1, #0xc000
    str r1, [r0]
    dsb
    isb

    // rcp_canary_status: sets N if the salt is valid. Writing a valid salt again is an RCP fault.
    mrc p7, #1, apsr_nzcv, c0, c0, #0
    bmi .Lrp235x_rcp_salt_valid
    // rcp_salt_core0, rcp_salt_core1. The value doesn't matter much: we only get here if the
    // bootrom didn't run, which a secure boot would have done.
    mcrr p7, #8, r0, r0, c0
    mcrr p7, #8, r0, r0, c1
    // Wake core 1 if it's waiting for its salt.
    sev
.Lrp235x_rcp_salt_valid:

    // pac::SIO.spinlock(31).write_value(1);
    ldr r0, =0xd000017c
    movs r1, #1
    str r1, [r0]

    // pac::PSM.frce_off().write_and_wait(|w| w.set_proc1(true));
    ldr r0, =0x40018004
    lsls r1, r1, #24
    str r1, [r0]
.Lrp235x_wait_frce_off_set:
    ldr r2, [r0]
    cmp r2, r1
    bne .Lrp235x_wait_frce_off_set

    // pac::PSM.frce_off().write_and_wait(|_| {{}});
    movs r1, #0
    str r1, [r0]
.Lrp235x_wait_frce_off_clear:
    ldr r2, [r0]
    cmp r2, r1
    bne .Lrp235x_wait_frce_off_clear

    // (&*cortex_m::peripheral::ICB::PTR).actlr.modify(|w| w | (1 << 29));
    ldr r0, =0xe000e008
    ldr r1, [r0]
    movs r2, #1
    lsls r2, r2, #29
    orrs r1, r2
    str r1, [r0]

    bx lr
    .size __pre_init, . - __pre_init
"#
);

// RISC-V (Hazard3) version of __pre_init for RP2350, called by riscv-rt before RAM init.
//
// No RCP (ARM-only) and no ACTLR (ARM-only) here. We only:
//  - release SIO spinlock 31 (used by the critical-section impl), as on ARM;
//  - pulse PSM.FRCE_OFF.PROC1 to make sure core 1 is reset.
#[cfg(all(feature = "rt", feature = "_rp235x", target_arch = "riscv32"))]
core::arch::global_asm!(
    r#"
    .section .text.__pre_init, "ax"
    .global __pre_init
    .type __pre_init, @function
__pre_init:
    # pac::SIO.spinlock(31).write_value(1);
    li t0, 0xd000017c
    li t1, 1
    sw t1, 0(t0)

    # pac::PSM.frce_off().write_and_wait(|w| w.set_proc1(true));
    li t0, 0x40018004
    li t1, 0x01000000
    sw t1, 0(t0)
1:
    lw t2, 0(t0)
    bne t2, t1, 1b

    # pac::PSM.frce_off().write_and_wait(|_| {{}});
    sw zero, 0(t0)
2:
    lw t2, 0(t0)
    bnez t2, 2b

    ret
    .size __pre_init, . - __pre_init
"#
);

// ---------------------------------------------------------------------------
// ACTLR.EXTEXCLALL
// ---------------------------------------------------------------------------

/// Set the EXTEXCLALL bit in ACTLR.
///
/// The default MPU memory map marks all memory as non-shareable, so atomics don't
/// synchronize memory accesses between cores at all. This bit forces all memory to be
/// considered shareable regardless of what the MPU says.
///
/// TODO: does this interfere somehow if the user wants to use a custom MPU configuration?
/// maybe we need to add a way to disable this?
///
/// This must be done FOR EACH CORE. Core 0 does this in `__pre_init`; this function is
/// called after runtime initialization for core 1.
#[cfg(all(feature = "_rp235x", target_arch = "arm"))]
pub(crate) unsafe fn enable_actlr_extexclall() {
    (&*cortex_m::peripheral::ICB::PTR).actlr.modify(|w| w | (1 << 29));
}

/// RISC-V: ACTLR/EXTEXCLALL doesn't exist on Hazard3, nothing to do.
#[cfg(all(feature = "_rp235x", not(target_arch = "arm")))]
#[allow(dead_code)]
pub(crate) unsafe fn enable_actlr_extexclall() {}