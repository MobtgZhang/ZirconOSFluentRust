//! Page fault path — demand-zero for committed VADs; links to #PF ISR.

use core::sync::atomic::{AtomicU64, Ordering};

use super::phys::pfn_bringup_alloc;
use super::pt;
use super::vad::{VadKind, VadTable};
use crate::arch::x86_64::paging::read_cr3;

/// #PF disposition (bring-up).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageFaultDisposition {
    Handled,
    AccessViolation,
}

static PF_VAD_PTR: AtomicU64 = AtomicU64::new(0);

/// Point #PF handling at an existing [`VadTable`] (e.g. current `EProcess::vad_root`).
pub fn set_page_fault_vad_table(ptr: *const VadTable) {
    PF_VAD_PTR.store(ptr as usize as u64, Ordering::Release);
}

#[must_use]
fn canonical_user_va(va: u64) -> bool {
    va < 0x0000_8000_0000_0000
}

/// `1` = resume faulting instruction (`iretq`); `0` = unhandled (ISR may halt).
#[must_use]
pub fn try_dispatch_page_fault(cr2: u64, err: u64) -> u64 {
    let present = (err & 1) != 0;
    if present {
        return 0;
    }
    let p = PF_VAD_PTR.load(Ordering::Acquire);
    if p == 0 {
        return 0;
    }
    let vad = unsafe { &*(p as *const VadTable) };
    if try_demand_zero_page(cr2, err, vad) {
        return 1;
    }
    0
}

fn try_demand_zero_page(cr2: u64, err: u64, vad: &VadTable) -> bool {
    let _ = err;
    let va = cr2 & !0xFFFu64;
    if !canonical_user_va(va) {
        return false;
    }
    let Some(entry) = vad.find_by_va(va) else {
        return false;
    };
    if !entry.committed || matches!(entry.kind, VadKind::Reserve) {
        return false;
    }
    let cr3 = read_cr3();
    if unsafe { pt::query_pte(cr3, va) }
        .map(|pte| pte & 1 != 0)
        .unwrap_or(false)
    {
        return false;
    }
    let Some(pa) = pfn_bringup_alloc() else {
        return false;
    };
    unsafe {
        core::ptr::write_bytes(pa as *mut u8, 0, 4096);
    }
    let mut flags = pt::page_flags_for_vad_entry(entry, true);
    flags.present = true;
    unsafe {
        if pt::map_4k(cr3, va, pa, flags).is_err() {
            super::phys::pfn_bringup_free(pa);
            return false;
        }
        crate::arch::x86_64::tlb::invlpg(va);
    }
    true
}

/// Legacy helper for tests / non-ISR callers.
#[must_use]
pub fn handle_page_fault(
    cr2: u64,
    error_code: u64,
    user_mode: bool,
    vads: Option<&VadTable>,
) -> PageFaultDisposition {
    let _ = user_mode;
    if let Some(v) = vads {
        if let Some(entry) = v.find_by_va(cr2) {
            if matches!(entry.kind, VadKind::Reserve) && !entry.committed {
                return PageFaultDisposition::AccessViolation;
            }
            return PageFaultDisposition::AccessViolation;
        }
    }
    let _ = (cr2, error_code);
    PageFaultDisposition::AccessViolation
}
