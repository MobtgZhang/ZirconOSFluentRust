//! Working set — minimal **accounting** for bring-up (no eviction).
//!
//! Full trimming / aging is out of scope until PFN pressure policies exist. See
//! [MM-Goals-and-Invariants.md](../../../../docs/en/MM-Goals-and-Invariants.md).
//!
//! `record_page_in` takes a **stub** VAD region cookie (typically `VadEntry::start_va`) so future per-region
//! stats can hang off the same call sites without renaming.

use core::sync::atomic::{AtomicU64, Ordering};

static WS_PAGES_ACCOUNTED: AtomicU64 = AtomicU64::new(0);
/// Last `vad_region_start_va` passed to [`WorkingSetBringup::record_page_in`] (bring-up placeholder only).
static WS_STUB_LAST_VAD_REGION: AtomicU64 = AtomicU64::new(0);

/// Global page-count hint (per-process structures can replace this later).
#[derive(Clone, Copy, Debug, Default)]
pub struct WorkingSetBringup;

impl WorkingSetBringup {
    /// Count one resident user page; `vad_region_start_va` is a placeholder for future per-VAD buckets.
    pub fn record_page_in(vad_region_start_va: u64) {
        WS_STUB_LAST_VAD_REGION.store(vad_region_start_va, Ordering::Relaxed);
        WS_PAGES_ACCOUNTED.fetch_add(1, Ordering::Relaxed);
    }

    #[must_use]
    pub fn pages_accounted() -> u64 {
        WS_PAGES_ACCOUNTED.load(Ordering::Relaxed)
    }

    #[must_use]
    pub fn stub_last_vad_region_start_va() -> u64 {
        WS_STUB_LAST_VAD_REGION.load(Ordering::Relaxed)
    }

    pub fn reset_for_test() {
        WS_PAGES_ACCOUNTED.store(0, Ordering::Relaxed);
        WS_STUB_LAST_VAD_REGION.store(0, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_page_in_increments() {
        WorkingSetBringup::reset_for_test();
        WorkingSetBringup::record_page_in(0x1000);
        WorkingSetBringup::record_page_in(0x2000);
        assert_eq!(WorkingSetBringup::pages_accounted(), 2);
        assert_eq!(WorkingSetBringup::stub_last_vad_region_start_va(), 0x2000);
        WorkingSetBringup::reset_for_test();
    }
}
