//! EPROCESS / KPROCESS placeholders.
//!
//! **Bring-up alignment:** [`EProcess::vad_root`] must stay consistent with the global #PF VAD binding:
//! call [`crate::mm::page_fault::bind_page_fault_to_process_vad`] (or [`crate::mm::page_fault::set_page_fault_vad_table`]
//! with `addr_of!(self.vad_root)`) when this process becomes the active user address space, and clear or rebind
//! when resetting the address space (see [`EProcess::bringup_reset_address_space`]) before another process’s `CR3`.
//! Thread scheduling uses [`crate::ke::sched::ThreadStub`] / [`crate::ps::thread::EThread`] registered from kmain;
//! full kernel relocate and production process teardown are roadmap gaps.

use crate::mm::vad::VadTable;
use crate::ob::handle::{HandleTable, KernelHandle};
use crate::ps::peb::PebRef;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ProcessId(pub u64);

/// Scheduling + VM control block (first field will mirror real KPROCESS layout later).
#[repr(C)]
pub struct KProcess {
    pub affinity: u64,
}

/// Executive process object (subset).
pub struct EProcess {
    pub pid: ProcessId,
    pub kprocess: KProcess,
    pub peb: PebRef,
    pub handles: HandleTable,
    pub vad_root: VadTable,
    pub unique_id: u64,
    /// Matches token / Terminal Services session (`\Sessions\<id>\...`).
    pub session_id: u8,
    /// Page-table root (CR3) for this process; `0` until per-process address spaces exist.
    pub cr3_phys: u64,
}

static NEXT_PID: AtomicU64 = AtomicU64::new(4);

impl EProcess {
    #[must_use]
    pub fn new_bootstrap() -> Self {
        let pid = ProcessId(NEXT_PID.fetch_add(4, Ordering::Relaxed));
        Self {
            pid,
            kprocess: KProcess { affinity: 1 },
            peb: PebRef::none(),
            handles: HandleTable::new(),
            vad_root: VadTable::new(),
            unique_id: pid.0,
            session_id: 0,
            cr3_phys: 0,
        }
    }

    pub fn alloc_handle(&mut self, obj: NonNull<()>) -> Option<KernelHandle> {
        self.handles.alloc_raw(obj)
    }

    #[must_use]
    pub fn object_from_handle(&self, h: KernelHandle) -> Option<NonNull<()>> {
        self.handles.get_raw(h)
    }

    /// Closes a handle slot; managed [`crate::ob::object::ObjectHeader`] bodies run typed delete.
    pub fn close_handle(&mut self, h: KernelHandle) -> Option<NonNull<()>> {
        self.handles.close_raw(h)
    }

    /// Clears `cr3_phys` after user thread teardown; full page-table walk + PFN free is future work.
    pub fn bringup_release_user_cr3_slot(&mut self) {
        self.cr3_phys = 0;
    }

    /// Clears the VAD tree and CR3 slot. Does **not** unmap PTEs or free PFNs — use before publishing a fresh `CR3` or for documented leak-acceptable bring-up teardown.
    pub fn bringup_reset_address_space(&mut self) {
        self.vad_root.clear();
        self.bringup_release_user_cr3_slot();
    }
}
