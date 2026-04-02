//! PFN database — compact sorted list of manageable physical pages + per-frame metadata.
//!
//! Layout is project-specific (clean-room). Index ↔ physical address via binary search on
//! [`sorted_phys_slice`].
//!
//! **Invariants (bring-up):** only pages listed after `pfn_bringup_init` are handed to the buddy
//! allocator; [`boot_mem`](crate::mm::boot_mem) must exclude the kernel image reservation so those
//! frames never appear here. [`PageState`] is updated by the buddy allocator on free/alloc blocks;
//! `share_count` / `reference_count` are reserved for shared/COW paths.

use super::PAGE_SIZE;

/// Maximum 4 KiB frames tracked (≈512 MiB); init truncates if conventional RAM exceeds this.
pub const MAX_PHYS_PAGES: usize = 131_072;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageState {
    Free,
    Zeroed,
    Standby,
    Modified,
    Active,
    Bad,
}

#[derive(Clone, Copy, Debug)]
pub struct MmPfnEntry {
    pub state: PageState,
    pub share_count: u8,
    pub reference_count: u16,
    /// Owning leaf PTE virtual address when applicable (reverse map hint).
    pub pte_va: u64,
}

const DEFAULT_ENTRY: MmPfnEntry = MmPfnEntry {
    state: PageState::Free,
    share_count: 0,
    reference_count: 0,
    pte_va: 0,
};

static mut PFN_SORTED_PHYS: [u64; MAX_PHYS_PAGES] = [0u64; MAX_PHYS_PAGES];
static mut PFN_PAGE_COUNT: usize = 0;
static mut PFN_TABLE: [MmPfnEntry; MAX_PHYS_PAGES] = [DEFAULT_ENTRY; MAX_PHYS_PAGES];

/// # Safety
/// Call once from BSP after `phys` buffer is sorted ascending.
pub unsafe fn init_from_sorted_phys(phys: *const u64, n: usize) -> Result<(), ()> {
    if n > MAX_PHYS_PAGES {
        return Err(());
    }
    PFN_PAGE_COUNT = n;
    let dst = core::ptr::addr_of_mut!(PFN_SORTED_PHYS).cast::<u64>();
    core::ptr::copy_nonoverlapping(phys, dst, n);
    let tbl = core::ptr::addr_of_mut!(PFN_TABLE).cast::<MmPfnEntry>();
    for i in 0..n {
        tbl.add(i).write(DEFAULT_ENTRY);
    }
    Ok(())
}

#[must_use]
pub fn managed_page_count() -> usize {
    unsafe { PFN_PAGE_COUNT }
}

#[must_use]
pub fn sorted_phys_slice() -> &'static [u64] {
    unsafe {
        let p = core::ptr::addr_of!(PFN_SORTED_PHYS).cast::<u64>();
        core::slice::from_raw_parts(p, PFN_PAGE_COUNT)
    }
}

#[must_use]
pub fn pfn_entry_slice_mut() -> &'static mut [MmPfnEntry] {
    unsafe {
        let p = core::ptr::addr_of_mut!(PFN_TABLE).cast::<MmPfnEntry>();
        core::slice::from_raw_parts_mut(p, PFN_PAGE_COUNT)
    }
}

#[must_use]
pub fn phys_to_index(phys: u64) -> Option<usize> {
    let s = sorted_phys_slice();
    s.binary_search(&phys).ok()
}

pub fn set_state_by_phys(phys: u64, state: PageState) {
    if let Some(i) = phys_to_index(phys) {
        unsafe {
            let p = core::ptr::addr_of_mut!(PFN_TABLE).cast::<MmPfnEntry>().add(i);
            (*p).state = state;
        }
    }
}

pub fn set_state_for_block(base: u64, pages: u64, state: PageState) {
    for i in 0..pages {
        let p = base.saturating_add(i.saturating_mul(PAGE_SIZE));
        set_state_by_phys(p, state);
    }
}

/// PFN reference count for shared / COW bookkeeping (best-effort when `phys` is not managed).
#[must_use]
pub fn pfn_reference_count(phys: u64) -> u16 {
    let Some(i) = phys_to_index(phys) else {
        return 0;
    };
    unsafe {
        let p = core::ptr::addr_of!(PFN_TABLE).cast::<MmPfnEntry>().add(i);
        (*p).reference_count
    }
}

pub fn pfn_set_reference_count(phys: u64, count: u16) {
    if let Some(i) = phys_to_index(phys) {
        unsafe {
            let p = core::ptr::addr_of_mut!(PFN_TABLE).cast::<MmPfnEntry>().add(i);
            (*p).reference_count = count;
        }
    }
}

pub fn pfn_reference_inc(phys: u64) {
    if let Some(i) = phys_to_index(phys) {
        unsafe {
            let p = core::ptr::addr_of_mut!(PFN_TABLE).cast::<MmPfnEntry>().add(i);
            (*p).reference_count = (*p).reference_count.saturating_add(1);
        }
    }
}

pub fn pfn_reference_dec(phys: u64) {
    if let Some(i) = phys_to_index(phys) {
        unsafe {
            let p = core::ptr::addr_of_mut!(PFN_TABLE).cast::<MmPfnEntry>().add(i);
            (*p).reference_count = (*p).reference_count.saturating_sub(1);
        }
    }
}

#[cfg(test)]
mod phys_index_tests {
    #[test]
    fn binary_search_phys_index_logic() {
        let sorted = [0x1000u64, 0x2000u64, 0x3000u64];
        assert_eq!(sorted.binary_search(&0x2000).ok(), Some(1));
        assert!(sorted.binary_search(&0x1500).is_err());
    }
}
