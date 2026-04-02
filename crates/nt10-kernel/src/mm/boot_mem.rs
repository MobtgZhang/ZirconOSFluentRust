//! Boot-time physical memory view from UEFI handoff: validation and usable conventional ranges.
//!
//! **GOP framebuffer:** frames are usually **not** `EfiConventionalMemory` (often MMIO or reserved).
//! If a platform maps the framebuffer into conventional RAM, subtract that range before PFN bring-up
//! so [`super::phys::pfn_bringup_init`] never allocates those pages.
//!
//! Used as input for a future PFN database. Descriptor type values follow the UEFI spec
//! (see also [`crate::mm::early_map`]); layout is defined in `nt10-boot-protocol`.

use crate::handoff::{HandoffMemoryDescriptor, ZirconBootInfo};
use crate::mm::early_map;

/// Expected `HandoffMemoryDescriptor` size in bytes (matches ZBM10 copy stride into the handoff).
pub const HANDOFF_DESCRIPTOR_BYTES: usize = 40;

/// Default low load address for [`crate::kmain`] / `NT10KRNL.BIN` (see `link/x86_64-uefi-load.ld`).
pub const KERNEL_IMAGE_PHYS_BASE: u64 = 0x800_0000;

/// Pages reserved from `KERNEL_IMAGE_PHYS_BASE` upward so the flat kernel image does not appear “free”.
/// Tune when the kernel image grows beyond this span.
pub const KERNEL_IMAGE_RESERVE_PAGES: u64 = 8192;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootInfoError {
    MagicOrVersion,
    NullMemoryMap,
    BadDescriptorSize,
}

/// Stricter checks than [`ZirconBootInfo::validate`] for early bring-up.
#[must_use]
pub fn validate_boot_info(info: &ZirconBootInfo) -> Result<(), BootInfoError> {
    if !info.validate() {
        return Err(BootInfoError::MagicOrVersion);
    }
    if info.mem_map_count > 0 {
        if info.mem_map.is_null() {
            return Err(BootInfoError::NullMemoryMap);
        }
        if info.mem_map_descriptor_size != HANDOFF_DESCRIPTOR_BYTES {
            return Err(BootInfoError::BadDescriptorSize);
        }
    }
    Ok(())
}

/// One contiguous run of **conventional** RAM pages usable for a bump/PFN allocator after subtracting
/// the kernel image reservation hole.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UsablePhysRange {
    /// Physical base address (page-aligned in practice).
    pub base: u64,
    pub page_count: u64,
}

fn page_shift() -> u32 {
    12
}

fn phys_end(base: u64, pages: u64) -> u64 {
    base.saturating_add(pages.saturating_mul(1u64 << page_shift()))
}

/// Subtract `[hole_base, hole_base + hole_pages * PAGE_SIZE)` from `[seg_base, seg_end)` and append
/// remaining conventional fragments to `out`, up to `out.len()`.
fn push_less_hole(
    seg_base: u64,
    seg_pages: u64,
    hole_base: u64,
    hole_pages: u64,
    out: &mut [UsablePhysRange],
    filled: &mut usize,
) {
    if seg_pages == 0 {
        return;
    }
    let seg_end = phys_end(seg_base, seg_pages);
    let hole_end = phys_end(hole_base, hole_pages);
    if hole_end <= seg_base || hole_base >= seg_end {
        if *filled < out.len() {
            out[*filled] = UsablePhysRange {
                base: seg_base,
                page_count: seg_pages,
            };
            *filled += 1;
        }
        return;
    }
    let hole_lo = hole_base.max(seg_base);
    let hole_hi = hole_end.min(seg_end);
    if hole_lo <= seg_base && hole_hi >= seg_end {
        return;
    }
    if seg_base < hole_lo {
        let pages = (hole_lo - seg_base) >> page_shift();
        if pages > 0 && *filled < out.len() {
            out[*filled] = UsablePhysRange {
                base: seg_base,
                page_count: pages,
            };
            *filled += 1;
        }
    }
    if hole_hi < seg_end {
        let base = hole_hi;
        let pages = (seg_end - hole_hi) >> page_shift();
        if pages > 0 && *filled < out.len() {
            out[*filled] = UsablePhysRange {
                base,
                page_count: pages,
            };
            *filled += 1;
        }
    }
}

