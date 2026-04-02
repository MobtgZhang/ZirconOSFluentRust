//! Minimal wait primitive for message-queue bring-up (no dispatcher objects yet).
//!
//! [`MsgWaitGen`] is a monotonic counter: waiters capture a generation, posters bump it.
//! Posters must call [`MsgWaitGen::wake_one`] after enqueue (Win32 path: per-thread slot `push` in
//! `subsystems::win32::msg_dispatch`).

use core::sync::atomic::{AtomicU32, Ordering};

/// Generation-based wake (avoids lost wakeups vs a single boolean).
pub struct MsgWaitGen {
    gen: AtomicU32,
}

impl MsgWaitGen {
    pub const fn new() -> Self {
        Self {
            gen: AtomicU32::new(0),
        }
    }

    #[must_use]
    pub fn current(&self) -> u32 {
        self.gen.load(Ordering::Acquire)
    }

    /// Spin until generation differs from `seen` (typically from [`Self::current`] before checking an empty queue).
    pub fn wait_until_changed(&self, seen: u32) {
        while self.gen.load(Ordering::Acquire) == seen {
            crate::ke::sched::block_cooperative_idle();
        }
    }

    pub fn wake_one(&self) {
        self.gen.fetch_add(1, Ordering::Release);
    }
}
