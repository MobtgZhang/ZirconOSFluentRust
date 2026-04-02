//! Physical frame allocation — buddy allocator + PFN database (see [`super::buddy`], [`super::pfn`]).
//!
//! **Note (vs. internal roadmap `ideas/claude/content1.2.md`):** older text may still say “`PfnBump`”;
//! this module uses the **buddy allocator** plus [`super::pfn`] for bring-up, not a bump-only cursor.
//! All frames are tracked in [`super::pfn`] when they fit [`super::pfn::MAX_PHYS_PAGES`].

use super::boot_mem::{usable_conventional_ranges, UsablePhysRange};
use super::buddy;
use super::pfn;
use crate::handoff::ZirconBootInfo;

pub use super::{PAGE_SHIFT, PAGE_SIZE};

/// `usable_conventional_ranges` output capacity (UEFI may fragment conventional memory beyond 8 runs).
pub const USABLE_RANGE_SLOTS: usize = 128;

static mut PFN_BRINGUP_SORT_SCRATCH: [u64; pfn::MAX_PHYS_PAGES] = [0u64; pfn::MAX_PHYS_PAGES];

/// # Safety
/// Call once from `kmain` on the UEFI handoff path, before any concurrent allocator use.
pub unsafe fn pfn_bringup_init(info: &ZirconBootInfo) {
    let mut ranges = [UsablePhysRange {
        base: 0,
        page_count: 0,
    }; USABLE_RANGE_SLOTS];
    let n = usable_conventional_ranges(info, &mut ranges);
    let tmp = unsafe {
        core::slice::from_raw_parts_mut(
            core::ptr::addr_of_mut!(PFN_BRINGUP_SORT_SCRATCH).cast::<u64>(),
            pfn::MAX_PHYS_PAGES,
        )
    };
    let mut k = 0usize;
    for ri in 0..n {
        let r = ranges[ri];
        for i in 0..r.page_count {
            if k >= tmp.len() {
                break;
            }
            tmp[k] = r.base.saturating_add(i.saturating_mul(PAGE_SIZE));
            k += 1;
        }
    }
    if k == 0 {
        return;
    }
    tmp[..k].sort_unstable();
    if unsafe { pfn::init_from_sorted_phys(tmp.as_ptr(), k) }.is_err() {
        return;
    }
    buddy::init_from_sorted_phys_pages(pfn::sorted_phys_slice());
}

/// Allocate one 4 KiB frame; returns **physical** base address.
#[must_use]
pub fn pfn_bringup_alloc() -> Option<u64> {
    buddy::alloc_order(0)
}

/// Free a single 4 KiB frame allocated from the buddy pool.
pub fn pfn_bringup_free(base: u64) {
    buddy::free_order(base, 0);
}

/// Free a block of `2^order` contiguous frames.
pub fn pfn_bringup_free_order(base: u64, order: usize) {
    buddy::free_order(base, order);
}

#[must_use]
pub fn pfn_pool_initialized() -> bool {
    buddy::is_initialized()
}
