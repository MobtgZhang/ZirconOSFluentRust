//! VirtIO block over MMIO — QEMU `virtio-blk` registration stub (no queue programming yet).
//!
//! Bring-up: when the firmware or loader exposes the guest disk as a **linear byte image** in memory,
//! use [`read_sectors_from_linear_image`] and mount via [`crate::fs::vfs::VfsMountPoint::with_virtio_blk_linear_image`].
//! Future: MMIO transport + virtqueue → same VFS slot.

/// Magic value at offset 0 of a VirtIO 1.0 MMIO transport (`virtio` little-endian).
pub const VIRTIO_MMIO_MAGIC_VALUE: u32 = 0x74726976;

/// Classic virtio-blk logical sector size for LBA math.
pub const VIRTIO_BLK_SECTOR_SIZE: usize = 512;

/// Transport fields used when wiring a bring-up driver (offsets per VirtIO 1.0 spec).
#[derive(Clone, Copy, Debug)]
pub struct VirtioMmioTransportStub {
    pub phys_base: u64,
    pub irq_gsi: u32,
}

impl VirtioMmioTransportStub {
    #[must_use]
    pub const fn new(phys_base: u64) -> Self {
        Self {
            phys_base,
            irq_gsi: 0,
        }
    }
}

/// Synchronous read of contiguous sectors from a whole-disk image slice (RAM-staged virtio image).
#[must_use]
pub fn read_sectors_from_linear_image(
    image: &[u8],
    start_lba: u64,
    buf: &mut [u8],
) -> usize {
    if buf.is_empty() {
        return 0;
    }
    let off = match (start_lba as usize).checked_mul(VIRTIO_BLK_SECTOR_SIZE) {
        Some(o) => o,
        None => return 0,
    };
    if off >= image.len() {
        return 0;
    }
    let avail = image.len() - off;
    let n = buf.len().min(avail);
    buf[..n].copy_from_slice(&image[off..off + n]);
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_sector_read_matches_offset() {
        let mut img = [0u8; 1024];
        img[512..520].copy_from_slice(b"SECTOR01");
        let mut out = [0u8; 8];
        let n = read_sectors_from_linear_image(&img, 1, &mut out);
        assert_eq!(n, 8);
        assert_eq!(&out, b"SECTOR01");
    }
}
