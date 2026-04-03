//! NonPagedPool-style slab (power-of-two total chunk sizes including 8-byte header).
//!
//! [`refill_class`] already **packs multiple small chunks into one 4 KiB PFN**; for large contiguous
//! off-screen or Section buffers use [`alloc_pfn_page_slab`] / [`free_pfn_page_slab`].
//!
//! Diagnostics: Cargo features `mm-pool-stats` (alloc/free counts), `mm-pool-tag-hist` (tag%8 buckets).
//! `debug_assertions` builds track a **live allocation count**; a free that would drop it below zero
//! trips `debug_assert!` (suspected double-free or balance bug). MM docs:
//! [MM-Pool-and-PFN-Bringup.md](../../../../docs/en/MM-Pool-and-PFN-Bringup.md),
//! [MM-Goals-and-Invariants.md](../../../../docs/en/MM-Goals-and-Invariants.md).

use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use super::phys::{pfn_bringup_alloc, pfn_bringup_free};
#[cfg(not(test))]
use crate::rtl::log::{log_line_serial, log_usize_serial, SUB_MM};
use crate::sync::spinlock::SpinLock;

/// Failed `ex_allocate_pool_with_tag` attempts (telemetry).
static POOL_ALLOC_FAILS: AtomicU32 = AtomicU32::new(0);

/// Total bytes per chunk (8-byte header + payload).
const CLASSES: [usize; 9] = [16, 32, 64, 128, 256, 512, 1024, 2048, 4096];

static POOL_LOCK: SpinLock<PoolState> = SpinLock::new(PoolState::new());
static POOL_BYTES: AtomicUsize = AtomicUsize::new(0);

#[cfg(feature = "mm-pool-stats")]
static POOL_ALLOCS: AtomicUsize = AtomicUsize::new(0);
#[cfg(feature = "mm-pool-stats")]
static POOL_FREES: AtomicUsize = AtomicUsize::new(0);

#[cfg(feature = "mm-pool-tag-hist")]
static POOL_TAG_BUCKETS: [AtomicU32; 8] = [
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
    AtomicU32::new(0),
];

/// Successful alloc minus successful free (debug only); catches global imbalance / suspected double-free.
#[cfg(debug_assertions)]
static POOL_DBG_LIVE: AtomicUsize = AtomicUsize::new(0);

#[cfg(feature = "mm-pool-tag-hist")]
#[inline]
const fn tag_bucket_index(tag: u32) -> usize {
    (tag as usize) % 8
}

struct PoolState {
    /// Stored as `usize` so the pool lock is `Sync` (raw pointers are not `Send`).
    heads: [usize; CLASSES.len()],
}

impl PoolState {
    const fn new() -> Self {
        Self { heads: [0; CLASSES.len()] }
    }
}

#[must_use]
fn class_index_for_alloc(requested: usize) -> Option<usize> {
    if requested == 0 {
        return None;
    }
    let need = requested.saturating_add(8);
    CLASSES.iter().position(|&c| c >= need)
}

unsafe fn push_head(state: &mut PoolState, idx: usize, p: *mut u8) {
    let next = state.heads[idx];
    core::ptr::write_unaligned(p as *mut u64, next as u64);
    state.heads[idx] = p as usize;
}

unsafe fn pop_head(state: &mut PoolState, idx: usize) -> Option<*mut u8> {
    let p = state.heads[idx];
    if p == 0 {
        return None;
    }
    let pp = p as *mut u8;
    let next = core::ptr::read_unaligned(pp as *const u64) as usize;
    state.heads[idx] = next;
    Some(pp)
}

unsafe fn refill_class(state: &mut PoolState, idx: usize) -> bool {
    let size = CLASSES[idx];
    let Some(frame) = pfn_bringup_alloc() else {
        return false;
    };
    POOL_BYTES.fetch_add(4096, Ordering::Relaxed);
    let base = frame as *mut u8;
    core::ptr::write_bytes(base, 0, 4096);
    let count = 4096 / size;
    for i in (0..count).rev() {
        let chunk = base.add(i * size);
        push_head(state, idx, chunk);
    }
    true
}

#[cfg(test)]
fn log_pool_alloc_fail(_reason: &[u8], _size: usize, _class_idx: Option<usize>) {
    POOL_ALLOC_FAILS.fetch_add(1, Ordering::Relaxed);
}

