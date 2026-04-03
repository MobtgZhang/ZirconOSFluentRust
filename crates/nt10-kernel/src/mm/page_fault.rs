//! Page fault path — demand-zero for committed VADs; write faults on `WriteCopy` mappings; links to #PF ISR.
//!
//! x86_64 `#PF` error code (documented bits): P=present, W=write, U=user. We treat **user canonical**
//! VA as `< 0x0000_8000_0000_0000`; faults outside that range from user mode are not installed.
//!
//! # Bring-up invariants: global VAD binding vs `EProcess`
//!
//! - [`try_dispatch_page_fault`] reads [`PF_VAD_PTR`] (set via [`set_page_fault_vad_table`]). The ISR uses this
//!   pointer; it is **not** inferred from `CR3` alone today.
//! - [`crate::ps::process::EProcess::vad_root`] is the VAD tree for the active bring-up process. Keep the global
//!   pointer aimed at **that** same table by calling [`bind_page_fault_to_process_vad`] when installing user
//!   mappings or switching published `CR3` — both [`crate::kmain`] paths (built-in page tables vs UEFI user CR3)
//!   must stay in sync.
//! - **Single active binding:** one global `VadTable*` is assumed. Per-process #PF will add explicit fault
//!   context switches at `CR3` / process create–teardown instead of relying on a hidden singleton.

use core::sync::atomic::{AtomicU64, Ordering};

use super::pfn;
use super::phys::{pfn_bringup_alloc, pfn_bringup_free};
use super::pt;
use super::section::SectionObject;
use super::vad::{PageProtect, VadKind, VadTable};
use crate::arch::x86_64::paging::read_cr3;
use crate::ps::process::EProcess;

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

/// Bring-up helper: bind ISR #PF dispatch to `proc.vad_root`. Prefer this over raw [`set_page_fault_vad_table`]
/// so switch points stay grep-friendly and documented.
pub fn bind_page_fault_to_process_vad(proc: &EProcess) {
    set_page_fault_vad_table(core::ptr::addr_of!(proc.vad_root));
}

/// Current global VAD binding for [`try_dispatch_page_fault`], if any.
#[must_use]
pub fn page_fault_vad_table_ptr() -> Option<*const VadTable> {
    let p = PF_VAD_PTR.load(Ordering::Acquire);
    if p == 0 {
        None
    } else {
        Some(p as *const VadTable)
    }
}

#[must_use]
fn canonical_user_va(va: u64) -> bool {
    super::user_va::user_pointer_canonical(va)
}

/// `1` = resume faulting instruction (`iretq`); `0` = unhandled (ISR may halt).
#[must_use]
pub fn try_dispatch_page_fault(cr2: u64, err: u64) -> u64 {
    let p = PF_VAD_PTR.load(Ordering::Acquire);
    if p == 0 {
        log_unhandled_page_fault_serial(cr2, err, PfFailReason::NoVadBinding);
        return 0;
    }
    let vad = unsafe { &*(p as *const VadTable) };
    try_dispatch_page_fault_for_vad(vad, cr2, err)
}

