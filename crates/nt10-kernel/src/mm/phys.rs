//! Physical page (PFN) bump allocator for bring-up.
//!
//! Filled from UEFI memory-map conventional ranges (minus kernel reservation). Assumes
//! **identity mapping** of low physical RAM for early use (same as [`super::early_map`] bring-up).

use super::boot_mem::{usable_conventional_ranges, UsablePhysRange};
use crate::handoff::ZirconBootInfo;

pub const PAGE_SHIFT: u32 = 12;
pub const PAGE_SIZE: u64 = 1u64 << PAGE_SHIFT;

/// Sequential allocator over one or more conventional RAM runs.
#[derive(Clone, Debug)]
pub struct PfnBump {
    ranges: [UsablePhysRange; 8],
    range_count: usize,
    ri: usize,
    /// Next page index within `ranges[ri]`.
    idx_in_range: u64,
}

impl Default for PfnBump {
    fn default() -> Self {
        Self {
            ranges: [UsablePhysRange {
                base: 0,
                page_count: 0,
            }; 8],
            range_count: 0,
            ri: 0,
            idx_in_range: 0,
        }
    }
}

impl PfnBump {
    /// Build from a slice of usable ranges (e.g. first `n` slots from [`usable_conventional_ranges`]).
    #[must_use]
    pub fn from_ranges(ranges: &[UsablePhysRange]) -> Self {
        let mut s = Self::default();
        for (i, r) in ranges.iter().enumerate().take(8) {
            s.ranges[i] = *r;
        }
        s.range_count = ranges.len().min(8);
        s
    }

    /// # Safety
    /// `info` must be validated per [`super::boot_mem::validate_boot_info`].
    #[must_use]
    pub unsafe fn from_handoff(info: &ZirconBootInfo) -> Self {
        let mut buf = [UsablePhysRange {
            base: 0,
            page_count: 0,
        }; 8];
        let n = usable_conventional_ranges(info, &mut buf);
        Self::from_ranges(&buf[..n])
    }

    /// Allocate one 4 KiB frame; returns **physical** base address.
    pub fn alloc_frame(&mut self) -> Option<u64> {
        while self.ri < self.range_count {
            let r = &self.ranges[self.ri];
            if self.idx_in_range < r.page_count {
                let phys = r.base.saturating_add(self.idx_in_range.saturating_mul(PAGE_SIZE));
                self.idx_in_range += 1;
                return Some(phys);
            }
            self.ri += 1;
            self.idx_in_range = 0;
        }
        None
    }

    /// Frames successfully allocated so far (for diagnostics).
    #[must_use]
    pub fn allocated_frames(&self) -> u64 {
        let mut total = 0u64;
        for j in 0..self.ri.min(self.range_count) {
            total = total.saturating_add(self.ranges[j].page_count);
        }
        if self.ri < self.range_count {
            total = total.saturating_add(self.idx_in_range);
        }
        total
    }
}

/// Single-threaded bring-up pool (set from `kmain` after handoff validation).
static mut PFN_BRINGUP: Option<PfnBump> = None;

/// # Safety
/// Call once from `kmain` on the UEFI handoff path, before any concurrent allocator use.
pub unsafe fn pfn_bringup_init(info: &ZirconBootInfo) {
    PFN_BRINGUP = Some(PfnBump::from_handoff(info));
}

/// Allocate a frame from the bring-up PFN pool (`None` if uninitialized or exhausted).
pub fn pfn_bringup_alloc() -> Option<u64> {
    unsafe {
        let p = core::ptr::addr_of_mut!(PFN_BRINGUP);
        (*p).as_mut()?.alloc_frame()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bump_spans_ranges() {
        let ranges = [
            UsablePhysRange {
                base: 0x1000,
                page_count: 2,
            },
            UsablePhysRange {
                base: 0x10_000,
                page_count: 1,
            },
        ];
        let mut b = PfnBump::from_ranges(&ranges);
        assert_eq!(b.alloc_frame(), Some(0x1000));
        assert_eq!(b.alloc_frame(), Some(0x2000));
        assert_eq!(b.alloc_frame(), Some(0x10_000));
        assert_eq!(b.alloc_frame(), None);
    }
}
