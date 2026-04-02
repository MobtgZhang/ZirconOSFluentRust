//! Page fault path — demand-zero for committed VADs; write faults on `WriteCopy` mappings; links to #PF ISR.

use core::sync::atomic::{AtomicU64, Ordering};

use super::pfn;
use super::phys::{pfn_bringup_alloc, pfn_bringup_free};
use super::pt;
use super::section::SectionObject;
use super::vad::{PageProtect, VadKind, VadTable};
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
    let p = PF_VAD_PTR.load(Ordering::Acquire);
    if p == 0 {
        return 0;
    }
    let vad = unsafe { &*(p as *const VadTable) };
    if present {
        if try_cow_write_fault(cr2, err, vad) {
            return 1;
        }
        return 0;
    }
    if try_demand_file_mapped_page(cr2, err, vad) {
        return 1;
    }
    if try_demand_zero_page(cr2, err, vad) {
        return 1;
    }
    0
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
        crate::arch::x86_64::tlb::invlpg(va);
    }
    pfn::pfn_set_reference_count(pa, 1);
    crate::hal::x86_64::serial::write_line(b"nt10-kernel: file-backed demand #PF handled\r\n");
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
            crate::arch::x86_64::tlb::invlpg(va);
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
    unsafe {
        crate::arch::x86_64::tlb::invlpg(va);
    }
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
        crate::arch::x86_64::tlb::invlpg(va);
    }
    pfn::pfn_set_reference_count(pa, 1);
    crate::hal::x86_64::serial::write_line(b"nt10-kernel: demand-zero #PF handled\r\n");
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
}
