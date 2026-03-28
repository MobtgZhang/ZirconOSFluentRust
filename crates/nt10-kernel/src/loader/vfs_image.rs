//! Read a mounted volume image into a buffer via the VFS + IRP path.

use crate::fs::vfs::{vfs_read_via_irp, OpenFileHandle, VfsTable};
use crate::io::irp::Irp;
use core::ptr::NonNull;

/// Reads up to `dest.len()` bytes from mount `slot` (must have [`crate::fs::vfs::VfsMountPoint::block_volume`]).
#[must_use]
pub fn read_mount_into_buffer(vfs: &VfsTable, slot: usize, dest: &mut [u8]) -> Result<usize, ()> {
    let mut h = OpenFileHandle::open_mount(vfs, slot)?;
    let mut total = 0usize;
    while total < dest.len() {
        let mut irp = Irp::new_read(Some(NonNull::dangling()));
        let before = total;
        let _rc = vfs_read_via_irp(vfs, &mut h, &mut dest[total..], &mut irp);
        let n = irp.information;
        if n == 0 {
            break;
        }
        total += n;
        if total == before {
            break;
        }
    }
    Ok(total)
}
