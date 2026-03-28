//! Handle tables (per-process index → object).

use core::ptr::NonNull;

/// Opaque kernel object reference; not a Win32 `HANDLE` value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KernelHandle(pub u32);

pub const MAX_HANDLES_PER_PROCESS: usize = 1024;

/// Minimal fixed-size table for bring-up.
pub struct HandleTable {
    slots: [Option<NonNull<()>>; MAX_HANDLES_PER_PROCESS],
    next: u32,
}

impl Default for HandleTable {
    fn default() -> Self {
        Self::new()
    }
}

impl HandleTable {
    pub const fn new() -> Self {
        const NONE: Option<NonNull<()>> = None;
        Self {
            slots: [NONE; MAX_HANDLES_PER_PROCESS],
            next: 0,
        }
    }

    /// Reserve a slot; real OB integration will type the object pointer.
    pub fn alloc_raw(&mut self, obj: NonNull<()>) -> Option<KernelHandle> {
        let idx = self.next as usize;
        if idx >= MAX_HANDLES_PER_PROCESS {
            return None;
        }
        self.slots[idx] = Some(obj);
        self.next += 1;
        Some(KernelHandle(idx as u32))
    }

    #[must_use]
    pub fn get_raw(&self, h: KernelHandle) -> Option<NonNull<()>> {
        let idx = h.0 as usize;
        if idx >= MAX_HANDLES_PER_PROCESS {
            return None;
        }
        self.slots[idx]
    }

    /// Removes the slot if present; returns the previous raw pointer.
    pub fn close_raw(&mut self, h: KernelHandle) -> Option<NonNull<()>> {
        let idx = h.0 as usize;
        if idx >= MAX_HANDLES_PER_PROCESS {
            return None;
        }
        self.slots[idx].take()
    }
}
