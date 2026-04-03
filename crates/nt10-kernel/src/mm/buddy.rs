//! Physical buddy allocator over conventional RAM (per contiguous run).
//!
//! Free blocks are linked through the first 8 bytes of the first page of each block.
//!
//! **Order semantics:** `order = 0` → one 4 KiB page; `order = 9` → `2^9` pages = 512 pages =
//! 2 MiB. [`MAX_ORDER`] allows larger blocks (e.g. 1 GiB at order 18) when RAM runs permit.
//! Pages must be identity-mapped where the allocator reads/writes list links.
//!
//! **Invariants:** every [`alloc_order`] result is returned exactly once to [`free_order`] with the
//! same `order`; only PFNs seeded from [`super::pfn`] after boot memory exclusion participate.
//! See [MM-Goals-and-Invariants.md](../../../../docs/en/MM-Goals-and-Invariants.md).

use super::pfn::{self, PageState};
use super::PAGE_SIZE;
use crate::sync::spinlock::SpinLock;

/// Inclusive max order: `2^MAX_ORDER` pages per block (2^18 * 4K = 1 GiB).
pub const MAX_ORDER: usize = 18;

static BUDDY_STATE: SpinLock<BuddyState> = SpinLock::new(BuddyState::new());

struct BuddyState {
    /// Physical address of first free block per order (`0` = empty).
    free_head: [u64; MAX_ORDER + 1],
    initialized: bool,
}

impl BuddyState {
    const fn new() -> Self {
        Self {
            free_head: [0u64; MAX_ORDER + 1],
            initialized: false,
        }
    }
}

unsafe fn read_link(p: u64) -> u64 {
    core::ptr::read_unaligned(p as *const u64)
}

unsafe fn write_link(p: u64, next: u64) {
    core::ptr::write_unaligned(p as *mut u64, next);
}

fn buddy_addr(block: u64, order: usize) -> u64 {
    let size = PAGE_SIZE << order;
    block ^ size
}

/// Largest `order` such that `block` is aligned to `2^order` pages and `2^order <= rem_pages`.
fn max_order_for_block(block: u64, rem_pages: u64) -> usize {
    let mut o = MAX_ORDER.min(63);
    loop {
        let pages_in_block = 1u64 << o;
        if pages_in_block == 0 {
            return 0;
        }
        let need = pages_in_block.saturating_mul(PAGE_SIZE);
        if rem_pages >= pages_in_block && block % need == 0 {
            return o;
        }
        if o == 0 {
            return 0;
        }
        o -= 1;
    }
}

unsafe fn push_free(state: &mut BuddyState, block: u64, order: usize) {
    if order > MAX_ORDER {
        return;
    }
    let head = state.free_head[order];
    write_link(block, head);
    state.free_head[order] = block;
    pfn::set_state_for_block(block, 1u64 << order, PageState::Free);
}

unsafe fn pop_free(state: &mut BuddyState, order: usize) -> Option<u64> {
    let head = state.free_head[order];
    if head == 0 {
        return None;
    }
    let next = read_link(head);
    state.free_head[order] = next;
    Some(head)
}

/// Remove `block` from the intrusive list `order` (exact match on block start).
unsafe fn remove_from_order_list(state: &mut BuddyState, order: usize, block: u64) -> bool {
    let head = &mut state.free_head[order];
    if *head == block {
        *head = read_link(block);
        return true;
    }
    let mut cur = *head;
    while cur != 0 {
        let n = read_link(cur);
        if n == block {
            write_link(cur, read_link(block));
            return true;
        }
        cur = n;
    }
    false
}

unsafe fn try_merge_recursive(state: &mut BuddyState, mut block: u64, mut order: usize) {
    loop {
        if order >= MAX_ORDER {
            push_free(state, block, order);
            return;
        }
        let bud = buddy_addr(block, order);
        if bud == block {
            push_free(state, block, order);
            return;
        }
        if pfn::phys_to_index(bud).is_none() {
            push_free(state, block, order);
            return;
        }
        if remove_from_order_list(state, order, bud) {
            block = block.min(bud);
            order += 1;
        } else {
            push_free(state, block, order);
            return;
        }
    }
}

