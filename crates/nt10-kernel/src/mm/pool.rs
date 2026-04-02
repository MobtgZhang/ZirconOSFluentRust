//! NonPagedPool-style slab (power-of-two total chunk sizes including 8-byte header).

use core::sync::atomic::{AtomicUsize, Ordering};

use super::phys::pfn_bringup_alloc;
use crate::sync::spinlock::SpinLock;

/// Total bytes per chunk (8-byte header + payload).
const CLASSES: [usize; 9] = [16, 32, 64, 128, 256, 512, 1024, 2048, 4096];

static POOL_LOCK: SpinLock<PoolState> = SpinLock::new(PoolState::new());
static POOL_BYTES: AtomicUsize = AtomicUsize::new(0);

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

/// `ExAllocatePoolWithTag` analogue — returns zeroed **payload** (after 8-byte header) or null.
#[must_use]
pub fn ex_allocate_pool_with_tag(size: usize, tag: u32) -> *mut u8 {
    let Some(idx) = class_index_for_alloc(size) else {
        return core::ptr::null_mut();
    };
    let mut g = POOL_LOCK.lock();
    let state = &mut *g;
    unsafe {
        if state.heads[idx] == 0 && !refill_class(state, idx) {
            return core::ptr::null_mut();
        }
        let Some(p) = pop_head(state, idx) else {
            return core::ptr::null_mut();
        };
        core::ptr::write_unaligned(p as *mut u32, tag);
        core::ptr::write_unaligned(p.add(4) as *mut u32, idx as u32);
        let user = p.add(8);
        let payload = CLASSES[idx] - 8;
        core::ptr::write_bytes(user, 0, payload);
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
            return;
        }
        let mut g = POOL_LOCK.lock();
        let state = &mut *g;
        push_head(state, idx, hdr);
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
