//! Handle tables (per-process index → object).

use core::ptr::NonNull;

/// Opaque kernel object reference; not a Win32 `HANDLE` value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KernelHandle(pub u32);

pub const MAX_HANDLES_PER_PROCESS: usize = 1024;

/// Minimal fixed-size table for bring-up (per-slot reference counts for future OB teardown).
pub struct HandleTable {
    slots: [Option<NonNull<()>>; MAX_HANDLES_PER_PROCESS],
    ref_count: [u32; MAX_HANDLES_PER_PROCESS],
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
            ref_count: [0; MAX_HANDLES_PER_PROCESS],
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
        self.ref_count[idx] = 1;
        self.next += 1;
        Some(KernelHandle(idx as u32))
    }

    /// Increment reference on an existing handle (dup semantics sketch).
    pub fn reference_raw(&mut self, h: KernelHandle) -> Result<(), ()> {
        let idx = h.0 as usize;
        if idx >= MAX_HANDLES_PER_PROCESS || self.slots[idx].is_none() {
            return Err(());
        }
        self.ref_count[idx] = self.ref_count[idx].saturating_add(1);
        Ok(())
    }

    #[must_use]
    pub fn get_raw(&self, h: KernelHandle) -> Option<NonNull<()>> {
        let idx = h.0 as usize;
        if idx >= MAX_HANDLES_PER_PROCESS {
            return None;
        }
        self.slots[idx]
    }

    /// Decrements reference count; clears slot when it reaches zero. Returns pointer on final release.
    pub fn close_raw(&mut self, h: KernelHandle) -> Option<NonNull<()>> {
        let idx = h.0 as usize;
        if idx >= MAX_HANDLES_PER_PROCESS {
            return None;
        }
        if self.slots[idx].is_none() {
            return None;
        }
        let r = self.ref_count[idx].saturating_sub(1);
        self.ref_count[idx] = r;
        if r == 0 {
            let obj = self.slots[idx].take();
            if let Some(p) = obj {
                let hdr = p.cast::<crate::ob::object::ObjectHeader>();
                if unsafe { hdr.as_ref().is_managed_object() } {
                    unsafe {
                        crate::ob::object::ob_on_last_handle_released(p);
                    }
                    return None;
                }
                return Some(p);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate alloc;

    #[test]
    fn refcount_dup_delays_release() {
        let mut t = HandleTable::new();
        let b = alloc::boxed::Box::new(0u64);
        let p = NonNull::new(alloc::boxed::Box::into_raw(b).cast::<()>()).unwrap();
        let h = t.alloc_raw(p).unwrap();
        assert!(t.reference_raw(h).is_ok());
        assert!(t.close_raw(h).is_none());
        let released = t.close_raw(h);
        assert_eq!(released, Some(p));
        let _ = unsafe { alloc::boxed::Box::from_raw(p.as_ptr().cast::<u64>()) };
    }
}