/// Build buddy free lists from sorted contiguous runs (see `init_from_sorted_phys_pages`).
///
/// # Safety
/// BSP only; every page in runs must be registered in [`pfn`] and identity-mapped if touched.
pub unsafe fn init_from_sorted_phys_pages(sorted_pages: &[u64]) {
    let mut g = BUDDY_STATE.lock();
    let state = &mut *g;
    *state = BuddyState::new();
    if sorted_pages.is_empty() {
        state.initialized = true;
        return;
    }
    let mut i = 0usize;
    while i < sorted_pages.len() {
        let start = sorted_pages[i];
        let mut j = i + 1;
        while j < sorted_pages.len() && sorted_pages[j] == sorted_pages[j - 1] + PAGE_SIZE {
            j += 1;
        }
        let run_pages = (j - i) as u64;
        add_contiguous_run(state, start, run_pages);
        i = j;
    }
    state.initialized = true;
}

unsafe fn add_contiguous_run(state: &mut BuddyState, mut base: u64, mut pages: u64) {
    while pages > 0 {
        let o = max_order_for_block(base, pages);
        let n = 1u64 << o;
        push_free(state, base, o);
        base = base.saturating_add(n.saturating_mul(PAGE_SIZE));
        pages = pages.saturating_sub(n);
    }
}

#[must_use]
pub fn is_initialized() -> bool {
    let g = BUDDY_STATE.lock();
    g.initialized
}

/// Allocate `2^order` contiguous 4 KiB frames. Returns base physical address.
pub fn alloc_order(order: usize) -> Option<u64> {
    if order > MAX_ORDER {
        return None;
    }
    let mut g = BUDDY_STATE.lock();
    let state = &mut *g;
    if !state.initialized {
        return None;
    }
    unsafe { alloc_order_inner(state, order) }
}

unsafe fn alloc_order_inner(state: &mut BuddyState, order: usize) -> Option<u64> {
    if let Some(b) = pop_free(state, order) {
        pfn::set_state_for_block(b, 1u64 << order, PageState::Active);
        return Some(b);
    }
    if order >= MAX_ORDER {
        return None;
    }
    let bigger = alloc_order_inner(state, order + 1)?;
    let half_pages = 1u64 << order;
    let size_bytes = half_pages.saturating_mul(PAGE_SIZE);
    let right = bigger + size_bytes;
    push_free(state, right, order);
    pfn::set_state_for_block(bigger, half_pages, PageState::Active);
    Some(bigger)
}

/// Free a block of `2^order` pages at `base` (must match allocation boundaries).
pub fn free_order(base: u64, order: usize) {
    if order > MAX_ORDER {
        return;
    }
    let mut g = BUDDY_STATE.lock();
    let state = &mut *g;
    if !state.initialized {
        return;
    }
    unsafe {
        try_merge_recursive(state, base, order);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buddy_addr_xor_matches_order_size() {
        let base = 0x1000u64;
        for order in 0..=6 {
            let size = PAGE_SIZE << order;
            let bud = buddy_addr(base, order);
            assert_eq!(bud, base ^ size);
        }
    }

    #[test]
    fn max_order_aligns_to_power_of_two_pages() {
        let base = 0x200_000u64;
        let o = max_order_for_block(base, 512);
        assert!(o <= 9);
        let base2 = 0x1000u64;
        let o2 = max_order_for_block(base2, 1);
        assert_eq!(o2, 0);
    }

    #[test]
    fn max_order_respects_run_length() {
        let base = 0x1000u64;
        let o = max_order_for_block(base, 3);
        assert!(1u64 << o <= 3);
    }

    #[test]
    fn max_order_zero_remaining_pages_is_zero() {
        assert_eq!(max_order_for_block(0x1000u64, 0), 0);
    }

    #[test]
    fn buddy_addr_roundtrip_orders_zero_through_nine() {
        for order in 0..=9u32 {
            let size = PAGE_SIZE << order;
            let base = size;
            let bud = buddy_addr(base, order as usize);
            assert_eq!(bud ^ base, size);
        }
    }
}
