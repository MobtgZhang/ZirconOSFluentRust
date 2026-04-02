//! Early identity map (first 512 MiB) using 2 MiB pages.

use core::arch::asm;
use core::sync::atomic::{AtomicU64, Ordering};

use super::msr;

const PD_COUNT: usize = 1;
const PD_ENTRIES: usize = 256; // 256 * 2 MiB = 512 MiB

/// PD index for [`crate::mm::user_va::USER_BRINGUP_VA`] (256 MiB): `128 * 2 MiB == 0x1000_0000`.
const BRINGUP_USER_PD_IDX: usize = 128;

/// Physical CR3 of [`PML4`] after [`init_low_identity`] succeeds; `0` if UEFI already had paging (no-op path).
static BUILTIN_PML4_PHYS: AtomicU64 = AtomicU64::new(0);

#[repr(C, align(4096))]
struct PageTable([u64; 512]);

static mut PML4: PageTable = PageTable([0; 512]);
static mut PDPT: PageTable = PageTable([0; 512]);
static mut PD: PageTable = PageTable([0; 512]);

/// # Safety
/// BSP only. If paging is already enabled (e.g. UEFI), this is a no-op to avoid clobbering `CR3`.
pub unsafe fn init_low_identity() {
    let cr0_check: u64;
    asm!("mov {}, cr0", out(reg) cr0_check, options(nomem, nostack));
    if (cr0_check & (1 << 31)) != 0 {
        return;
    }

    let pml4 = core::ptr::addr_of_mut!(PML4) as u64;
    let pdpt = core::ptr::addr_of_mut!(PDPT) as u64;
    let pd = core::ptr::addr_of_mut!(PD) as u64;

    let pd_tbl = &mut *core::ptr::addr_of_mut!(PD);
    for e in pd_tbl.0.iter_mut() {
        *e = 0;
    }
    for i in 0..PD_ENTRIES {
        let phys = (i as u64) * 0x200_000;
        pd_tbl.0[i] = phys | 0x83; // present + writable + huge (2 MiB)
    }
    // User-readable/writable 2 MiB page for ring-3 bring-up (see `mm::user_va::USER_BRINGUP_VA`).
    pd_tbl.0[BRINGUP_USER_PD_IDX] |= 1 << 2; // U/S

    let pdpt_tbl = &mut *core::ptr::addr_of_mut!(PDPT);
    for e in pdpt_tbl.0.iter_mut() {
        *e = 0;
    }
    pdpt_tbl.0[0] = pd | 0x03;

    let pml4_tbl = &mut *core::ptr::addr_of_mut!(PML4);
    for e in pml4_tbl.0.iter_mut() {
        *e = 0;
    }
    pml4_tbl.0[0] = pdpt | 0x03;

    BUILTIN_PML4_PHYS.store(pml4, Ordering::Release);

    let cr4: u64;
    asm!("mov {}, cr4", out(reg) cr4, options(nomem, nostack));
    asm!("mov cr4, {}", in(reg) cr4 | (1 << 5), options(nomem, nostack)); // PAE

    asm!("mov cr3, {}", in(reg) pml4, options(nomem, nostack));

    let mut cr0: u64;
    asm!("mov {}, cr0", out(reg) cr0, options(nomem, nostack));
    if (cr0 & (1 << 31)) == 0 {
        cr0 |= 1 << 31;
        asm!("mov cr0, {}", in(reg) cr0, options(nomem, nostack));
    }

    let _ = PD_COUNT; // reserved for future multi-PD
}

/// `true` when the CPU uses the page tables installed by [`init_low_identity`] (QEMU `-kernel` path).
#[must_use]
pub fn using_builtin_page_tables() -> bool {
    let builtin = BUILTIN_PML4_PHYS.load(Ordering::Relaxed);
    if builtin == 0 {
        return false;
    }
    let cr3: u64;
    unsafe {
        asm!("mov {}, cr3", out(reg) cr3, options(nomem, nostack));
    }
    cr3 == builtin
}

/// Reload `CR3` with its current value to invalidate the TLB (BSP bring-up).
#[inline]
pub fn flush_tlb_all() {
    unsafe {
        let cr3: u64;
        core::arch::asm!("mov {}, cr3", out(reg) cr3, options(nomem, nostack));
        core::arch::asm!("mov cr3, {}", in(reg) cr3, options(nomem, nostack));
    }
}

/// PTE bit 63: no-execute when `EFER.NXE` is set (Intel SDM).
pub const PTE_NX: u64 = 1u64 << 63;

const IA32_EFER: u32 = 0xC000_0080;
const EFER_NXE: u64 = 1 << 11;

/// Current `CR3` (PML4 physical base).
#[must_use]
pub fn read_cr3() -> u64 {
    let v: u64;
    unsafe {
        asm!("mov {}, cr3", out(reg) v, options(nomem, nostack));
    }
    v
}

/// Load a new PML4 physical base (full TLB invalidation except global; bring-up only).
///
/// # Safety
/// `pa` must be 4 KiB-aligned and point to a valid PML4; current RIP/stack must remain mapped.
#[inline]
pub unsafe fn write_cr3(pa: u64) {
    unsafe {
        asm!("mov cr3, {}", in(reg) pa, options(nomem, nostack));
    }
}

/// Enable `EFER.NXE` so PTE bit 63 is treated as no-execute.
///
/// # Safety
/// Must run on x86_64 before relying on NX PTEs; harmless if already set.
pub unsafe fn enable_nxe() {
    let efer = unsafe { msr::rdmsr(IA32_EFER) };
    unsafe {
        msr::wrmsr(IA32_EFER, efer | EFER_NXE);
    }
}

/// Apply or clear NX on a leaf PTE value (bring-up helper; does not flush TLB).
#[must_use]
pub const fn pte_with_nx(pte: u64, nx: bool) -> u64 {
    if nx {
        pte | PTE_NX
    } else {
        pte & !PTE_NX
    }
}