#[cfg(not(test))]
fn log_pool_alloc_fail(reason: &[u8], size: usize, class_idx: Option<usize>) {
    POOL_ALLOC_FAILS.fetch_add(1, Ordering::Relaxed);
    log_line_serial(SUB_MM, reason);
    log_usize_serial(SUB_MM, b"pool_alloc_req_bytes=", size);
    if let Some(i) = class_idx {
        log_usize_serial(SUB_MM, b"pool_alloc_class_idx=", i);
    }
}

/// `ExAllocatePoolWithTag` analogue — returns zeroed **payload** (after 8-byte header) or null.
#[must_use]
pub fn ex_allocate_pool_with_tag(size: usize, tag: u32) -> *mut u8 {
    let Some(idx) = class_index_for_alloc(size) else {
        log_pool_alloc_fail(
            b"ex_allocate_pool_with_tag rejected (size 0 or no slab class)",
            size,
            None,
        );
        return core::ptr::null_mut();
    };
    let mut g = POOL_LOCK.lock();
    let state = &mut *g;
    unsafe {
        if state.heads[idx] == 0 && !refill_class(state, idx) {
            log_pool_alloc_fail(
                b"ex_allocate_pool_with_tag failed (PFN refill)",
                size,
                Some(idx),
            );
            return core::ptr::null_mut();
        }
        let Some(p) = pop_head(state, idx) else {
            log_pool_alloc_fail(
                b"ex_allocate_pool_with_tag failed (empty freelist after refill)",
                size,
                Some(idx),
            );
            return core::ptr::null_mut();
        };
        core::ptr::write_unaligned(p as *mut u32, tag);
        core::ptr::write_unaligned(p.add(4) as *mut u32, idx as u32);
        let user = p.add(8);
        let payload = CLASSES[idx] - 8;
        core::ptr::write_bytes(user, 0, payload);
        #[cfg(feature = "mm-pool-stats")]
        POOL_ALLOCS.fetch_add(1, Ordering::Relaxed);
        #[cfg(feature = "mm-pool-tag-hist")]
        POOL_TAG_BUCKETS[tag_bucket_index(tag)].fetch_add(1, Ordering::Relaxed);
        #[cfg(debug_assertions)]
        POOL_DBG_LIVE.fetch_add(1, Ordering::Relaxed);
        user
    }
}

/// `ExFreePoolWithTag` analogue (`ptr` is the **user** pointer from [`ex_allocate_pool_with_tag`]).
pub fn ex_free_pool_with_tag(ptr: *mut u8, tag: u32) {
    if ptr.is_null() {
        return;
    }
    unsafe {
        let hdr = ptr.sub(8);
        let stored_tag = core::ptr::read_unaligned(hdr as *const u32);
        let idx = core::ptr::read_unaligned(hdr.add(4) as *const u32) as usize;
        if stored_tag != tag || idx >= CLASSES.len() {
            #[cfg(not(test))]
            log_line_serial(SUB_MM, b"ex_free_pool_with_tag ignored (tag/class mismatch)");
            return;
        }
        #[cfg(debug_assertions)]
        {
            let prev = POOL_DBG_LIVE.load(Ordering::Acquire);
            debug_assert!(
                prev > 0,
                "pool: live count underflow (double-free or alloc/free mismatch)"
            );
            POOL_DBG_LIVE.fetch_sub(1, Ordering::Release);
        }
        #[cfg(feature = "mm-pool-tag-hist")]
        POOL_TAG_BUCKETS[tag_bucket_index(stored_tag)].fetch_sub(1, Ordering::Relaxed);
        let mut g = POOL_LOCK.lock();
        let state = &mut *g;
        push_head(state, idx, hdr);
        #[cfg(feature = "mm-pool-stats")]
        POOL_FREES.fetch_add(1, Ordering::Relaxed);
    }
}

/// PagedPool bring-up alias (no paging-out yet).
#[must_use]
pub fn ex_allocate_paged_pool_with_tag(size: usize, tag: u32) -> *mut u8 {
    ex_allocate_pool_with_tag(size, tag)
}

/// PagedPool free alias.
pub fn ex_free_paged_pool_with_tag(ptr: *mut u8, tag: u32) {
    ex_free_pool_with_tag(ptr, tag);
}

