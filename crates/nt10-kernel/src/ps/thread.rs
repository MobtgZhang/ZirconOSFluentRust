//! ETHREAD — per-thread kernel state (minimal; ties into [`crate::ke::sched::ThreadId`]).
//!
//! Phase 3 bring-up: the **current desktop** for Win32 routing is stored in
//! [`crate::subsystems::win32::msg_dispatch`] by `ThreadId.0` (`thread_bind_desktop` /
//! `set_current_thread_for_win32`), not in this struct yet.

use crate::ke::sched::ThreadId;
use crate::ps::process::ProcessId;

/// Executive thread object (subset).
#[derive(Clone, Copy, Debug)]
pub struct EThread {
    pub tid: ThreadId,
    pub pid: ProcessId,
}

impl EThread {
    #[must_use]
    pub const fn new_system_thread(pid: ProcessId, tid: ThreadId) -> Self {
        Self { tid, pid }
    }
}
