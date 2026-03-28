//! PCI configuration space access (legacy I/O CF8/CFC) and xHCI device discovery.
//!
//! Assumes **identity-mapped** I/O ports (x86_64 BSP). Used to locate USB3 xHCI (class 0x0C / 0x03 / PI 0x30).

#[cfg(target_arch = "x86_64")]
use core::arch::asm;

pub const PCI_CONFIG_ADDR: u16 = 0xCF8;
pub const PCI_CONFIG_DATA: u16 = 0xCFC;

/// USB xHCI programming interface (base class 0x0C, subclass 0x03, PI 0x30).
pub const PCI_CLASS_XHCI: u32 = 0x0C0300;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PciBarError {
    Unused,
    IoSpace,
    Unsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PciXhciLocation {
    pub bus: u8,
    pub dev: u8,
    pub func: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PciMmioBar {
    pub phys_base: u64,
    pub size: u64,
}

#[inline]
const fn pci_config_addr(bus: u8, dev: u8, func: u8, offset: u8) -> u32 {
    0x8000_0000
        | ((bus as u32) << 16)
        | (((dev as u32) & 0x1F) << 11)
        | (((func as u32) & 7) << 8)
        | ((offset as u32) & 0xFC)
}

/// Read 32-bit word from PCI config space (`offset` must be 4-byte aligned).
#[cfg(target_arch = "x86_64")]
pub unsafe fn read_config_u32(bus: u8, dev: u8, func: u8, offset: u8) -> u32 {
    unsafe {
        let addr = pci_config_addr(bus, dev, func, offset);
        asm!(
            "out dx, eax",
            in("dx") PCI_CONFIG_ADDR,
            in("eax") addr,
            options(nomem, nostack, preserves_flags),
        );
        let mut v: u32;
        asm!(
            "in eax, dx",
            out("eax") v,
            in("dx") PCI_CONFIG_DATA,
            options(nomem, nostack, preserves_flags),
        );
        v
    }
}

#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn read_config_u32(_bus: u8, _dev: u8, _func: u8, _offset: u8) -> u32 {
    0xFFFF_FFFF
}

/// Returns `(class_revision, vendor_device)` style: high 24 bits are class/sub/prog-if; low byte revision.
#[inline]
pub fn read_class_revision(bus: u8, dev: u8, func: u8) -> u32 {
    unsafe { read_config_u32(bus, dev, func, 0x08) }
}

/// Decode BAR0 (and BAR1 if 64-bit) into a **prefetchable or non-prefetchable** MMIO base + size.
/// `bar_index` is 0 for BAR0, 2 for BAR2, etc. (only 0 implemented for xhci bring-up).
pub unsafe fn read_mmio_bar(bus: u8, dev: u8, func: u8, bar_index: u8) -> Result<PciMmioBar, PciBarError> {
    let off = 0x10 + bar_index * 4;
    let raw_lo = read_config_u32(bus, dev, func, off);
    if raw_lo == 0 || raw_lo == 0xFFFF_FFFF {
        return Err(PciBarError::Unused);
    }
    if (raw_lo & 1) != 0 {
        return Err(PciBarError::IoSpace);
    }
    let is_64 = (raw_lo & 0b110) == 0b100;
    let low_mask = !(0xFu32);
    let base_lo = (raw_lo & low_mask) as u64;
    let raw_hi = if is_64 {
        read_config_u32(bus, dev, func, off + 4)
    } else {
        0
    };
    let base_hi = raw_hi as u64;
    let phys_base = (base_hi << 32) | base_lo;

    // Size probe: write all-1s, read back, restore.
    write_config_u32(bus, dev, func, off, 0xFFFF_FFFF);
    let mask_lo = read_config_u32(bus, dev, func, off);
    let mask_hi = if is_64 {
        write_config_u32(bus, dev, func, off + 4, 0xFFFF_FFFF);
        read_config_u32(bus, dev, func, off + 4)
    } else {
        0
    };
    write_config_u32(bus, dev, func, off, raw_lo);
    if is_64 {
        write_config_u32(bus, dev, func, off + 4, raw_hi);
    }

    let sz_lo = !(mask_lo & low_mask) as u64;
    let sz_hi = !(mask_hi as u64);
    let size = if is_64 {
        ((sz_hi << 32) | sz_lo).wrapping_add(1)
    } else {
        sz_lo.wrapping_add(1)
    };
    if size == 0 {
        return Err(PciBarError::Unsupported);
    }
    Ok(PciMmioBar { phys_base, size })
}

/// Enable **memory space** and **bus mastering** (required for DMA).
pub unsafe fn enable_mmio_and_bus_master(bus: u8, dev: u8, func: u8) {
    let r = read_config_u32(bus, dev, func, 0x04);
    let cmd = (r & 0xFFFF) | 0x0006; // memory + bus master
    write_config_u32(bus, dev, func, 0x04, (r & 0xFFFF_0000) | cmd);
}

#[cfg(target_arch = "x86_64")]
unsafe fn write_config_u32(bus: u8, dev: u8, func: u8, offset: u8, val: u32) {
    unsafe {
        let addr = pci_config_addr(bus, dev, func, offset);
        asm!(
            "out dx, eax",
            in("dx") PCI_CONFIG_ADDR,
            in("eax") addr,
            options(nomem, nostack, preserves_flags),
        );
        asm!(
            "out dx, eax",
            in("dx") PCI_CONFIG_DATA,
            in("eax") val,
            options(nomem, nostack, preserves_flags),
        );
    }
}

#[cfg(not(target_arch = "x86_64"))]
unsafe fn write_config_u32(_bus: u8, _dev: u8, _func: u8, _offset: u8, _val: u32) {}

/// Scan bus 0..=`bus_max`, devices 0..31, function 0 only (sufficient for QEMU q35 root ports).
pub fn find_first_xhci_mmio_bar(bus_max: u8) -> Option<(PciXhciLocation, PciMmioBar)> {
    for bus in 0..=bus_max {
        for dev in 0..32u8 {
            let vid_did = unsafe { read_config_u32(bus, dev, 0, 0) };
            if vid_did == 0xFFFF_FFFF || vid_did == 0 {
                continue;
            }
            let cls = read_class_revision(bus, dev, 0);
            let class = (cls >> 24) & 0xFF;
            let sub = (cls >> 16) & 0xFF;
            let pif = (cls >> 8) & 0xFF;
            if class != 0x0C || sub != 0x03 || pif != 0x30 {
                continue;
            }
            if let Ok(bar) = unsafe { read_mmio_bar(bus, dev, 0, 0) } {
                unsafe {
                    enable_mmio_and_bus_master(bus, dev, 0);
                }
                return Some((PciXhciLocation { bus, dev, func: 0 }, bar));
            }
        }
    }
    None
}
