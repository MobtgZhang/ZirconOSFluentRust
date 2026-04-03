//! I/O manager core — IRP dispatch façade.

use super::irp::Irp;
use crate::io::device::{BlockVolumeBringup, RamdiskDevice};
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

/// Ramdisk READ path: fills `buf` from `vol` at `*position`, completes `irp`, advances cursor.
/// End-to-end bring-up hook until a stacked [`DeviceObjectStub`] owns state.
pub fn io_read_ramdisk_complete_irp(
    vol: &RamdiskDevice,
    position: &mut u64,
    buf: &mut [u8],
    irp: &mut Irp,
) -> i32 {
    let n = vol.read_at(*position, buf);
    *position += n as u64;
    io_complete_request(irp, 0, n);
    0
}

/// Dispatch block read: ramdisk or VirtIO-MMIO polling (bare-metal x86_64 only).
pub fn io_read_block_volume_complete_irp(
    vol: &BlockVolumeBringup,
    position: &mut u64,
    buf: &mut [u8],
    irp: &mut Irp,
) -> i32 {
    match vol {
        BlockVolumeBringup::Ramdisk(d) => io_read_ramdisk_complete_irp(d, position, buf, irp),
        BlockVolumeBringup::VirtioMmio(p) => {
            #[cfg(all(target_arch = "x86_64", not(test)))]
            unsafe {
                match (**p).read_at_byte_offset(*position, buf) {
                    Ok(n) => {
                        *position += n as u64;
                        io_complete_request(irp, 0, n);
                        0
                    }
                    Err(()) => {
                        io_complete_request(irp, -5, 0);
                        -5
                    }
                }
            }
            #[cfg(not(all(target_arch = "x86_64", not(test))))]
            {
                let _ = (p, position, buf);
                io_complete_request(irp, -1, 0);
                -1
            }
        }
    }
}
