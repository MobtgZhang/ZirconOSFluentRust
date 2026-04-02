//! VirtIO block over MMIO — QEMU `virtio-blk` registration stub (no queue programming yet).
//!
//! Future: discover via ACPI/PCI, negotiate features, attach to [`crate::io::device::BlockVolumeBringup`].

/// Magic value at offset 0 of a VirtIO 1.0 MMIO transport (`virtio` little-endian).
pub const VIRTIO_MMIO_MAGIC_VALUE: u32 = 0x74726976;

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
