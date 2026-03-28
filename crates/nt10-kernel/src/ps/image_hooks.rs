//! Kernel-side hooks for PE/DLL load requests (bring-up observability).

use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

static LAST_ENTRY_RVA: AtomicU32 = AtomicU32::new(0);
static LAST_LOAD_BASE: AtomicU64 = AtomicU64::new(0);
static MAIN_IMAGE_COMMITTED: AtomicU32 = AtomicU32::new(0);
static DLL_LOAD_EVENTS: AtomicU32 = AtomicU32::new(0);
static LAST_IMPORT_SLOT_COUNT: AtomicU32 = AtomicU32::new(0);

/// Records the last main-module load request (e.g. from [`crate::loader::pe_load::load_pe_from_vfs_bringup`]).
pub fn notify_module_load_request(entry_rva: u32, load_base: u64) {
    LAST_ENTRY_RVA.store(entry_rva, Ordering::Release);
    LAST_LOAD_BASE.store(load_base, Ordering::Release);
}

/// Marks the main image mapping as committed for Win32 bring-up (after relocs / size checks).
pub fn notify_main_module_committed() {
    MAIN_IMAGE_COMMITTED.fetch_add(1, Ordering::Release);
}

/// Import table descriptor count from the last successful bring-up load (DLL binding slots).
pub fn notify_import_binding_slots(count: u32) {
    LAST_IMPORT_SLOT_COUNT.store(count, Ordering::Release);
}

/// One DLL load completed (placeholder: one bump per successful image load with imports).
pub fn notify_dll_load_completed() {
    DLL_LOAD_EVENTS.fetch_add(1, Ordering::Release);
}

#[must_use]
pub fn last_module_load_snapshot() -> (u32, u64) {
    (
        LAST_ENTRY_RVA.load(Ordering::Relaxed),
        LAST_LOAD_BASE.load(Ordering::Relaxed),
    )
}

#[must_use]
pub fn main_module_commit_count() -> u32 {
    MAIN_IMAGE_COMMITTED.load(Ordering::Relaxed)
}

#[must_use]
pub fn dll_load_event_count() -> u32 {
    DLL_LOAD_EVENTS.load(Ordering::Relaxed)
}

#[must_use]
pub fn last_import_slot_count() -> u32 {
    LAST_IMPORT_SLOT_COUNT.load(Ordering::Relaxed)
}
