//! Object header, type indices, and typed delete dispatch (clean-room OB bring-up).

/// Magic for [`ObjectHeader`] — unmanaged pointers must **not** use this value so
/// [`ob_on_last_handle_released`] skips OB teardown.
pub const OBJECT_HEADER_MAGIC: u32 = 0x4A424F5A;

/// Per-type index for delete dispatch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectTypeIndex(pub u8);

impl ObjectTypeIndex {
    pub const NONE: Self = Self(0);
    pub const WINDOW_STATION: Self = Self(10);
    pub const DESKTOP: Self = Self(11);
    /// File/section bring-up object; no FS-backed body yet — delete is a no-op hook.
    pub const FILE_OBJECT: Self = Self(12);
    pub const TEST_STUB: Self = Self(250);
}

/// Callbacks modeled after documented OB roles (ZirconOSFluent-local naming).
#[derive(Clone, Copy)]
pub struct ObTypeDescriptor {
    pub delete_procedure: Option<unsafe fn(*mut ())>,
    pub close_procedure: Option<unsafe fn(*mut ())>,
    pub ok_to_close_procedure: Option<fn(*mut ()) -> bool>,
}

pub const OB_TYPE_EMPTY: ObTypeDescriptor = ObTypeDescriptor {
    delete_procedure: None,
    close_procedure: None,
    ok_to_close_procedure: None,
};

#[repr(C)]
pub struct ObjectHeader {
    pub magic: u32,
    pub type_index: ObjectTypeIndex,
    pub pointer_count: u32,
    pub handle_count: u32,
    pub flags: u32,
}

impl ObjectHeader {
    /// Bring-up: one outstanding handle, no extra pointer refs.
    pub const fn new(ty: ObjectTypeIndex) -> Self {
        Self {
            magic: OBJECT_HEADER_MAGIC,
            type_index: ty,
            pointer_count: 0,
            handle_count: 1,
            flags: 0,
        }
    }

    #[must_use]
    pub const fn is_managed_object(&self) -> bool {
        self.magic == OBJECT_HEADER_MAGIC
    }

    pub fn reference(&mut self) {
        self.pointer_count = self.pointer_count.saturating_add(1);
    }

    #[must_use]
    pub fn dereference(&mut self) -> bool {
        self.pointer_count = self.pointer_count.saturating_sub(1);
        self.pointer_count == 0 && self.handle_count == 0
    }

    #[must_use]
    pub const fn is_live(&self) -> bool {
        self.pointer_count > 0 || self.handle_count > 0
    }
}

unsafe fn file_object_bringup_delete_static(_p: *mut ()) {}

fn file_object_bringup_ok_to_close(_p: *mut ()) -> bool {
    true
}

unsafe fn file_object_bringup_close_static(_p: *mut ()) {}

#[must_use]
pub fn ob_descriptor_for_type(ty: ObjectTypeIndex) -> ObTypeDescriptor {
    #[cfg(test)]
    if ty == ObjectTypeIndex::TEST_STUB {
        return ObTypeDescriptor {
            delete_procedure: Some(test_support::test_stub_delete_static),
            close_procedure: Some(test_support::test_stub_close_static),
            ok_to_close_procedure: Some(test_support::test_stub_ok_to_close),
        };
    }
    match ty {
        ObjectTypeIndex::WINDOW_STATION => ObTypeDescriptor {
            delete_procedure: Some(crate::ob::winsta::delete_window_station_static),
            close_procedure: Some(crate::ob::winsta::close_window_station_static),
            ok_to_close_procedure: Some(crate::ob::winsta::ok_to_close_window_station_static),
        },
        ObjectTypeIndex::DESKTOP => ObTypeDescriptor {
            delete_procedure: Some(crate::ob::winsta::delete_desktop_static),
            close_procedure: Some(crate::ob::winsta::close_desktop_static),
            ok_to_close_procedure: Some(crate::ob::winsta::ok_to_close_desktop_static),
        },
        ObjectTypeIndex::FILE_OBJECT => ObTypeDescriptor {
            delete_procedure: Some(file_object_bringup_delete_static),
            close_procedure: Some(file_object_bringup_close_static),
            ok_to_close_procedure: Some(file_object_bringup_ok_to_close),
        },
        _ => OB_TYPE_EMPTY,
    }
}

