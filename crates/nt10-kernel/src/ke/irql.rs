//! IRQL model (NT-style levels) — software enforcement is gradual; values match common NT names.

/// Passive / normal thread execution.
pub const PASSIVE_LEVEL: u8 = 0;
/// APC delivery.
pub const APC_LEVEL: u8 = 1;
/// Scheduler and DPCs.
pub const DISPATCH_LEVEL: u8 = 2;
/// Device interrupt floor (first DIRQL); real vectors map 3..=26 in full NT.
pub const DIRQL_BASE: u8 = 3;
pub const CLOCK_LEVEL: u8 = 28;
pub const IPI_LEVEL: u8 = 29;
pub const HIGH_LEVEL: u8 = 31;

use core::sync::atomic::{AtomicU8, Ordering};

static CURRENT_IRQL: AtomicU8 = AtomicU8::new(PASSIVE_LEVEL);

#[inline]
pub fn current() -> u8 {
    CURRENT_IRQL.load(Ordering::SeqCst)
}

/// # Safety
/// Caller must restore previous level; misuse can deadlock or miss interrupts.
pub unsafe fn raise(new: u8) -> u8 {
    let prev = CURRENT_IRQL.swap(new, Ordering::SeqCst);
    prev
}

pub fn lower(previous: u8) {
    CURRENT_IRQL.store(previous, Ordering::SeqCst);
}
