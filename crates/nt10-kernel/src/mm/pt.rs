//! 4 KiB page-table walk / map (identity access to paging structures — UEFI / bring-up assumption).

use super::phys::pfn_bringup_alloc;
use super::vad::{PageProtect, VadEntry, VadKind};
use crate::arch::x86_64::paging::pte_with_nx;

/// Standard x86_64 paging flags (low 12 bits + NX).
#[derive(Clone, Copy, Debug)]
pub struct PageFlags {
    pub present: bool,
    pub write: bool,
    pub user: bool,
    pub nx: bool,
    /// Prefer WC memory type when PAT allows (PAT bit + PCD/PWT per current MSR defaults).
    pub write_combining: bool,
}

impl PageFlags {
    #[must_use]
    pub const fn kernel_rw() -> Self {
        Self {
            present: true,
            write: true,
            user: false,
            nx: true,
            write_combining: false,
        }
    }

    #[must_use]
    pub const fn kernel_fb_wc() -> Self {
        Self {
            present: true,
            write: true,
            user: false,
            nx: true,
            write_combining: true,
        }
    }

    fn to_pte_low(self, pa: u64) -> u64 {
        let mut v = pa & 0x000F_FFFF_FFFF_F000;
        if self.present {
            v |= 1;
        }
        if self.write {
            v |= 2;
        }
        if self.user {
            v |= 4;
        }
        if self.write_combining {
            // PAT index 1 (WC) with default PAT MSR: (PAT, PCD, PWT) = (0,0,1) → WT is not WC.
            // Use PAT=1, PCD=0, PWT=0 → index 6 on typical reset PAT — may vary; tune with MSR 0x277 later.
            v |= 1 << 12;
        }
        v
    }