/// Same as [`try_dispatch_page_fault`] but uses an explicit [`VadTable`] (tests / helpers; no global).
#[must_use]
pub fn try_dispatch_page_fault_for_vad(vad: &VadTable, cr2: u64, err: u64) -> u64 {
    let present = (err & 1) != 0;
    let user = (err & 4) != 0;
    let va = cr2 & !0xFFFu64;
    if user && !canonical_user_va(va) {
        log_unhandled_page_fault_serial(cr2, err, PfFailReason::NonCanonicalUserVa);
        return 0;
    }
    if present {
        if try_cow_write_fault(cr2, err, vad) {
            return 1;
        }
        if try_present_protection_fault(cr2, err, vad) {
            return 1;
        }
        log_unhandled_page_fault_serial(cr2, err, PfFailReason::PresentNoHandler);
        return 0;
    }
    if try_demand_file_mapped_page(cr2, err, vad) {
        return 1;
    }
    if try_demand_zero_page(cr2, err, vad) {
        return 1;
    }
    if let Some(reason) = reserve_or_missing_vad_reason(vad, va) {
        log_unhandled_page_fault_serial(cr2, err, reason);
    } else {
        log_unhandled_page_fault_serial(cr2, err, PfFailReason::NotDemandPaged);
    }
    0
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PfFailReason {
    NoVadBinding,
    NonCanonicalUserVa,
    PresentNoHandler,
    ReserveNotCommitted,
    NoVadEntry,
    NotDemandPaged,
}

fn log_unhandled_page_fault_serial(cr2: u64, err: u64, reason: PfFailReason) {
    let _ = (cr2, err, reason);
    #[cfg(target_arch = "x86_64")]
    {
        use crate::rtl::log::{log_line_serial, SUB_MM};
        let tag: &[u8] = match reason {
            PfFailReason::NoVadBinding => b"pf_unhandled no_vad_binding",
            PfFailReason::NonCanonicalUserVa => b"pf_unhandled non_canonical_user",
            PfFailReason::PresentNoHandler => b"pf_unhandled present_no_handler",
            PfFailReason::ReserveNotCommitted => b"pf_unhandled reserve",
            PfFailReason::NoVadEntry => b"pf_unhandled no_vad",
            PfFailReason::NotDemandPaged => b"pf_unhandled not_demand_paged",
        };
        log_line_serial(SUB_MM, tag);
    }
}

fn reserve_or_missing_vad_reason(vad: &VadTable, va: u64) -> Option<PfFailReason> {
    let Some(entry) = vad.find_by_va(va) else {
        return Some(PfFailReason::NoVadEntry);
    };
    if matches!(entry.kind, VadKind::Reserve) && !entry.committed {
        return Some(PfFailReason::ReserveNotCommitted);
    }
    None
}

/// Present-bit faults we classify but do not fix yet (e.g. user write on read-only committed VAD).
fn try_present_protection_fault(cr2: u64, err: u64, vad: &VadTable) -> bool {
    let present = (err & 1) != 0;
    let write = (err & 2) != 0;
    let user = (err & 4) != 0;
    let instruction_fetch = (err & 0x10) != 0;
    if present && user && instruction_fetch {
        #[cfg(target_arch = "x86_64")]
        crate::rtl::log::log_line_serial(
            crate::rtl::log::SUB_MM,
            b"pf_present_instruction_fetch_bit",
        );
    }
    if !present || !write || !user {
        return false;
    }
    let va = cr2 & !0xFFFu64;
    let Some(entry) = vad.find_by_va(va) else {
        return false;
    };
    if !entry.committed || matches!(entry.kind, VadKind::Reserve) {
        return false;
    }
    let ro = matches!(
        entry.protect,
        PageProtect::ReadOnly | PageProtect::ExecuteRead
    );
    if ro {
        #[cfg(target_arch = "x86_64")]
        crate::rtl::log::log_line_serial(
            crate::rtl::log::SUB_MM,
            b"pf_present user_write_on_ro_vad",
        );
    }
    false
}

fn try_demand_file_mapped_page(cr2: u64, err: u64, vad: &VadTable) -> bool {
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
    if entry.kind != VadKind::Mapped {
        return false;
    }
    let Some(sec_nn) = entry.section else {
        return false;
    };
    let sec = unsafe { &*sec_nn.as_ptr().cast::<SectionObject>() };
    if !matches!(
        &sec.backing,
        crate::mm::section::SectionBacking::FileBackedStub { .. }
    ) {
        return false;
    }
    let cr3 = read_cr3();
    if unsafe { pt::query_pte(cr3, va) }
        .map(|pte| pte & 1 != 0)
        .unwrap_or(false)
    {
        return false;
    }
    let page_off = va.saturating_sub(entry.start_va);
    let file_off = match &sec.backing {
        crate::mm::section::SectionBacking::FileBackedStub { offset, .. } => offset.saturating_add(page_off),
        _ => return false,
    };
    let Some(pa) = pfn_bringup_alloc() else {
        return false;
    };
    let page = unsafe {
        core::slice::from_raw_parts_mut(pa as *mut u8, super::PAGE_SIZE as usize)
    };
    page.fill(0);
    let _ = sec.read_file_backed_page(file_off, page);
    let mut flags = pt::page_flags_for_vad_entry(entry, true);
    flags.present = true;
    unsafe {
        if pt::map_4k(cr3, va, pa, flags).is_err() {
            pfn_bringup_free(pa);
            return false;
        }
        crate::arch::x86_64::tlb::flush_after_pte_change(cr3, va);
    }
    pfn::pfn_set_reference_count(pa, 1);
    super::working_set::WorkingSetBringup::record_page_in(entry.start_va);
    crate::rtl::log::log_line_serial(crate::rtl::log::SUB_MM, b"file-backed demand #PF handled");
    true
}

#[inline]
fn cow_needs_private_copy_page(reference_count: u16) -> bool {
    reference_count > 1
}

fn try_cow_write_fault(cr2: u64, err: u64, vad: &VadTable) -> bool {
    let user = (err & 4) != 0;
    let write = (err & 2) != 0;
    if !write || !user {
        return false;
    }
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
    if entry.protect != PageProtect::WriteCopy {
        return false;
    }
    let cr3 = read_cr3();
    let Some(pte) = (unsafe { pt::query_pte(cr3, va) }) else {
        return false;
    };
    if (pte & 1) == 0 || (pte & 2) != 0 {
        return false;
    }
    let pa = pte & 0x000F_FFFF_FFFF_F000;
    let map_user = va < 0x0000_8000_0000_0000;
    let rc = pfn::pfn_reference_count(pa);
    if !cow_needs_private_copy_page(rc) {
        let flags = pt::page_flags_cow_promoted(entry, map_user);
        unsafe {
            if pt::map_4k(cr3, va, pa, flags).is_err() {
                return false;
            }
            crate::arch::x86_64::tlb::flush_after_pte_change(cr3, va);
        }
        return true;
    }
    let Some(new_pa) = pfn_bringup_alloc() else {
        return false;
    };
    unsafe {
        core::ptr::copy_nonoverlapping(pa as *const u8, new_pa as *mut u8, 4096);
    }
    let flags = pt::page_flags_cow_promoted(entry, map_user);
    let ok = unsafe { pt::map_4k(cr3, va, new_pa, flags).is_ok() };
    if !ok {
        pfn_bringup_free(new_pa);
        return false;
    }
    pfn::pfn_reference_dec(pa);
    pfn::pfn_reference_inc(new_pa);
    pfn::pfn_set_reference_count(new_pa, 1);
    crate::arch::x86_64::tlb::flush_after_pte_change(cr3, va);
    true
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
            pfn_bringup_free(pa);
            return false;
        }
        crate::arch::x86_64::tlb::flush_after_pte_change(cr3, va);
    }
    pfn::pfn_set_reference_count(pa, 1);
    super::working_set::WorkingSetBringup::record_page_in(entry.start_va);
    crate::rtl::log::log_line_serial(crate::rtl::log::SUB_MM, b"demand-zero #PF handled");
    true
}

