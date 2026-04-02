//! DEVICE_OBJECT — RAM-backed block for VFS bring-up.

/// Block volume for a mounted filesystem (bring-up: wraps [`RamdiskDevice`]).
#[derive(Clone, Copy)]
pub struct BlockVolumeBringup {
    pub disk: RamdiskDevice,
}

impl BlockVolumeBringup {
    #[must_use]
    pub const fn from_static_slice(s: &'static [u8]) -> Self {
        Self {
            disk: RamdiskDevice::from_static_slice(s),
        }
    }
}

/// Read-only device backing store in kernel memory (identity-mapped slice).
#[derive(Clone, Copy)]
pub struct RamdiskDevice {
    pub data: *const u8,
    pub len: usize,
}

impl RamdiskDevice {
    #[must_use]
    pub const fn from_static_slice(s: &'static [u8]) -> Self {
        Self {
            data: s.as_ptr(),
            len: s.len(),
        }
    }

    /// View the whole backing store (static ramdisk / identity-mapped only).
    ///
    /// # Safety
    /// `data` must reference `len` initialized bytes valid for `'a`.
    #[must_use]
    pub unsafe fn as_slice<'a>(&self) -> &'a [u8] {
        unsafe { core::slice::from_raw_parts(self.data, self.len) }
    }

    /// Copies up to `buf.len()` bytes from `offset`; returns byte count copied.
    #[must_use]
    pub fn read_at(&self, offset: u64, buf: &mut [u8]) -> usize {
        if buf.is_empty() {
            return 0;
        }
        let off = offset as usize;
        if off >= self.len {
            return 0;
        }
        let avail = self.len - off;
        let n = buf.len().min(avail);
        unsafe {
            core::ptr::copy_nonoverlapping(self.data.add(off), buf.as_mut_ptr(), n);
        }
        n
    }
}
