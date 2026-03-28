//! ETHREAD — per-thread kernel state (minimal; ties into [`crate::ke::sched::ThreadId`]).

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