/// Type indices that map to non-empty [`ObTypeDescriptor`] in bring-up (extend as new objects gain procedures).
pub const OB_BRINGUP_TYPED_INDICES: &[ObjectTypeIndex] = &[
    ObjectTypeIndex::WINDOW_STATION,
    ObjectTypeIndex::DESKTOP,
    ObjectTypeIndex::FILE_OBJECT,
];

/// Enumerate bring-up types and their descriptors (single entry point for future full type tables).
pub fn ob_for_each_bringup_type(mut f: impl FnMut(ObjectTypeIndex, ObTypeDescriptor)) {
    for &ty in OB_BRINGUP_TYPED_INDICES {
        f(ty, ob_descriptor_for_type(ty));
    }
}

/// If `ptr` points at a managed [`ObjectHeader`], run `delete_procedure`.
///
/// # Safety
/// `ptr` must be the object base (header is the first field).
#[inline]
pub unsafe fn object_delete_if_managed(ptr: core::ptr::NonNull<()>) {
    let h = ptr.cast::<ObjectHeader>().as_ref();
    if !h.is_managed_object() {
        return;
    }
    let desc = ob_descriptor_for_type(h.type_index);
    if let Some(del) = desc.delete_procedure {
        unsafe { del(ptr.as_ptr()) };
    }
}

/// Last handle table reference dropped: decrement handle count and run `delete_procedure` when zero.
///
/// # Safety
/// `ptr` must be the object base when managed.
#[inline]
pub unsafe fn ob_on_last_handle_released(ptr: core::ptr::NonNull<()>) {
    let raw = ptr.as_ptr().cast::<ObjectHeader>();
    let h = unsafe { &mut *raw };
    if !h.is_managed_object() {
        return;
    }
    h.handle_count = h.handle_count.saturating_sub(1);
    if h.handle_count == 0 {
        let desc = ob_descriptor_for_type(h.type_index);
        if let Some(close) = desc.close_procedure {
            unsafe { close(ptr.as_ptr()) };
        }
        if let Some(del) = desc.delete_procedure {
            unsafe { del(ptr.as_ptr()) };
        }
    }
}

#[cfg(test)]
extern crate alloc;

#[cfg(test)]
pub mod test_support {
    use super::*;
    use core::sync::atomic::{AtomicUsize, Ordering};

    pub static TEST_DELETE_CALLS: AtomicUsize = AtomicUsize::new(0);
    pub static TEST_CLOSE_CALLS: AtomicUsize = AtomicUsize::new(0);

    #[repr(C)]
    pub struct TestStubObject {
        pub header: ObjectHeader,
        pub payload: u64,
    }

    pub unsafe fn test_stub_close_static(_p: *mut ()) {
        TEST_CLOSE_CALLS.fetch_add(1, Ordering::Relaxed);
    }

    pub fn test_stub_ok_to_close(_p: *mut ()) -> bool {
        true
    }

    pub unsafe fn test_stub_delete_static(p: *mut ()) {
        TEST_DELETE_CALLS.fetch_add(1, Ordering::Relaxed);
        let _ = alloc::boxed::Box::from_raw(p.cast::<TestStubObject>());
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::{TestStubObject, TEST_CLOSE_CALLS, TEST_DELETE_CALLS};
    use super::*;
    use alloc::boxed::Box as AllocBox;
    use core::ptr::NonNull;
    use core::sync::atomic::Ordering;

    #[test]
    fn bringup_foreach_lists_winsta_desktop_with_delete() {
        let mut with_delete = 0u32;
        ob_for_each_bringup_type(|_ty, d| {
            if d.delete_procedure.is_some() {
                with_delete += 1;
            }
        });
        assert_eq!(with_delete, 3);
    }

    #[test]
    fn delete_runs_once_on_last_handle_close_path() {
        TEST_DELETE_CALLS.store(0, Ordering::Relaxed);
        TEST_CLOSE_CALLS.store(0, Ordering::Relaxed);
        let t = AllocBox::new(TestStubObject {
            header: ObjectHeader::new(ObjectTypeIndex::TEST_STUB),
            payload: 42,
        });
        let raw = AllocBox::into_raw(t);
        let p = NonNull::new(raw.cast::<()>()).unwrap();
        unsafe {
            ob_on_last_handle_released(p);
        }
        assert_eq!(TEST_CLOSE_CALLS.load(Ordering::Relaxed), 1);
        assert_eq!(TEST_DELETE_CALLS.load(Ordering::Relaxed), 1);
    }
}
