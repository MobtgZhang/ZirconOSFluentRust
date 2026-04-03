//! VirtIO block — linear RAM image (bring-up) and optional **VirtIO 1.x MMIO** polling path.
//!
//! MMIO path assumes identity-mapped MMIO and queue memory (common with QEMU `virtio-mmio`).

#![cfg_attr(
    test,
    allow(dead_code, unused_imports, unused_variables, unused_mut)
)]

pub use super::virtio_mmio::MMIO_MAGIC as VIRTIO_MMIO_MAGIC_VALUE;
pub use super::virtio_virtqueue::VIRTIO_BLK_T_IN;

use super::virtio_mmio::{
    read32, write32, OFF_CONFIG0, OFF_DEVICE_FEATURES, OFF_DEVICE_FEATURES_SEL, OFF_DRIVER_FEATURES,
    OFF_DRIVER_FEATURES_SEL, OFF_INTERRUPT_ACK, OFF_INTERRUPT_STATUS, OFF_MAGIC, OFF_QUEUE_DESC_HIGH,
    OFF_QUEUE_DESC_LOW, OFF_QUEUE_DEVICE_HIGH, OFF_QUEUE_DEVICE_LOW, OFF_QUEUE_DRIVER_HIGH,
    OFF_QUEUE_DRIVER_LOW, OFF_QUEUE_NOTIFY, OFF_QUEUE_NUM, OFF_QUEUE_NUM_MAX, OFF_QUEUE_READY,
    OFF_QUEUE_SEL, OFF_STATUS, OFF_VENDOR_ID, OFF_VERSION, STATUS_ACKNOWLEDGE, STATUS_DRIVER,
    STATUS_DRIVER_OK, STATUS_FEATURES_OK,
};
use super::virtio_virtqueue::{
    ring_layout, VirtioBlkReqLe, VRING_DESC_F_NEXT, VRING_DESC_F_WRITE, VIRTIO_BLK_S_OK, VringDesc,
};

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

    /// # Safety
    /// `phys_base` must be identity-mapped and reference at least 4 bytes of MMIO.
    #[must_use]
    pub unsafe fn read_magic_identity_mapped(self) -> u32 {
        read32(self.phys_base, OFF_MAGIC)
    }

    #[must_use]
    pub fn magic_matches_identity_mapped(self) -> bool {
        if self.phys_base == 0 {
            return false;
        }
        #[cfg(all(target_arch = "x86_64", not(test)))]
        unsafe {
            return self.read_magic_identity_mapped() == super::virtio_mmio::MMIO_MAGIC;
        }
        #[cfg(any(not(target_arch = "x86_64"), test))]
        {
            let _ = self;
            false
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VirtioBlkInitError {
    BadMagic,
    BadVersion,
    NotBlockDevice,
    QueueTooSmall,
    Layout,
    FeatureNegotiation,
}

/// Polling virtio-blk over MMIO (queue 0, synchronous sector read).
pub struct VirtioBlkMmioBringup {
    mmio_base: u64,
    capacity_sectors: u64,
    slab: [u8; 4096],
    qsz: u16,
    desc_off: u16,
    avail_off: u16,
    used_off: u16,
    data_off: u16,
    last_used_idx: u16,
    next_avail_idx: u16,
}

unsafe fn write_desc(ptr: *mut u8, addr: u64, len: u32, flags: u16, next: u16) {
    core::ptr::write_unaligned(ptr as *mut u64, addr.to_le());
    core::ptr::write_unaligned(ptr.add(8) as *mut u32, len.to_le());
    core::ptr::write_unaligned(ptr.add(12) as *mut u16, flags.to_le());
    core::ptr::write_unaligned(ptr.add(14) as *mut u16, next.to_le());
}

impl VirtioBlkMmioBringup {
    /// # Safety
    /// `mmio_base` must reference a virtio-mmio block device; queue memory is identity-mapped.
    pub unsafe fn init(mmio_base: u64) -> Result<Self, VirtioBlkInitError> {
        if read32(mmio_base, OFF_MAGIC) != super::virtio_mmio::MMIO_MAGIC {
            return Err(VirtioBlkInitError::BadMagic);
        }
        if read32(mmio_base, OFF_VERSION) != 2 {
            return Err(VirtioBlkInitError::BadVersion);
        }
        if read32(mmio_base, super::virtio_mmio::OFF_DEVICE_ID) != 2 {
            return Err(VirtioBlkInitError::NotBlockDevice);
        }
        let _ = read32(mmio_base, OFF_VENDOR_ID);

        write32(mmio_base, OFF_STATUS, 0);
        write32(mmio_base, OFF_STATUS, STATUS_ACKNOWLEDGE);
        write32(mmio_base, OFF_STATUS, STATUS_ACKNOWLEDGE | STATUS_DRIVER);

        write32(mmio_base, OFF_DEVICE_FEATURES_SEL, 1);
        let dev_hi = read32(mmio_base, OFF_DEVICE_FEATURES);
        if dev_hi & 1 == 0 {
            return Err(VirtioBlkInitError::FeatureNegotiation);
        }
        write32(mmio_base, OFF_DRIVER_FEATURES_SEL, 1);
        write32(mmio_base, OFF_DRIVER_FEATURES, 1);
        write32(mmio_base, OFF_DEVICE_FEATURES_SEL, 0);
        write32(mmio_base, OFF_DRIVER_FEATURES_SEL, 0);
        write32(mmio_base, OFF_DRIVER_FEATURES, 0);

        write32(
            mmio_base,
            OFF_STATUS,
            STATUS_ACKNOWLEDGE | STATUS_DRIVER | STATUS_FEATURES_OK,
        );

        write32(mmio_base, OFF_QUEUE_SEL, 0);
        let qmax = read32(mmio_base, OFF_QUEUE_NUM_MAX);
        if qmax < 1 {
            return Err(VirtioBlkInitError::QueueTooSmall);
        }
        let qsz = 4u16.min(qmax as u16).max(1);
        write32(mmio_base, OFF_QUEUE_NUM, u32::from(qsz));

        let (d_off, a_off, u_off, dt_off, need) =
            ring_layout(qsz as usize).ok_or(VirtioBlkInitError::Layout)?;
        if need > 4096 {
            return Err(VirtioBlkInitError::Layout);
        }

        let mut s = Self {
            mmio_base,
            capacity_sectors: 0,
            slab: [0u8; 4096],
            qsz,
            desc_off: d_off as u16,
            avail_off: a_off as u16,
            used_off: u_off as u16,
            data_off: dt_off as u16,
            last_used_idx: 0,
            next_avail_idx: 0,
        };

        let slab_pa = s.slab.as_ptr() as u64;
        let desc_pa = slab_pa + u64::from(s.desc_off);
        let avail_pa = slab_pa + u64::from(s.avail_off);
        let used_pa = slab_pa + u64::from(s.used_off);

        write32(mmio_base, OFF_QUEUE_DESC_LOW, desc_pa as u32);
        write32(mmio_base, OFF_QUEUE_DESC_HIGH, (desc_pa >> 32) as u32);
        write32(mmio_base, OFF_QUEUE_DRIVER_LOW, avail_pa as u32);
        write32(mmio_base, OFF_QUEUE_DRIVER_HIGH, (avail_pa >> 32) as u32);
        write32(mmio_base, OFF_QUEUE_DEVICE_LOW, used_pa as u32);
        write32(mmio_base, OFF_QUEUE_DEVICE_HIGH, (used_pa >> 32) as u32);

        write32(mmio_base, OFF_QUEUE_READY, 1);

        write32(
            mmio_base,
            OFF_STATUS,
            STATUS_ACKNOWLEDGE | STATUS_DRIVER | STATUS_FEATURES_OK | STATUS_DRIVER_OK,
        );

        let cap_lo = read32(mmio_base, OFF_CONFIG0);
        let cap_hi = read32(mmio_base, OFF_CONFIG0 + 4);
        s.capacity_sectors = u64::from(cap_lo) | (u64::from(cap_hi) << 32);

        Ok(s)
    }

    #[must_use]
    pub fn capacity_sectors(&self) -> u64 {
        self.capacity_sectors
    }

    pub fn read_at_byte_offset(&mut self, offset: u64, buf: &mut [u8]) -> Result<usize, ()> {
        #[cfg(all(target_arch = "x86_64", not(test)))]
        {
            if buf.is_empty() {
                return Ok(0);
            }
            let mut total = 0usize;
            while total < buf.len() {
                let pos = offset.saturating_add(total as u64);
                let lba = pos / 512;
                if lba >= self.capacity_sectors {
                    break;
                }
                let sec_off = (pos % 512) as usize;
                let chunk = (VIRTIO_BLK_SECTOR_SIZE - sec_off).min(buf.len() - total);
                unsafe {
                    self.read_one_sector_inner(lba, sec_off, &mut buf[total..total + chunk])?;
                }
                total += chunk;
            }
            Ok(total)
        }
        #[cfg(not(all(target_arch = "x86_64", not(test))))]
        {
            let _ = (offset, buf);
            Err(())
        }
    }

    #[cfg(all(target_arch = "x86_64", not(test)))]
    unsafe fn read_one_sector_inner(
        &mut self,
        lba: u64,
        byte_off_in_sector: usize,
        buf: &mut [u8],
    ) -> Result<(), ()> {
        if buf.is_empty() || byte_off_in_sector >= VIRTIO_BLK_SECTOR_SIZE {
            return Ok(());
        }
        if byte_off_in_sector + buf.len() > VIRTIO_BLK_SECTOR_SIZE {
            return Err(());
        }

        let slab = &mut self.slab[..];
        let dt = self.data_off as usize;
        let req_ptr = slab.as_mut_ptr().add(dt);
        let data_ptr = req_ptr.add(16);
        let status_ptr = data_ptr.add(VIRTIO_BLK_SECTOR_SIZE);

        req_ptr.write_bytes(0, 16);
        req_ptr
            .cast::<u32>()
            .write_volatile((VIRTIO_BLK_T_IN as u32).to_le());
        req_ptr.add(8).cast::<u64>().write_volatile(lba.to_le());

        data_ptr.write_bytes(0, VIRTIO_BLK_SECTOR_SIZE);
        status_ptr.write_volatile(0xFFu8);

        let desc_base = self.desc_off as usize;
        let d0 = slab.as_mut_ptr().add(desc_base);
        let d1 = d0.add(core::mem::size_of::<VringDesc>());
        let d2 = d1.add(core::mem::size_of::<VringDesc>());

        let req_pa = req_ptr as u64;
        let data_pa = data_ptr as u64;
        let st_pa = status_ptr as u64;

        write_desc(
            d0,
            req_pa,
            core::mem::size_of::<VirtioBlkReqLe>() as u32,
            VRING_DESC_F_NEXT,
            1,
        );
        write_desc(
            d1,
            data_pa,
            VIRTIO_BLK_SECTOR_SIZE as u32,
            VRING_DESC_F_NEXT | VRING_DESC_F_WRITE,
            2,
        );
        write_desc(d2, st_pa, 1, VRING_DESC_F_WRITE, 0);

        let avail_base = self.avail_off as usize;
        let av = slab.as_mut_ptr().add(avail_base);
        let head = 0u16;
        let slot = (self.next_avail_idx % self.qsz) as usize;
        av.cast::<u16>().write_volatile(0);
        av.add(2).cast::<u16>().write_volatile(0);
        av.add(4 + slot * 2).cast::<u16>().write_volatile(head);
        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
        self.next_avail_idx = self.next_avail_idx.wrapping_add(1);
        av.add(2).cast::<u16>().write_volatile(self.next_avail_idx);

        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
        write32(self.mmio_base, OFF_QUEUE_NOTIFY, 0);

        let used_base = self.used_off as usize;
        let used_idx_ptr = slab.as_mut_ptr().add(used_base + 2).cast::<u16>();
        let want = self.last_used_idx.wrapping_add(1);
        let mut spins = 0usize;
        while used_idx_ptr.read_volatile() != want {
            spins = spins.saturating_add(1);
            if spins > 10_000_000 {
                return Err(());
            }
            core::hint::spin_loop();
        }

        let intr = read32(self.mmio_base, OFF_INTERRUPT_STATUS);
        if intr != 0 {
            write32(self.mmio_base, OFF_INTERRUPT_ACK, intr);
        }

        if status_ptr.read_volatile() != VIRTIO_BLK_S_OK {
            return Err(());
        }

        core::ptr::copy_nonoverlapping(
            data_ptr.add(byte_off_in_sector),
            buf.as_mut_ptr(),
            buf.len(),
        );

        self.last_used_idx = want;
        Ok(())
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
