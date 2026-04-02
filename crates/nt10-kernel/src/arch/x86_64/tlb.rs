//! TLB maintenance — BSP `invlpg` and SMP IPI shootdown when multiple CPUs are marked online.
//!
//! **SMP acceptance:** application processors must load the **same IDT** as the BSP (including the gate
//! for [`TLB_FLUSH_IPI_VECTOR`]) before [`smp_set_online_cpu_count`] reports more than one logical CPU.
//! Otherwise the flush IPI vector is undefined on APs and shootdown is unreliable.

use core::arch::asm;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

#[cfg(target_arch = "x86_64")]
use core::arch::global_asm;

#[cfg(target_arch = "x86_64")]
use crate::ke::spinlock::SpinLock;

/// LAPIC fixed-delivery IPI vector for remote `invlpg` (must match [`crate::arch::x86_64::idt`] entry).
#[cfg(target_arch = "x86_64")]
pub const TLB_FLUSH_IPI_VECTOR: u8 = 0xFD;

#[cfg(target_arch = "x86_64")]
static ONLINE_LOGICAL_CPUS: AtomicU32 = AtomicU32::new(1);

#[cfg(target_arch = "x86_64")]
static PENDING_START: AtomicU64 = AtomicU64::new(0);

#[cfg(target_arch = "x86_64")]
static PENDING_END: AtomicU64 = AtomicU64::new(0);

/// APs remaining to acknowledge the current IPI (BSP does not count).
#[cfg(target_arch = "x86_64")]
static REMOTE_IPI_ACK: AtomicU32 = AtomicU32::new(0);

#[cfg(target_arch = "x86_64")]
static TLB_IPI_LOCK: SpinLock<()> = SpinLock::new(());

#[cfg(target_arch = "x86_64")]
global_asm!(
    ".globl tlb_flush_ipi_entry",
    ".align 16",
    "tlb_flush_ipi_entry:",
    "sub rsp, 8",
    "push rax",
    "push rcx",
    "push rdx",
    "push rsi",
    "push rdi",
    "push r8",
    "push r9",
    "push r10",
    "push r11",
    "call {rust}",
    "pop r11",
    "pop r10",
    "pop r9",
    "pop r8",
    "pop rdi",
    "pop rsi",
    "pop rdx",
    "pop rcx",
    "pop rax",
    "add rsp, 8",
    "iretq",
    rust = sym tlb_flush_ipi_rust,
);

#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
extern "C" fn tlb_flush_ipi_rust() {
    let lo = PENDING_START.load(Ordering::Acquire);
    let hi = PENDING_END.load(Ordering::Acquire);
    if lo < hi {
        shootdown_range(lo, hi);
    }
    REMOTE_IPI_ACK.fetch_sub(1, Ordering::Release);
    unsafe {
        let apic = crate::hal::x86_64::apic::cached_mmio_phys();
        if apic != 0 {
            crate::hal::x86_64::apic::send_eoi(apic);
        }
    }
}

#[cfg(target_arch = "x86_64")]
unsafe extern "C" {
    fn tlb_flush_ipi_entry();
}

/// Entry stub for IDT vector [`TLB_FLUSH_IPI_VECTOR`].
#[cfg(target_arch = "x86_64")]
#[must_use]
pub fn tlb_flush_ipi_entry_addr() -> usize {
    tlb_flush_ipi_entry as *const () as usize
}

/// Call from AP bring-up **only after** each CPU has the same IDT handler for [`TLB_FLUSH_IPI_VECTOR`].
#[cfg(target_arch = "x86_64")]
pub fn smp_set_online_cpu_count(n: u32) {
    ONLINE_LOGICAL_CPUS.store(n.max(1), Ordering::Release);
}

#[cfg(target_arch = "x86_64")]
#[must_use]
pub fn smp_online_cpu_count() -> u32 {
    ONLINE_LOGICAL_CPUS.load(Ordering::Relaxed)
}

#[cfg(not(target_arch = "x86_64"))]
#[must_use]
pub fn smp_online_cpu_count() -> u32 {
    1
}

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

/// Invalidate TLB entries covering `[va_start, va_end)` at 4 KiB steps on **this** CPU.
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

/// Local shootdown, then (x86_64 only) fixed IPI to other CPUs if [`smp_online_cpu_count`] &gt; 1.
pub fn shootdown_range_all_cpus(va_start: u64, va_end: u64) {
    shootdown_range(va_start, va_end);
    #[cfg(target_arch = "x86_64")]
    {
        if smp_online_cpu_count() <= 1 {
            return;
        }
        let _g = TLB_IPI_LOCK.lock();
        PENDING_START.store(va_start, Ordering::Release);
        PENDING_END.store(va_end, Ordering::Release);
        let rem = smp_online_cpu_count().saturating_sub(1);
        REMOTE_IPI_ACK.store(rem, Ordering::Release);
        core::sync::atomic::fence(Ordering::SeqCst);
        unsafe {
            crate::hal::x86_64::apic::send_ipi_all_excluding_self(TLB_FLUSH_IPI_VECTOR);
        }
        while REMOTE_IPI_ACK.load(Ordering::Acquire) != 0 {
            core::hint::spin_loop();
        }
    }
}

#[cfg(all(test, target_arch = "x86_64"))]
mod shootdown_bringup_tests {
    use super::*;

    #[test]
    #[ignore = "invlpg and LAPIC paths are ring-0 only; run under QEMU/kernel harness"]
    fn shootdown_single_cpu_returns_without_ipi_wait_storm() {
        smp_set_online_cpu_count(1);
        shootdown_range_all_cpus(0x1000, 0x2000);
    }
}
