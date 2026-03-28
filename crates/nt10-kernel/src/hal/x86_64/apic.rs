//! Local APIC (xAPIC MMIO). Timer IRQ does not use the 8259 PIC when active.
//!
//! Requires the APIC MMIO region (typically `0xFEE0_0000`) to be mapped by firmware paging
//! (common under UEFI + QEMU/OVMF).

use core::sync::atomic::{AtomicU64, Ordering};

use crate::arch::x86_64::msr;

/// `IA32_APIC_BASE` MSR.
pub const MSR_IA32_APIC_BASE: u32 = 0x0000_001B;

static BSP_APIC_MMIO_PHYS: AtomicU64 = AtomicU64::new(0);

#[inline]
pub fn cached_mmio_phys() -> u64 {
    BSP_APIC_MMIO_PHYS.load(Ordering::Relaxed)
}

/// # Safety
/// `phys_base` must be the BSP LAPIC MMIO window (4 KiB aligned).
pub unsafe fn set_cached_mmio_phys(phys_base: u64) {
    BSP_APIC_MMIO_PHYS.store(phys_base, Ordering::Release);
}

#[inline]
unsafe fn read_mmio(phys_base: u64, offset: u32) -> u32 {
    let p = phys_base.checked_add(u64::from(offset)).unwrap() as *const u32;
    core::ptr::read_volatile(p)
}

#[inline]
unsafe fn write_mmio(phys_base: u64, offset: u32, val: u32) {
    let p = phys_base.checked_add(u64::from(offset)).unwrap() as *mut u32;
    core::ptr::write_volatile(p, val);
}

/// Spurious interrupt vector register offset.
const REG_SVR: u32 = 0x0F0;
/// End-of-interrupt.
const REG_EOI: u32 = 0x0B0;
/// LVT timer.
const REG_LVT_TIMER: u32 = 0x320;
/// Timer initial count.
const REG_TIMER_INIT: u32 = 0x380;
/// Timer divide configuration.
const REG_TIMER_DIV: u32 = 0x3E0;

/// APIC enabled bit in `IA32_APIC_BASE`.
const APIC_BASE_ENABLE: u64 = 1 << 11;

#[must_use]
pub unsafe fn bsp_mmio_phys_from_msr() -> u64 {
    let v = msr::rdmsr(MSR_IA32_APIC_BASE);
    v & 0xFFFF_FFFF_FFFF_F000
}

#[must_use]
pub unsafe fn apic_hw_enabled() -> bool {
    (msr::rdmsr(MSR_IA32_APIC_BASE) & APIC_BASE_ENABLE) != 0
}

/// Program the local timer in **periodic** mode with `vector` and `initial_count`.
///
/// Divide value `3` = divide-by-16 (Intel SDM encoding).
///
/// # Safety
/// LAPIC MMIO must be reachable at `phys_base`; SVR must allow APIC operation.
pub unsafe fn setup_lvt_timer_periodic(phys_base: u64, vector: u8, initial_count: u32) {
    // Spurious vector 0xFF + APIC software enable.
    write_mmio(phys_base, REG_SVR, 0x1FF);
    // Mask timer during setup.
    write_mmio(phys_base, REG_LVT_TIMER, 1 << 16);
    write_mmio(phys_base, REG_TIMER_DIV, 0x3);
    // Periodic (bit 17), vector, unmasked.
    write_mmio(
        phys_base,
        REG_LVT_TIMER,
        (1 << 17) | u32::from(vector & 0xFF),
    );
    write_mmio(phys_base, REG_TIMER_INIT, initial_count);
}

/// # Safety
/// Call from the timer ISR that was raised by this LAPIC.
pub unsafe fn send_eoi(phys_base: u64) {
    write_mmio(phys_base, REG_EOI, 0);
}

/// Try to arm the BSP LAPIC timer; on success caches MMIO base for [`send_eoi`] in the ISR.
///
/// Returns `false` if the APIC MSR is disabled or base is null.
pub fn try_init_bsp_timer(vector: u8, initial_count: u32) -> bool {
    unsafe {
        if !apic_hw_enabled() {
            return false;
        }
        let base = bsp_mmio_phys_from_msr();
        if base == 0 {
            return false;
        }
        setup_lvt_timer_periodic(base, vector, initial_count);
        set_cached_mmio_phys(base);
        true
    }
}

/// Read timer current count (debug).
#[must_use]
pub fn debug_timer_current_count() -> u32 {
    let base = cached_mmio_phys();
    if base == 0 {
        return 0;
    }
    unsafe { read_mmio(base, 0x390) }
}
