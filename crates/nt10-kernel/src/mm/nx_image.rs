//! DEP / NX — PE hints and per-section PTE NX policy (public COFF fields only).

use core::sync::atomic::{AtomicBool, Ordering};

use crate::loader::pe::IMAGE_SCN_MEM_EXECUTE;

static LAST_PE_MARKED_NX_COMPAT: AtomicBool = AtomicBool::new(false);

/// Records whether the most recently parsed PE image had NX compat set in DllCharacteristics.
pub fn record_pe_nx_hint(nx_compat_marked: bool) {
    LAST_PE_MARKED_NX_COMPAT.store(nx_compat_marked, Ordering::Release);
}

#[must_use]
pub fn last_pe_nx_compat_marked() -> bool {
    LAST_PE_MARKED_NX_COMPAT.load(Ordering::Relaxed)
}

/// When mapping a PE section, set PTE NX when the section is **not** marked executable.
#[must_use]
pub const fn nx_pte_for_section_characteristics(section_characteristics: u32) -> bool {
    (section_characteristics & IMAGE_SCN_MEM_EXECUTE) == 0
}
