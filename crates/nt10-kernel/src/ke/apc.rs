//! Kernel APC queue (BSP bring-up): queued at any IRQL, delivered only at [`crate::ke::irql::PASSIVE_LEVEL`].

use core::ptr::null_mut;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::ke::irql::{self, PASSIVE_LEVEL};
use crate::ke::spinlock::SpinLock;

pub type ApcRoutine = fn(*mut ());

#[derive(Clone, Copy)]
pub struct KapcEntry {
    pub routine: ApcRoutine,
    pub context: *mut (),
}

unsafe impl Send for KapcEntry {}

const APC_CAP: usize = 8;

static BSP_KAPC_QUEUE: SpinLock<[Option<KapcEntry>; APC_CAP]> = SpinLock::new([None; APC_CAP]);
static APC_QUEUED: AtomicUsize = AtomicUsize::new(0);

/// Enqueue a kernel APC on the BSP (FIFO in first free slot).
pub fn queue_kernel_apc(routine: ApcRoutine, context: *mut ()) -> Result<(), ()> {
    let mut q = BSP_KAPC_QUEUE.lock();
    for slot in q.iter_mut() {
        if slot.is_none() {
            *slot = Some(KapcEntry {
                routine,
                context,
            });
            APC_QUEUED.fetch_add(1, Ordering::Relaxed);
            return Ok(());
        }
    }
    Err(())
}

/// Drain all pending kernel APCs when at `PASSIVE_LEVEL` (typically before idle or after syscall return).
pub fn deliver_pending_at_passive() {
    if irql::current() != PASSIVE_LEVEL {
        return;
    }
    let mut q = BSP_KAPC_QUEUE.lock();
    for slot in q.iter_mut() {
        if let Some(e) = slot.take() {
            (e.routine)(e.context);
            APC_QUEUED.fetch_sub(1, Ordering::Relaxed);
        }
    }
}

#[must_use]
pub fn pending_count() -> usize {
    APC_QUEUED.load(Ordering::Relaxed)
}

fn sample_kapc_noop(_: *mut ()) {}

/// Registers a no-op APC for bring-up verification (drain from `kmain`).
pub fn enqueue_bringup_sample() {
    let _ = queue_kernel_apc(sample_kapc_noop, null_mut());
}
