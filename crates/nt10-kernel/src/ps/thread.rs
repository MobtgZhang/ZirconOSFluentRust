//! ETHREAD — per-thread kernel state (minimal; ties into [`crate::ke::sched::ThreadId`]).
//!
//! **Bring-up:** `tid` matches [`crate::ke::sched::ThreadStub::id`] when both are created together in kmain;
//! the scheduler’s RR queue is still stub-level until real thread state is switched in-kernel.
//!
//! Win32 message routing: per-tid desktop/TEB live in [`crate::subsystems::win32::msg_dispatch`]
//! after [`crate::subsystems::win32::msg_dispatch::apply_ethread_routing`] (or
//! `thread_bind_desktop` / `thread_bind_win32`). Fields here mirror that for bring-up introspection.

use crate::ke::sched::ThreadId;
use crate::ps::process::ProcessId;

/// Executive thread object (subset).
#[derive(Clone, Copy, Debug)]
pub struct EThread {
    pub tid: ThreadId,
    pub pid: ProcessId,
    /// User-mode TEB base (bring-up); `0` if unset.
    pub teb_user_va: u64,
    /// Kernel `DesktopObject*` as `usize` (low-memory bring-up); `0` if unset.
    pub desktop_kernel_ptr: usize,
}

impl EThread {
    #[must_use]
    pub const fn new_system_thread(pid: ProcessId, tid: ThreadId) -> Self {
        Self {
            tid,
            pid,
            teb_user_va: 0,
            desktop_kernel_ptr: 0,
        }
    }

    #[must_use]
    pub const fn with_win32_routing(mut self, teb_user_va: u64, desktop_kernel_ptr: usize) -> Self {
        self.teb_user_va = teb_user_va;
        self.desktop_kernel_ptr = desktop_kernel_ptr;
        self
    }
}
