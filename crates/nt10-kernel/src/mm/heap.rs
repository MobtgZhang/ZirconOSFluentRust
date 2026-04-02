//! Kernel bump heap over **identity-mapped** physical pages from [`super::phys::pfn_bringup_alloc`].
//!
//! Grow by grabbing new frames from the PFN pool; suitable only for single-threaded bring-up.

use super::phys::{pfn_bringup_alloc, PAGE_SIZE};

/// Variable-size bump allocator inside a contiguous byte arena.
#[derive(Debug)]
pub struct BumpArena {
    base: *mut u8,
    /// Total committed bytes (multiple of page size).
    capacity: usize,
    used: usize,
}

// Send safety: caller must only use from one CPU during bring-up; memory is identity-mapped RAM.
unsafe impl Send for BumpArena {}

impl BumpArena {
    pub const fn empty() -> Self {
        Self {
            base: core::ptr::null_mut(),
            capacity: 0,
            used: 0,
        }
    }

    /// # Safety
    /// `base`..`base+capacity` must be valid for writes and identity-mapped.
    pub unsafe fn from_raw(base: *mut u8, capacity: usize) -> Self {
        Self {
            base,
            capacity,
            used: 0,
        }
    }

    #[must_use]
    pub fn is_initialized(&self) -> bool {
        !self.base.is_null() && self.capacity > 0
    }

    /// Align `used` upward to `align` (power of two).
    fn align_up(val: usize, align: usize) -> usize {
        debug_assert!(align.is_power_of_two());
        (val.wrapping_sub(1) | align.wrapping_sub(1)).wrapping_add(1)
    }

    /// Allocate `size` bytes with `align` (power of two). Returns null if OOM.
    pub fn alloc(&mut self, align: usize, size: usize) -> *mut u8 {
        if self.base.is_null() || align == 0 || !align.is_power_of_two() {
            return core::ptr::null_mut();
        }
        let start = Self::align_up(self.used, align);
        let end = match start.checked_add(size) {
            Some(e) => e,
            None => return core::ptr::null_mut(),
        };
        if end > self.capacity {
            return core::ptr::null_mut();
        }
        self.used = end;
        // SAFETY: bounded by arena
        unsafe { self.base.add(start) }
    }

    /// Grow arena by `pages` **contiguous** frames from the global PFN bring-up pool.
    ///
    /// # Safety
    /// Physical addresses must be identity-mapped like other low-memory bring-up.
    pub unsafe fn grow_from_pfn_pool(&mut self, pages: usize) -> bool {
        if pages == 0 {
            return true;
        }
        let mut expect_next: Option<u64> = None;
        let mut region_start: *mut u8 = core::ptr::null_mut();
        let mut total = 0usize;
        for _ in 0..pages {
            let Some(p) = pfn_bringup_alloc() else {
                return false;
            };
            match expect_next {
                None => {
                    region_start = p as *mut u8;
                    expect_next = Some(p.saturating_add(PAGE_SIZE));
                }
                Some(e) if e == p => {
                    expect_next = Some(p.saturating_add(PAGE_SIZE));
                }
                Some(_) => return false,
            }
            total = total.saturating_add(PAGE_SIZE as usize);
        }
        if self.base.is_null() {
            self.base = region_start;
            self.capacity = total;
            self.used = 0;
        } else {
            let end = self.base as usize + self.capacity;
            if end != region_start as usize {
                return false;
            }
            self.capacity = self.capacity.saturating_add(total);
        }
        true
    }
}

/// Global bring-up heap (single-threaded).
static mut KERNEL_BUMP: BumpArena = BumpArena::empty();

/// # Safety
/// Call after [`super::phys::pfn_bringup_init`]. Typically grows one or more pages then serves `kernel_bump_alloc`.
pub unsafe fn kernel_heap_bringup_reserve_pages(pages: usize) -> bool {
    let p = core::ptr::addr_of_mut!(KERNEL_BUMP);
    (*p).grow_from_pfn_pool(pages)
}

/// Allocate from the global bump heap (`align` power of two). Null if uninitialized or OOM.
pub fn kernel_bump_alloc(align: usize, size: usize) -> *mut u8 {
    unsafe {
        let p = core::ptr::addr_of_mut!(KERNEL_BUMP);
        (*p).alloc(align, size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arena_align_and_bump() {
        let mut space = [0u8; 256];
        let mut a = unsafe { BumpArena::from_raw(space.as_mut_ptr(), space.len()) };
        let p0 = a.alloc(16, 10);
        assert!(!p0.is_null());
        assert_eq!(p0 as usize % 16, 0);
        let p1 = a.alloc(8, 1);
        assert_eq!(p1, unsafe { p0.add(16) });
    }
}
