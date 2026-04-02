//! High-half kernel direct-map constants, optional framebuffer WC alias, and UEFI CR3 extension (D.3 bring-up).

use crate::arch::x86_64::paging::{self, read_cr3};
use crate::drivers::video::display_mgr::parse_framebuffer_handoff;
use crate::handoff::ZirconBootInfo;

/// Same 2 MiB slot as [`crate::arch::x86_64::paging`] bring-up user PD index (`USER_BRINGUP_VA` / 2 MiB).
const BRINGUP_USER_PD_IDX: usize = 128;
const PD_ENTRIES_512M: usize = 256;

/// NT-style kernel direct-map base (see `docs/cn/Memory-and-Objects.md`).
pub const KERNEL_DIRECT_MAP_BASE: u64 = 0xFFFF_8000_0000_0000;

/// Kernel virtual alias for GOP framebuffer (bring-up; below canonical hole).
pub const FRAMEBUFFER_VMAP_BASE: u64 = 0xFFFF_9000_1000_0000;

/// Map the UEFI GOP framebuffer at [`FRAMEBUFFER_VMAP_BASE`] with WC-style PTE flags.
///
/// # Safety
/// Requires PFN pool + identity-mapped table pages; safe only during BSP bring-up.
pub unsafe fn try_map_uefi_framebuffer_wc(info: &ZirconBootInfo) -> Result<(), ()> {
    if !super::phys::pfn_pool_initialized() {
        return Err(());
    }
    let fb = parse_framebuffer_handoff(&info.framebuffer).map_err(|_| ())?;
    let cr3 = crate::arch::x86_64::paging::read_cr3();
    super::pt::map_framebuffer_wc(cr3, FRAMEBUFFER_VMAP_BASE, fb.base_phys, fb.byte_len)
        .map_err(|_| ())
}

/// Clone the active UEFI PML4, add **PML4[256]** → 512 MiB direct map at [`KERNEL_DIRECT_MAP_BASE`], then switch `CR3`.
///
/// On failure the original `CR3` is restored and temporary table pages are freed. Low-half entries are
/// unchanged (copy of firmware PML4), so execution continues at the same virtual addresses.
///
/// Skipped when [`paging::using_builtin_page_tables`] is true (QEMU `-kernel` already uses a owned layout).
///
/// # Safety
/// BSP only; requires identity-mapped access to current PML4 and PFN pool for new tables + probe.
pub unsafe fn try_uefi_add_kernel_direct_map_mirror_and_switch() -> Result<(), ()> {
    if paging::using_builtin_page_tables() {
        return Err(());
    }
    if !super::phys::pfn_pool_initialized() {
        return Err(());
    }
    let old_cr3 = read_cr3();
    let Some(new_pml4_pa) = super::phys::pfn_bringup_alloc() else {
        return Err(());
    };
    let Some(pdpt_hi_pa) = super::phys::pfn_bringup_alloc() else {
        super::phys::pfn_bringup_free(new_pml4_pa);
        return Err(());
    };
    let Some(pd_hi_pa) = super::phys::pfn_bringup_alloc() else {
        super::phys::pfn_bringup_free(pdpt_hi_pa);
        super::phys::pfn_bringup_free(new_pml4_pa);
        return Err(());
    };

    let src = old_cr3 as *const u64;
    let dst = new_pml4_pa as *mut u64;
    unsafe {
        core::ptr::copy_nonoverlapping(src, dst, 512);
    }

    let pd = pd_hi_pa as *mut u64;
    for i in 0..PD_ENTRIES_512M {
        let phys = (i as u64) * 0x200_000;
        let mut ent = phys | 0x83;
        if i == BRINGUP_USER_PD_IDX {
            ent |= 1 << 2;
        }
        unsafe {
            pd.add(i).write(ent);
        }
    }
    let pdpt = pdpt_hi_pa as *mut u64;
    unsafe {
        for i in 0..512 {
            pdpt.add(i).write(0);
        }
        pdpt.write_volatile(pd_hi_pa | 0x03);
        dst.add(256).write_volatile(pdpt_hi_pa | 0x03);
    }

    unsafe {
        paging::write_cr3(new_pml4_pa);
    }
    paging::flush_tlb_all();

    const PROBE_OFF: u64 = 0x0800_0000;
    let lo = PROBE_OFF as *const u8;
    let hi = (KERNEL_DIRECT_MAP_BASE.wrapping_add(PROBE_OFF)) as *const u8;
    let ok = unsafe { lo.read_volatile() == hi.read_volatile() };
    if !ok {
        unsafe {
            paging::write_cr3(old_cr3);
        }
        paging::flush_tlb_all();
        super::phys::pfn_bringup_free(pd_hi_pa);
        super::phys::pfn_bringup_free(pdpt_hi_pa);
        super::phys::pfn_bringup_free(new_pml4_pa);
        return Err(());
    }
    Ok(())
}
