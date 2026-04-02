//! Clone firmware/kernel page tables for the first Ring-3 thread on the UEFI path.
//!
//! Duplicates PML4 entry #0 down to the PD slot that covers [`super::user_va::USER_BRINGUP_VA`],
//! then replaces the 2 MiB huge mapping with a private 4 KiB page table so user mappings can be
//! customized without sharing those PTEs with the original CR3.

use crate::arch::x86_64::paging::{self, read_cr3};

use super::phys::pfn_bringup_alloc;
use super::pt;
use super::user_va::{USER_BRINGUP_STACK_TOP, USER_BRINGUP_VA};
use super::PAGE_SIZE;

const PTE_ADDR_MASK: u64 = 0x000F_FFFF_FFFF_F000;
const PD_PS: u64 = 1 << 7;

fn pdpt_i(va: u64) -> usize {
    ((va >> 30) & 0x1FF) as usize
}

fn pd_i(va: u64) -> usize {
    ((va >> 21) & 0x1FF) as usize
}

unsafe fn read_pte(table_phys: u64, idx: usize) -> u64 {
    core::ptr::read_volatile((table_phys + (idx as u64 * 8)) as *const u64)
}

unsafe fn write_pte(table_phys: u64, idx: usize, val: u64) {
    core::ptr::write_volatile((table_phys + (idx as u64 * 8)) as *mut u64, val);
}

unsafe fn alloc_zero_page() -> Option<u64> {
    let p = pfn_bringup_alloc()?;
    core::ptr::write_bytes(p as *mut u8, 0, 4096);
    Some(p)
}

/// Returns a new PML4 physical address; caller must [`paging::write_cr3`] when ready.
///
/// # Safety
/// BSP only; PFN pool initialized; identity-mapped access to current page tables.
pub unsafe fn build_uefi_first_user_cr3() -> Option<u64> {
    let src = read_cr3();
    let new_pml4 = alloc_zero_page()?;
    for i in 0..512 {
        write_pte(new_pml4, i, read_pte(src, i));
    }

    let old_pdpt0 = read_pte(new_pml4, 0) & PTE_ADDR_MASK;
    let new_pdpt0 = alloc_zero_page()?;
    for i in 0..512 {
        write_pte(new_pdpt0, i, read_pte(old_pdpt0, i));
    }
    write_pte(new_pml4, 0, new_pdpt0 | 0x03);

    let di = pdpt_i(USER_BRINGUP_VA);
    let old_pd = read_pte(new_pdpt0, di) & PTE_ADDR_MASK;
    let new_pd = alloc_zero_page()?;
    for i in 0..512 {
        write_pte(new_pd, i, read_pte(old_pd, i));
    }
    write_pte(new_pdpt0, di, new_pd | 0x03);

    let pi = pd_i(USER_BRINGUP_VA);
    let pd_ent = read_pte(new_pd, pi);
    if (pd_ent & 1) == 0 {
        return None;
    }
    let new_pt = alloc_zero_page()?;
    if (pd_ent & PD_PS) != 0 {
        let base = pd_ent & PTE_ADDR_MASK;
        let nx = (pd_ent & paging::PTE_NX) != 0;
        let attr_lo = (pd_ent & 0xFFF) & !PD_PS;
        for ti in 0..512 {
            let pa_pg = base + (ti as u64) * PAGE_SIZE;
            let low = pa_pg | attr_lo | 1;
            write_pte(new_pt, ti, paging::pte_with_nx(low, nx));
        }
    } else {
        let old_pt = pd_ent & PTE_ADDR_MASK;
        for ti in 0..512 {
            write_pte(new_pt, ti, read_pte(old_pt, ti));
        }
    }
    write_pte(new_pd, pi, new_pt | 0x03);

    Some(new_pml4)
}

/// Map the first user code page from `code_src` and leave the top stack page unmapped (demand-zero).
///
/// # Safety
/// `cr3` must be the result of [`build_uefi_first_user_cr3`]; `code_src`/`code_len` valid.
pub unsafe fn map_uefi_bringup_user_code_and_stack(
    cr3: u64,
    code_src: *const u8,
    code_len: usize,
) -> Result<(), ()> {
    let stack_page_va = USER_BRINGUP_STACK_TOP.wrapping_sub(PAGE_SIZE);
    let _ = pt::unmap_4k(cr3, USER_BRINGUP_VA).map_err(|_| ())?;
    let _ = pt::unmap_4k(cr3, stack_page_va).map_err(|_| ())?;

    let code_pa = pfn_bringup_alloc().ok_or(())?;
    let n = code_len.min(4096);
    core::ptr::copy_nonoverlapping(code_src, code_pa as *mut u8, n);

    let exec_user = pt::PageFlags {
        present: true,
        write: false,
        user: true,
        nx: false,
        write_combining: false,
    };
    pt::map_4k(cr3, USER_BRINGUP_VA, code_pa, exec_user).map_err(|_| ())?;
    super::pfn::pfn_set_reference_count(code_pa, 1);
    crate::arch::x86_64::tlb::shootdown_range(USER_BRINGUP_VA, USER_BRINGUP_STACK_TOP);
    Ok(())
}
