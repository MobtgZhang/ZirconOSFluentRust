//! EPROCESS / KPROCESS placeholders.

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
        }
    }

    pub fn alloc_handle(&mut self, obj: NonNull<()>) -> Option<KernelHandle> {
        self.handles.alloc_raw(obj)
    }

    #[must_use]
    pub fn object_from_handle(&self, h: KernelHandle) -> Option<NonNull<()>> {
        self.handles.get_raw(h)
    }

    /// Closes a handle slot (bring-up: no typed object destructor yet).
    pub fn close_handle(&mut self, h: KernelHandle) -> Option<NonNull<()>> {
        self.handles.close_raw(h)
    }
}
