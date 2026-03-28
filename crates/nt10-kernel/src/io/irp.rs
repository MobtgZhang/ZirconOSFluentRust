//! I/O Request Packet (stacked driver model).

use core::ptr::NonNull;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IrpMajor {
    Create = 0,
    Close = 2,
    Read = 3,
    Write = 4,
    Pnp = 0x1b,
    Power = 0x16,
}

/// Max synchronous completion routines chained on one IRP (bring-up).
pub const IRP_COMPLETION_STACK: usize = 4;

/// Completion runs after status/information are finalized for one stack frame.
pub type IoCompletionRoutine = unsafe fn(*mut Irp, *mut ());

#[repr(C)]
pub struct Irp {
    pub major: u8,
    pub minor: u8,
    pub flags: u16,
    pub status: i32,
    pub information: usize,
    pub device: Option<NonNull<()>>,
    completion_depth: u8,
    completion_stack_routine: [Option<IoCompletionRoutine>; IRP_COMPLETION_STACK],
    completion_stack_ctx: [*mut (); IRP_COMPLETION_STACK],
}

impl Irp {
    #[must_use]
    pub fn new_read(dev: Option<NonNull<()>>) -> Self {
        const NONE: Option<IoCompletionRoutine> = None;
        Self {
            major: IrpMajor::Read as u8,
            minor: 0,
            flags: 0,
            status: 0,
            information: 0,
            device: dev,
            completion_depth: 0,
            completion_stack_routine: [NONE; IRP_COMPLETION_STACK],
            completion_stack_ctx: [core::ptr::null_mut(); IRP_COMPLETION_STACK],
        }
    }

    /// Push a completion (LIFO order on [`crate::io::iomgr::io_complete_request`]: last pushed runs first).
    pub fn push_completion(&mut self, routine: IoCompletionRoutine, ctx: *mut ()) -> Result<(), ()> {
        let d = self.completion_depth as usize;
        if d >= IRP_COMPLETION_STACK {
            return Err(());
        }
        self.completion_stack_routine[d] = Some(routine);
        self.completion_stack_ctx[d] = ctx;
        self.completion_depth += 1;
        Ok(())
    }

    /// Back-compat: single completion = one push.
    pub fn set_completion(&mut self, routine: Option<IoCompletionRoutine>, ctx: *mut ()) {
        self.completion_depth = 0;
        for r in &mut self.completion_stack_routine {
            *r = None;
        }
        if let Some(f) = routine {
            let _ = self.push_completion(f, ctx);
        }
    }

    pub fn complete(&mut self, status: i32, information: usize) {
        self.status = status;
        self.information = information;
    }

    /// Drain completion routines from outermost to innermost (last `push_completion` first).
    pub fn drain_completions(&mut self) {
        while self.completion_depth > 0 {
            self.completion_depth -= 1;
            let i = self.completion_depth as usize;
            if let Some(f) = self.completion_stack_routine[i].take() {
                let ctx = self.completion_stack_ctx[i];
                unsafe { f(core::ptr::from_mut(self), ctx) };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicU32, Ordering};

    static C1: AtomicU32 = AtomicU32::new(0);
    static C2: AtomicU32 = AtomicU32::new(0);

    unsafe fn comp1(irp: *mut Irp, _: *mut ()) {
        C1.fetch_add(1, Ordering::Relaxed);
        unsafe {
            (*irp).information += 10;
        }
    }
    unsafe fn comp2(_: *mut Irp, _: *mut ()) {
        C2.fetch_add(1, Ordering::Relaxed);
    }

    #[test]
    fn completion_stack_lifo() {
        C1.store(0, Ordering::Relaxed);
        C2.store(0, Ordering::Relaxed);
        let mut irp = Irp::new_read(None);
        irp.push_completion(comp1, core::ptr::null_mut()).unwrap();
        irp.push_completion(comp2, core::ptr::null_mut()).unwrap();
        irp.complete(0, 1);
        irp.drain_completions();
        assert_eq!(C2.load(Ordering::Relaxed), 1, "outer completion first");
        assert_eq!(C1.load(Ordering::Relaxed), 1);
        assert_eq!(irp.information, 11);
    }
}
