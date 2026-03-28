//! Win32 execution engine — ties loader notifications to subsystem bring-up paths.
//!
//! **Environment block / handle inheritance**: real `CreateProcess` copies env + inheritable handles;
//! extend [`crate::ps::process::EProcess`] when user-mode spawn crosses address spaces
//! (`extensions/phase-08-ipc-services.md`).

use crate::ps::image_hooks;

/// After a main PE image is mapped and relocated (see [`crate::loader::pe_load::load_pe_from_vfs_bringup`]).
pub fn on_main_module_mapping_complete(entry_rva: u32, load_base: u64, import_dll_slots: usize) {
    image_hooks::notify_module_load_request(entry_rva, load_base);
    image_hooks::notify_main_module_committed();
    image_hooks::notify_import_binding_slots(import_dll_slots as u32);
    if import_dll_slots > 0 {
        image_hooks::notify_dll_load_completed();
    }
}