/// Legacy helper for tests / non-ISR callers — mirrors [`try_dispatch_page_fault`] when `vads` is `Some`.
#[must_use]
pub fn handle_page_fault(
    cr2: u64,
    error_code: u64,
    user_mode: bool,
    vads: Option<&VadTable>,
) -> PageFaultDisposition {
    let mut err = error_code;
    if user_mode {
        err |= 4;
    }
    if let Some(v) = vads {
        let handled = try_dispatch_page_fault_for_vad(v, cr2, err) != 0;
        return if handled {
            PageFaultDisposition::Handled
        } else {
            PageFaultDisposition::AccessViolation
        };
    }
    let _ = (cr2, error_code);
    PageFaultDisposition::AccessViolation
}

#[cfg(test)]
mod cow_branch_tests {
    use super::cow_needs_private_copy_page;

    #[test]
    fn cow_promote_in_place_when_shared_count_at_most_one() {
        assert!(!cow_needs_private_copy_page(0));
        assert!(!cow_needs_private_copy_page(1));
    }

    #[test]
    fn cow_private_copy_when_reference_count_gt_one() {
        assert!(cow_needs_private_copy_page(2));
        assert!(cow_needs_private_copy_page(u16::MAX));
    }

    #[test]
    fn handle_page_fault_without_vad_is_access_violation() {
        assert_eq!(
            super::handle_page_fault(0x1000, 0, true, None),
            super::PageFaultDisposition::AccessViolation
        );
    }
}

#[cfg(test)]
mod vad_binding_tests {
    use super::{page_fault_vad_table_ptr, set_page_fault_vad_table};
    use crate::mm::vad::{PageProtect, VadEntry, VadKind, VadTable};

    #[test]
    fn page_fault_vad_pointer_roundtrip() {
        let t = VadTable::new();
        let p = core::ptr::addr_of!(t);
        set_page_fault_vad_table(p);
        assert_eq!(page_fault_vad_table_ptr(), Some(p));
        set_page_fault_vad_table(core::ptr::null());
        assert_eq!(page_fault_vad_table_ptr(), None);
    }

    #[test]
    fn switching_global_binding_points_at_different_vad_trees() {
        let mut t1 = VadTable::new();
        let t2 = VadTable::new();
        let va = 0x0000_0000_0001_0000u64;
        t1.insert(VadEntry::new_range(
            va,
            va + 0x1000,
            VadKind::Private,
            PageProtect::ReadWrite,
            true,
        ))
        .unwrap();
        set_page_fault_vad_table(core::ptr::addr_of!(t1));
        assert_eq!(
            page_fault_vad_table_ptr(),
            Some(core::ptr::addr_of!(t1))
        );
        assert!(t1.find_by_va(va).is_some());
        set_page_fault_vad_table(core::ptr::addr_of!(t2));
        assert_eq!(
            page_fault_vad_table_ptr(),
            Some(core::ptr::addr_of!(t2))
        );
        assert!(t2.find_by_va(va).is_none());
        set_page_fault_vad_table(core::ptr::null());
    }
}
