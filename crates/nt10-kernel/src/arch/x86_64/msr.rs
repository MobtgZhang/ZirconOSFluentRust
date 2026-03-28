//! Model-specific registers (`rdmsr` / `wrmsr`).

use core::arch::asm;

/// Read 64-bit MSR `index` (ecx = index, edx:eax = value).
///
/// # Safety
/// The MSR must exist on this CPU; some indices fault on bare metal.
#[inline]
pub unsafe fn rdmsr(index: u32) -> u64 {
    let lo: u32;
    let hi: u32;
    unsafe {
        asm!(
            "rdmsr",
            in("ecx") index,
            out("eax") lo,
            out("edx") hi,
            options(nomem, nostack),
        );
    }
    ((hi as u64) << 32) | lo as u64
}

/// Write 64-bit MSR.
///
/// # Safety
/// Writing invalid combinations may fault or brick timing state.
#[inline]
pub unsafe fn wrmsr(index: u32, value: u64) {
    let lo = value as u32;
    let hi = (value >> 32) as u32;
    unsafe {
        asm!(
            "wrmsr",
            in("ecx") index,
            in("eax") lo,
            in("edx") hi,
            options(nomem, nostack),
        );
    }
}
