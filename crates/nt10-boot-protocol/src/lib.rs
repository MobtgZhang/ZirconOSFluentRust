//! Handoff structures between ZBM10 (UEFI) and the NT10 kernel.
//! Layout is stable across crates; bump `ZIRCON_BOOT_INFO_VERSION` on incompatible changes.

#![no_std]

/// Magic `"ZIRNON10"` — same as historical `ZIRNON10_MAGIC` in boot stub docs.
pub const ZIRNON10_MAGIC: u64 = 0x5A49_524E_4F4E_3130;

/// Increment when `ZirconBootInfo` or descriptor layout breaks compatibility.
pub const ZIRCON_BOOT_INFO_VERSION: u32 = 2;

/// UEFI `EFI_MEMORY_DESCRIPTOR` layout (UEFI 2.x, 48 bytes with natural alignment).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HandoffMemoryDescriptor {
    pub r#type: u32,
    pub _padding: u32,
    pub physical_start: u64,
    pub virtual_start: u64,
    pub number_of_pages: u64,
    pub attribute: u64,
}

// Matches `r_efi::efi::MemoryDescriptor` / UEFI `EFI_MEMORY_DESCRIPTOR` (40 bytes on x86_64 UEFI).
const _: () = assert!(core::mem::size_of::<HandoffMemoryDescriptor>() == 40);

/// Framebuffer as seen after GOP (kernel may reinterpret `pixel_format`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FramebufferInfo {
    pub base: u64,
    pub size: usize,
    pub horizontal_resolution: u32,
    pub vertical_resolution: u32,
    pub pixels_per_scan_line: u32,
    /// Opaque GOP pixel format value from firmware.
    pub pixel_format: u32,
}

/// Parameters passed from ZBM10 to the kernel entry (physical pointer until kernel maps itself).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ZirconBootInfo {
    pub magic: u64,
    pub version: u32,
    pub reserved0: u32,
    /// Pointer to an array of `HandoffMemoryDescriptor` in **physical** address space.
    pub mem_map: *mut HandoffMemoryDescriptor,
    pub mem_map_count: usize,
    /// Bytes per descriptor (from UEFI `GetMemoryMap`).
    pub mem_map_descriptor_size: usize,
    pub framebuffer: FramebufferInfo,
    /// ACPI 2.0 RSDP (physical), or 0 if not found.
    pub acpi_rsdp: u64,
    /// Physical entry point jumped to after handoff; 0 if not used yet.
    pub kernel_entry_phys: u64,
    pub cmdline: [u8; 256],
    /// Optional initrd base (physical), 0 if none.
    pub initrd_phys: u64,
    pub initrd_size: u64,
    /// MMIO base for TPM (platform-specific), 0 if unknown.
    pub tpm_mmio_phys: u64,
    pub tpm_mmio_size: u64,
    /// SMBIOS entry anchor (physical), 0 if unknown.
    pub smbios_anchor_phys: u64,
    /// Optional entropy from firmware RNG protocol (may be zeroed).
    pub firmware_rng_seed: [u8; 16],
    /// ACPI RSDP revision field when available.
    pub acpi_rsdp_revision: u8,
    pub _reserved_tail: [u8; 7],
}

impl Default for ZirconBootInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl ZirconBootInfo {
    pub const fn new() -> Self {
        Self {
            magic: ZIRNON10_MAGIC,
            version: ZIRCON_BOOT_INFO_VERSION,
            reserved0: 0,
            mem_map: core::ptr::null_mut(),
            mem_map_count: 0,
            mem_map_descriptor_size: 0,
            framebuffer: FramebufferInfo {
                base: 0,
                size: 0,
                horizontal_resolution: 0,
                vertical_resolution: 0,
                pixels_per_scan_line: 0,
                pixel_format: 0,
            },
            acpi_rsdp: 0,
            kernel_entry_phys: 0,
            cmdline: [0; 256],
            initrd_phys: 0,
            initrd_size: 0,
            tpm_mmio_phys: 0,
            tpm_mmio_size: 0,
            smbios_anchor_phys: 0,
            firmware_rng_seed: [0; 16],
            acpi_rsdp_revision: 0,
            _reserved_tail: [0; 7],
        }
    }

    #[must_use]
    pub fn validate(&self) -> bool {
        self.magic == ZIRNON10_MAGIC && self.version == ZIRCON_BOOT_INFO_VERSION
    }
}
