//! Hypervisor detection — CPUID-based probes (no third-party hypervisor code;
//! [`crate::milestones::PHASE_HYPERV`]).
//!
//! Full `CPUID` inline assembly is deferred: on `x86_64-unknown-none` LLVM reserves `RBX`, so
//! real probing should live in a small `global_asm!` stub or assembly file. Call sites use stubs until then.

use super::cpuid::CpuidHypervisorBits;

/// `CPUID.1:ECX[31]` — hypervisor present bit (Intel/AMD architecture manuals).
#[must_use]
pub fn hypervisor_present() -> bool {
    hypervisor_bits().leaf1_ecx_hypervisor_present
}

#[must_use]
fn hypervisor_bits() -> CpuidHypervisorBits {
    CpuidHypervisorBits::bare_metal_stub()
}

/// Hypervisor vendor string from leaf `0x4000_0000` when present (best-effort).
#[must_use]
pub fn hypervisor_vendor12() -> Option<[u8; 12]> {
    let b = hypervisor_bits();
    if b.vendor_id.iter().any(|&x| x != 0) {
        Some(b.vendor_id)
    } else {
        None
    }
}

/// Legacy stub name used by early call sites.
#[must_use]
pub fn hypervisor_present_stub() -> bool {
    hypervisor_present()
}

/// Short string for serial logs until full enlightenment reporting exists.
#[must_use]
pub fn hypervisor_caps_summary_stub() -> &'static str {
    if hypervisor_present() {
        "hypervisor_present"
    } else {
        "bare_metal"
    }
}
