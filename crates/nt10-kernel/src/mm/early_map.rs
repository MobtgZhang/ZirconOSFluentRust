//! Summarize UEFI memory descriptors handed off from ZBM10 (public UEFI type values).
//!
//! For a PFN-oriented view (usable conventional runs minus the kernel image reservation), see
//! [`crate::mm::boot_mem`].

use crate::handoff::{HandoffMemoryDescriptor, ZirconBootInfo};

/// `EfiConventionalMemory` (UEFI spec).
pub const EFI_MEMORY_CONVENTIONAL: u32 = 7;
/// `EfiLoaderCode`
pub const EFI_MEMORY_LOADER_CODE: u32 = 1;
/// `EfiLoaderData`
pub const EFI_MEMORY_LOADER_DATA: u32 = 2;
/// `EfiBootServicesCode`
pub const EFI_MEMORY_BOOT_SERVICES_CODE: u32 = 4;
/// `EfiBootServicesData`
pub const EFI_MEMORY_BOOT_SERVICES_DATA: u32 = 5;
/// `EfiACPIReclaimMemory`
pub const EFI_MEMORY_ACPI_RECLAIM: u32 = 9;
/// `EfiACPIMemoryNVS`
pub const EFI_MEMORY_ACPI_NVS: u32 = 10;
/// `EfiRuntimeServicesCode`
pub const EFI_MEMORY_RUNTIME_CODE: u32 = 12;
/// `EfiRuntimeServicesData`
pub const EFI_MEMORY_RUNTIME_DATA: u32 = 13;

/// # Safety
/// `info` must satisfy `validate()`, `mem_map` must point to `mem_map_count` valid descriptors.
#[must_use]
pub unsafe fn conventional_page_count(info: &ZirconBootInfo) -> u64 {
    if info.mem_map.is_null() || info.mem_map_count == 0 {
        return 0;
    }
    let mut pages = 0u64;
    for i in 0..info.mem_map_count {
        let d = &*info.mem_map.add(i);
        if d.r#type == EFI_MEMORY_CONVENTIONAL {
            pages = pages.saturating_add(d.number_of_pages);
        }
    }
    pages
}

/// Pages that should remain tracked after ExitBootServices for ACPI / runtime services.
#[must_use]
pub unsafe fn reserved_firmware_page_count(info: &ZirconBootInfo) -> u64 {
    if info.mem_map.is_null() || info.mem_map_count == 0 {
        return 0;
    }
    let mut pages = 0u64;
    for i in 0..info.mem_map_count {
        let d = &*info.mem_map.add(i);
        if matches!(
            d.r#type,
            EFI_MEMORY_ACPI_RECLAIM
                | EFI_MEMORY_ACPI_NVS
                | EFI_MEMORY_RUNTIME_CODE
                | EFI_MEMORY_RUNTIME_DATA
        ) {
            pages = pages.saturating_add(d.number_of_pages);
        }
    }
    pages
}

#[must_use]
pub fn descriptor_slice<'a>(info: &'a ZirconBootInfo) -> Option<&'a [HandoffMemoryDescriptor]> {
    if info.mem_map.is_null() || info.mem_map_count == 0 {
        return None;
    }
    Some(unsafe { core::slice::from_raw_parts(info.mem_map, info.mem_map_count) })
}
