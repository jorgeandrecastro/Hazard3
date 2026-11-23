//! Hazard3 startup and stack guard support.

/// Hazard3 has no MPU or MSPLIM, so no stack guard is installed.
pub fn install_core0_stack_guard() -> Result<(), ()> {
    Ok(())
}

#[inline(always)]
#[allow(dead_code)]
pub(crate) unsafe fn install_stack_guard(_stack_bottom: *mut usize) -> Result<(), ()> {
    Ok(())
}

// Called before RAM initialization. Release the critical-section spinlock and
// reset core 1, which may have survived a reset of core 0.
#[cfg(all(feature = "rt", feature = "_rp235x", target_arch = "riscv32"))]
core::arch::global_asm!(
    r#"
    .section .text.__pre_init, "ax"
    .global __pre_init
    .type __pre_init, @function
__pre_init:
    li t0, 0xd000017c
    li t1, 1
    sw t1, 0(t0)

    li t0, 0x40018004
    li t1, 0x01000000
    sw t1, 0(t0)
1:
    lw t2, 0(t0)
    bne t2, t1, 1b

    sw zero, 0(t0)
2:
    lw t2, 0(t0)
    bnez t2, 2b

    ret
    .size __pre_init, . - __pre_init
"#
);
