//! Deferred Procedure Call queue (per-CPU list, DISPATCH_LEVEL drain).

use crate::ke::spinlock::SpinLock;
use core::ptr::null_mut;

/// Type-erased DPC routine until real closure storage exists.
pub type DpcRoutine = fn(*mut ());

/// Single DPC object (intrusive list placeholder).
pub struct DpcObject {
    pub routine: DpcRoutine,
    pub context: *mut (),
    pub next: *mut DpcObject,
}

impl DpcObject {
    pub const fn new(routine: DpcRoutine, context: *mut ()) -> Self {
        Self {
            routine,
            context,
            next: null_mut(),
        }
    }
}

/// Per-CPU FIFO (singly linked); BSP only for bring-up.
pub struct DpcQueue {
    head: *mut DpcObject,
    tail: *mut DpcObject,
}

// Intrusive pointers are only touched from the BSP / under the global queue lock.
unsafe impl Send for DpcQueue {}

impl DpcQueue {
    pub const fn new() -> Self {
        Self {
            head: null_mut(),
            tail: null_mut(),
        }
    }

    /// # Safety
    /// `dpc` must remain valid until the routine runs.
    pub unsafe fn enqueue(&mut self, dpc: *mut DpcObject) {
        if dpc.is_null() {
            return;
        }
        (*dpc).next = null_mut();
        if self.head.is_null() {
            self.head = dpc;
            self.tail = dpc;
            return;
        }
        (*self.tail).next = dpc;
        self.tail = dpc;
    }

    /// Run all queued DPCs (intended at DISPATCH_LEVEL).
    pub unsafe fn drain(&mut self) {
        let mut cur = self.head;
        self.head = null_mut();
        self.tail = null_mut();
        while !cur.is_null() {
            let next = (*cur).next;
            ((*cur).routine)((*cur).context);
            cur = next;
        }
    }
}

static BSP_DPC_QUEUE: SpinLock<DpcQueue> = SpinLock::new(DpcQueue::new());

/// Queue a DPC on the BSP.
pub fn bsp_enqueue_dpc(dpc: *mut DpcObject) {
    if dpc.is_null() {
        return;
    }
    unsafe {
        BSP_DPC_QUEUE.lock().enqueue(dpc);
    }
}

/// Drain the BSP queue (timer IRQ path).
pub fn bsp_drain_pending() {
    unsafe {
        BSP_DPC_QUEUE.lock().drain();
    }
}
