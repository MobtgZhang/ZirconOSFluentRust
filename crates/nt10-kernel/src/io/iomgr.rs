//! I/O manager core — IRP dispatch façade.

use super::irp::Irp;
use core::ptr::NonNull;

/// Driver routine invoked for a major function (placeholder).
pub type IoDispatchFn = fn(ctx: NonNull<()>) -> i32;

/// Minimal device node until full device stack exists.
#[derive(Clone, Copy, Default)]
pub struct DeviceObjectStub {
    pub major_dispatch: Option<IoDispatchFn>,
}

/// Root of the bring-up device tree (one stub until stacks are wired).
#[derive(Clone, Copy)]
pub struct IoManager {
    pub root: DeviceObjectStub,
}

impl IoManager {
    pub const fn new() -> Self {
        Self {
            root: DeviceObjectStub::new(),
        }
    }
}

impl DeviceObjectStub {
    pub const fn new() -> Self {
        Self {
            major_dispatch: None,
        }
    }

    pub fn dispatch(&self, ctx: NonNull<()>) -> i32 {
        match self.major_dispatch {
            Some(f) => f(ctx),
            None => -1,
        }
    }
}

/// Sets final status/information then runs all stacked completion routines (LIFO).
pub fn io_complete_request(irp: &mut Irp, status: i32, information: usize) {
    irp.complete(status, information);
    irp.drain_completions();
}