    #[must_use]
    pub fn to_pte(self, pa: u64) -> u64 {
        let low = self.to_pte_low(pa);
        pte_with_nx(low, self.nx)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapError {
    OutOfTablePages,
    InvalidVirtualAddress,
}

fn pml4_i(va: u64) -> usize {
    ((va >> 39) & 0x1FF) as usize
}
fn pdpt_i(va: u64) -> usize {
    ((va >> 30) & 0x1FF) as usize
}
fn pd_i(va: u64) -> usize {
    ((va >> 21) & 0x1FF) as usize
}
fn pt_i(va: u64) -> usize {
    ((va >> 12) & 0x1FF) as usize
}

unsafe fn read_pte(phys: u64) -> u64 {
    core::ptr::read_volatile(phys as *const u64)
}

unsafe fn write_pte(phys: u64, val: u64) {
    core::ptr::write_volatile(phys as *mut u64, val);
}

unsafe fn alloc_zeroed_table() -> Result<u64, MapError> {
    let p = pfn_bringup_alloc().ok_or(MapError::OutOfTablePages)?;
    core::ptr::write_bytes(p as *mut u8, 0, 4096);
    Ok(p)
}

unsafe fn ensure_next_level(parent_phys: u64, index: usize) -> Result<u64, MapError> {
    let slot = parent_phys + (index as u64 * 8);
    let cur = read_pte(slot);
    if (cur & 1) != 0 {
        return Ok(cur & 0x000F_FFFF_FFFF_F000);
    }
    let t = alloc_zeroed_table()?;
    write_pte(slot, t | 0x03);
    Ok(t)
}

/// Map one 4 KiB page. Page-table pages are taken from the PFN pool; accessed by **identity** VA.
///
/// # Safety
/// `cr3_phys` must be the active PML4 physical base; caller must invalidate TLB for `va` if required.
pub unsafe fn map_4k(cr3_phys: u64, va: u64, pa: u64, flags: PageFlags) -> Result<(), MapError> {
    if va & 0xFFF != 0 || pa & 0xFFF != 0 {
        return Err(MapError::InvalidVirtualAddress);
    }
    let pml4 = cr3_phys;
    let pdpt = ensure_next_level(pml4, pml4_i(va))?;
    let pd = ensure_next_level(pdpt, pdpt_i(va))?;
    let pt = ensure_next_level(pd, pd_i(va))?;
    let slot = pt + (pt_i(va) as u64 * 8);
    write_pte(slot, flags.to_pte(pa));
    Ok(())
}

/// Unmap one 4 KiB page; returns the previous physical address (with low flags cleared) if present.
///
/// # Safety
/// Same as [`map_4k`].
pub unsafe fn unmap_4k(cr3_phys: u64, va: u64) -> Result<u64, MapError> {
    let pml4 = cr3_phys;
    let pdpt_e = read_pte(pml4 + pml4_i(va) as u64 * 8);
    if (pdpt_e & 1) == 0 {
        return Ok(0);
    }
    let pdpt = pdpt_e & 0x000F_FFFF_FFFF_F000;
    let pd_e = read_pte(pdpt + pdpt_i(va) as u64 * 8);
    if (pd_e & 1) == 0 {
        return Ok(0);
    }
    let pd = pd_e & 0x000F_FFFF_FFFF_F000;
    let pt_e = read_pte(pd + pd_i(va) as u64 * 8);
    if (pt_e & 1) == 0 || (pt_e & (1 << 7)) != 0 {
        // huge page at PD — not handled
        return Ok(0);
    }
    let pt = pt_e & 0x000F_FFFF_FFFF_F000;
    let slot = pt + pt_i(va) as u64 * 8;
    let old = read_pte(slot);
    write_pte(slot, 0);
    Ok(old & 0x000F_FFFF_FFFF_F000)
}

/// Return leaf PTE value (raw) if the page is mapped.
///
/// # Safety
/// Same as [`map_4k`].
#[must_use]
pub unsafe fn query_pte(cr3_phys: u64, va: u64) -> Option<u64> {
    let pml4 = cr3_phys;
    let pdpt_e = read_pte(pml4 + pml4_i(va) as u64 * 8);
    if (pdpt_e & 1) == 0 {
        return None;
    }
    let pdpt = pdpt_e & 0x000F_FFFF_FFFF_F000;
    let pd_e = read_pte(pdpt + pdpt_i(va) as u64 * 8);
    if (pd_e & 1) == 0 {
        return None;
    }
    let pd = pd_e & 0x000F_FFFF_FFFF_F000;
    let pt_e = read_pte(pd + pd_i(va) as u64 * 8);
    if (pt_e & 1) == 0 {
        return None;
    }
    let pt = pt_e & 0x000F_FFFF_FFFF_F000;
    Some(read_pte(pt + pt_i(va) as u64 * 8))
}

/// Build PTE flags for a VAD entry. `map_as_user` should be true for canonical lower-half VAs.
#[must_use]
pub fn page_flags_for_vad_entry(e: &super::vad::VadEntry, map_as_user: bool) -> PageFlags {
    let nx = !matches!(
        e.protect,
        super::vad::PageProtect::ExecuteRead | super::vad::PageProtect::ExecuteReadWrite
    );
    let write = matches!(
        e.protect,
        super::vad::PageProtect::ReadWrite
            | super::vad::PageProtect::ExecuteReadWrite
            | super::vad::PageProtect::WriteCopy
    );
    let present = e.kind != super::vad::VadKind::Reserve;
    PageFlags {
        present,
        write,
        user: map_as_user,
        nx,
        write_combining: false,
    }
}

#[allow(dead_code)] // Legacy path; prefer [`page_flags_for_vad_entry`].
#[must_use]
fn protect_to_flags(p: PageProtect, kind: VadKind) -> PageFlags {
    let user = true;
    let nx = !matches!(
        p,
        PageProtect::ExecuteRead | PageProtect::ExecuteReadWrite
    );
    let write = matches!(
        p,
        PageProtect::ReadWrite | PageProtect::ExecuteReadWrite | PageProtect::WriteCopy
    );
    let present = kind != VadKind::Reserve;
    PageFlags {
        present,
        write,
        user,
        nx,
        write_combining: false,
    }
}

/// Best-effort: map each 4 KiB page in the VAD range (bring-up; ignores demand-zero / file I/O).
///
/// # Safety
/// Requires identity-mapped PFN pool and valid `cr3_phys`.
pub unsafe fn apply_vad_to_page_tables(cr3_phys: u64, e: &VadEntry) -> Result<(), MapError> {
    if !e.committed || e.start_va >= e.end_va {
        return Ok(());
    }
    let map_user = e.start_va < 0x0000_8000_0000_0000;
    let flags = page_flags_for_vad_entry(e, map_user);
    if !flags.present {
        return Ok(());
    }
    let mut va = e.start_va;
    while va < e.end_va {
        let pa = va;
        map_4k(cr3_phys, va, pa, flags)?;
        va = va.saturating_add(4096);
    }
    crate::arch::x86_64::tlb::shootdown_range(e.start_va, e.end_va);
    Ok(())
}

/// Optional 2 MiB promotion (not implemented).
#[allow(dead_code)]
pub fn try_promote_last_mapping_to_2m(_cr3_phys: u64, _va: u64) -> Result<(), MapError> {
    Err(MapError::InvalidVirtualAddress)
}

/// Map framebuffer physical range at a kernel virtual alias (WC hint).
///
/// # Safety
/// Caller supplies a kernel VA window reserved for the mapping.
pub unsafe fn map_framebuffer_wc(
    cr3_phys: u64,
    virt_base: u64,
    phys_base: u64,
    byte_len: usize,
) -> Result<(), MapError> {
    let mut off = 0u64;
    let f = PageFlags::kernel_fb_wc();
    while (off as usize) < byte_len {
        let va = virt_base + off;
        let pa = phys_base + off;
        map_4k(cr3_phys, va, pa, f)?;
        off = off.saturating_add(4096);
    }
    crate::arch::x86_64::tlb::shootdown_range(virt_base, virt_base + byte_len as u64);
    Ok(())
}