/// Collect `EfiConventionalMemory` runs from the handoff map, minus the kernel image reservation.
///
/// Returns the number of entries written to `out` (may be truncated if `out` is too small).
///
/// # Safety
/// `info` must satisfy [`validate_boot_info`] and live while descriptors are read.
#[must_use]
pub unsafe fn usable_conventional_ranges(
    info: &ZirconBootInfo,
    out: &mut [UsablePhysRange],
) -> usize {
    let mut n = 0usize;
    if info.mem_map.is_null() || info.mem_map_count == 0 {
        return 0;
    }
    let hole_base = if info.kernel_entry_phys != 0 {
        info.kernel_entry_phys
    } else {
        KERNEL_IMAGE_PHYS_BASE
    };
    for i in 0..info.mem_map_count {
        let d: &HandoffMemoryDescriptor = &*info.mem_map.add(i);
        if d.r#type != early_map::EFI_MEMORY_CONVENTIONAL {
            continue;
        }
        push_less_hole(
            d.physical_start,
            d.number_of_pages,
            hole_base,
            KERNEL_IMAGE_RESERVE_PAGES,
            out,
            &mut n,
        );
    }
    n
}

/// Total page count across ranges returned by [`usable_conventional_ranges`] (full scan; does not write `out`).
///
/// # Safety
/// Same as [`usable_conventional_ranges`].
#[must_use]
pub unsafe fn total_usable_pages(info: &ZirconBootInfo) -> u64 {
    let mut tmp = [UsablePhysRange {
        base: 0,
        page_count: 0,
    }; 2];
    let mut total = 0u64;
    if info.mem_map.is_null() || info.mem_map_count == 0 {
        return 0;
    }
    let hole_base = if info.kernel_entry_phys != 0 {
        info.kernel_entry_phys
    } else {
        KERNEL_IMAGE_PHYS_BASE
    };
    for i in 0..info.mem_map_count {
        let d: &HandoffMemoryDescriptor = &*info.mem_map.add(i);
        if d.r#type != early_map::EFI_MEMORY_CONVENTIONAL {
            continue;
        }
        let mut n = 0usize;
        push_less_hole(
            d.physical_start,
            d.number_of_pages,
            hole_base,
            KERNEL_IMAGE_RESERVE_PAGES,
            &mut tmp,
            &mut n,
        );
        for j in 0..n {
            total = total.saturating_add(tmp[j].page_count);
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handoff::ZirconBootInfo;

    #[test]
    fn validate_default_handoff_ok() {
        let info = ZirconBootInfo::new();
        assert!(validate_boot_info(&info).is_ok());
    }

    #[test]
    fn push_less_hole_splits_segment() {
        let mut out = [UsablePhysRange {
            base: 0,
            page_count: 0,
        }; 4];
        let mut n = 0usize;
        push_less_hole(
            0x1000,
            16,
            0x5000,
            4,
            &mut out,
            &mut n,
        );
        assert_eq!(n, 2);
        assert_eq!(out[0].base, 0x1000);
        assert_eq!(out[0].page_count, 4);
        assert_eq!(out[1].base, 0x9000);
        assert_eq!(out[1].page_count, 8);
    }

    #[test]
    fn push_less_hole_no_overlap_passthrough() {
        let mut out = [UsablePhysRange {
            base: 0,
            page_count: 0,
        }; 2];
        let mut n = 0usize;
        // Hole far above segment so the whole run is retained.
        push_less_hole(0x10_0000, 8, 0x1000_0000, 1024, &mut out, &mut n);
        assert_eq!(n, 1);
        assert_eq!(out[0].page_count, 8);
    }
}
