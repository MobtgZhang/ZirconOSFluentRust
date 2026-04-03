//! VirtIO 1.x MMIO register layout (transport only; see public VirtIO spec).
//!
//! All offsets are byte offsets from the device MMIO base (identity-mapped in bring-up).

/// `virtio` little-endian.
pub const MMIO_MAGIC: u32 = 0x7472_6976;

pub const OFF_MAGIC: usize = 0x000;
pub const OFF_VERSION: usize = 0x004;
pub const OFF_DEVICE_ID: usize = 0x008;
pub const OFF_VENDOR_ID: usize = 0x00c;
pub const OFF_DEVICE_FEATURES: usize = 0x010;
pub const OFF_DEVICE_FEATURES_SEL: usize = 0x014;
pub const OFF_DRIVER_FEATURES: usize = 0x020;
pub const OFF_DRIVER_FEATURES_SEL: usize = 0x024;
pub const OFF_QUEUE_SEL: usize = 0x030;
pub const OFF_QUEUE_NUM_MAX: usize = 0x034;
pub const OFF_QUEUE_NUM: usize = 0x038;
pub const OFF_QUEUE_READY: usize = 0x044;
pub const OFF_QUEUE_NOTIFY: usize = 0x050;
pub const OFF_INTERRUPT_STATUS: usize = 0x060;
pub const OFF_INTERRUPT_ACK: usize = 0x064;
pub const OFF_STATUS: usize = 0x070;
pub const OFF_QUEUE_DESC_LOW: usize = 0x080;
pub const OFF_QUEUE_DESC_HIGH: usize = 0x084;
pub const OFF_QUEUE_DRIVER_LOW: usize = 0x090;
pub const OFF_QUEUE_DRIVER_HIGH: usize = 0x094;
pub const OFF_QUEUE_DEVICE_LOW: usize = 0x0a0;
pub const OFF_QUEUE_DEVICE_HIGH: usize = 0x0a4;
pub const OFF_CONFIG0: usize = 0x100;

pub const STATUS_ACKNOWLEDGE: u32 = 1;
pub const STATUS_DRIVER: u32 = 2;
pub const STATUS_DRIVER_OK: u32 = 4;
pub const STATUS_FEATURES_OK: u32 = 8;
pub const STATUS_FAILED: u32 = 128;

/// Feature bit 32: `VIRTIO_F_VERSION_1`.
pub const FEATURE_VERSION_1: u64 = 1u64 << 32;

#[inline]
unsafe fn reg_ptr(base: u64, off: usize) -> *mut u32 {
    (base as usize + off) as *mut u32
}

#[inline]
pub unsafe fn read32(base: u64, off: usize) -> u32 {
    core::ptr::read_volatile(reg_ptr(base, off))
}

#[inline]
pub unsafe fn write32(base: u64, off: usize, v: u32) {
    core::ptr::write_volatile(reg_ptr(base, off), v);
}

#[inline]
pub unsafe fn read_config_le64(base: u64, cfg_off: usize) -> u64 {
    let p = (base as usize + OFF_CONFIG0 + cfg_off) as *const u64;
    u64::from_le(core::ptr::read_volatile(p))
}
