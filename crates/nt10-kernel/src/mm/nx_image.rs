//! DEP / NX — last-recorded PE hint from optional header (no page-table enforcement here).

use core::sync::atomic::{AtomicBool, Ordering};

static LAST_PE_MARKED_NX_COMPAT: AtomicBool = AtomicBool::new(false);

/// Records whether the most recently parsed PE image had NX compat set in DllCharacteristics.
pub fn record_pe_nx_hint(nx_compat_marked: bool) {
    LAST_PE_MARKED_NX_COMPAT.store(nx_compat_marked, Ordering::Release);
}

#[must_use]
pub fn last_pe_nx_compat_marked() -> bool {
    LAST_PE_MARKED_NX_COMPAT.load(Ordering::Relaxed)
}
