//! TLB maintenance (BSP bring-up; AP IPI deferred).
//!
//! Multi-processor shootdown: use [`shootdown_range_all_cpus`] — today it forwards to the BSP path only;
//! AP [`invlpg`] via IPI is left for SMP bring-up.

use core::arch::asm;

/// Single `invlpg` for a virtual address (4 KiB granularity).
///
/// # Safety
/// `va` must be canonical for the current address width.
#[inline]
pub unsafe fn invlpg(va: u64) {
    unsafe {
        asm!("invlpg [{}]", in(reg) va, options(nostack));
    }
}

/// Invalidate TLB entries covering `[va_start, va_end)` at 4 KiB steps (BSP only).
pub fn shootdown_range(va_start: u64, va_end: u64) {
    if va_start >= va_end {
        return;
    }
    let mut a = va_start & !0xFFFu64;
    while a < va_end {
        unsafe {
            invlpg(a);
        }
        a = a.saturating_add(4096);
    }
}

/// Invalidate TLB entries on **all** logical processors for `[va_start, va_end)`.
///
/// Bring-up: identical to [`shootdown_range`] (BSP only). When APs are online, replace with an IPI
/// that runs `invlpg` (or `mov cr3, cr3`) on each remote CPU.
pub fn shootdown_range_all_cpus(va_start: u64, va_end: u64) {
    shootdown_range(va_start, va_end);
}
