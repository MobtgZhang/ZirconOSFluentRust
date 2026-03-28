//! Global tick counter driven by the platform timer interrupt (bring-up).

use core::sync::atomic::{AtomicU64, Ordering};

static TICKS: AtomicU64 = AtomicU64::new(0);

#[inline]
pub fn tick() {
    TICKS.fetch_add(1, Ordering::Relaxed);
}

#[must_use]
#[inline]
pub fn ticks() -> u64 {
    TICKS.load(Ordering::Relaxed)
}
