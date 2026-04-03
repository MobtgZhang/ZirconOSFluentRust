//! Page file / swap backend — **unsupported** in bring-up (no disk eviction).
//!
//! Implementations must use explicit backends (e.g. VirtIO block) and public on-disk layout only;
//! do not mirror Windows paging-file internals.
//!
//! # IRP-shaped stub
//!
//! Real paging I/O would layer on [`crate::io::irp::Irp`] read/write against a block volume. The helpers
//! below return typed errors until a backend exists.

use crate::io::irp::Irp;

/// Backing for paged-out anonymous memory (future).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageFileBackend {
    /// No swap device; `commit` failures surface as `pfn_pool_starved` / allocator errors.
    Unsupported,
}

/// Errors for bring-up paging-file I/O (no NT-private status codes as authority).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageFileIoError {
    /// No VirtIO/block paging device wired.
    Unsupported,
    /// Guest file offset or length out of range for the stub volume.
    InvalidRange,
    /// MDL / user buffer smaller than requested transfer.
    BufferTooSmall,
    /// IRP stack or completion state inconsistent for a synchronous stub.
    IrpState,
}

#[must_use]
pub const fn page_file_backend_bringup() -> PageFileBackend {
    PageFileBackend::Unsupported
}

/// Stub: issue a paging **read** via an [`Irp`] — always [`PageFileIoError::Unsupported`] today.
pub fn stub_pagefile_issue_read_irp(
    irp: &mut Irp,
    _guest_byte_offset: u64,
    _len: usize,
) -> Result<(), PageFileIoError> {
    let _ = irp;
    Err(PageFileIoError::Unsupported)
}

/// Stub: issue a paging **write** via an [`Irp`] — always [`PageFileIoError::Unsupported`] today.
pub fn stub_pagefile_issue_write_irp(
    irp: &mut Irp,
    _guest_byte_offset: u64,
    _len: usize,
) -> Result<(), PageFileIoError> {
    let _ = irp;
    Err(PageFileIoError::Unsupported)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::irp::Irp;

    #[test]
    fn stub_read_returns_unsupported() {
        let mut irp = Irp::new_read(None);
        assert_eq!(
            stub_pagefile_issue_read_irp(&mut irp, 0, 4096),
            Err(PageFileIoError::Unsupported)
        );
    }
}