/// One zeroed 4 KiB physical page from the PFN pool (explicit free via [`free_pfn_page_slab`]).
#[must_use]
pub fn alloc_pfn_page_slab() -> Option<u64> {
    let p = pfn_bringup_alloc()?;
    unsafe {
        core::ptr::write_bytes(p as *mut u8, 0, 4096);
    }
    Some(p)
}

/// Returns a page obtained from [`alloc_pfn_page_slab`].
pub fn free_pfn_page_slab(pa: u64) {
    if pa != 0 {
        pfn_bringup_free(pa);
    }
}

/// Count of failed [`ex_allocate_pool_with_tag`] calls (for diagnostics).
#[must_use]
pub fn pool_alloc_fail_count() -> u32 {
    POOL_ALLOC_FAILS.load(Ordering::Relaxed)
}

/// `(successful_allocs, successful_frees, approx_bytes_in_slab_pages)` when `mm-pool-stats` is enabled;
/// otherwise `(0, 0, current POOL_BYTES)`.
#[must_use]
pub fn pool_stats_snapshot() -> (usize, usize, usize) {
    #[cfg(feature = "mm-pool-stats")]
    {
        (
            POOL_ALLOCS.load(Ordering::Relaxed),
            POOL_FREES.load(Ordering::Relaxed),
            POOL_BYTES.load(Ordering::Relaxed),
        )
    }
    #[cfg(not(feature = "mm-pool-stats"))]
    {
        (0, 0, POOL_BYTES.load(Ordering::Relaxed))
    }
}

/// Snapshot of [`POOL_TAG_BUCKETS`] when `mm-pool-tag-hist` is enabled; otherwise zeros.
#[must_use]
pub fn pool_tag_buckets_snapshot() -> [u32; 8] {
    #[cfg(feature = "mm-pool-tag-hist")]
    {
        core::array::from_fn(|i| POOL_TAG_BUCKETS[i].load(Ordering::Relaxed))
    }
    #[cfg(not(feature = "mm-pool-tag-hist"))]
    {
        [0; 8]
    }
}

/// Debug-only live count (outstanding successful allocs not yet freed). Zero in non-`debug_assertions` builds.
#[must_use]
pub fn pool_debug_live_count() -> usize {
    #[cfg(debug_assertions)]
    {
        POOL_DBG_LIVE.load(Ordering::Relaxed)
    }
    #[cfg(not(debug_assertions))]
    {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pfn_page_slab_round_trip() {
        let Some(p) = alloc_pfn_page_slab() else {
            return;
        };
        unsafe {
            *(p as *mut u8) = 0xAB;
        }
        free_pfn_page_slab(p);
    }

    #[test]
    fn pool_small_round_trip_reuses_freelist() {
        let p = ex_allocate_pool_with_tag(16, 0x4141_4141);
        if p.is_null() {
            // Host tests have no PFN bring-up; same pattern as `pfn_page_slab_round_trip`.
            return;
        }
        ex_free_pool_with_tag(p, 0x4141_4141);
        let p2 = ex_allocate_pool_with_tag(16, 0x4141_4141);
        assert!(!p2.is_null());
        ex_free_pool_with_tag(p2, 0x4141_4141);
    }

    #[test]
    fn pool_stats_snapshot_smoke() {
        let (a, f, b) = pool_stats_snapshot();
        let _ = (a, f, b);
    }

    #[cfg(feature = "mm-pool-tag-hist")]
    #[test]
    fn tag_buckets_track_tag_mod_8() {
        let t0 = 0x0102_0304u32;
        let t1 = t0.wrapping_add(8);
        let p0 = ex_allocate_pool_with_tag(16, t0);
        let p1 = ex_allocate_pool_with_tag(16, t1);
        if p0.is_null() || p1.is_null() {
            if !p0.is_null() {
                ex_free_pool_with_tag(p0, t0);
            }
            if !p1.is_null() {
                ex_free_pool_with_tag(p1, t1);
            }
            return;
        }
        let b = pool_tag_buckets_snapshot();
        let i = tag_bucket_index(t0);
        assert!(b[i] >= 2, "same bucket for tags differing by 8");
        ex_free_pool_with_tag(p0, t0);
        ex_free_pool_with_tag(p1, t1);
    }
}
