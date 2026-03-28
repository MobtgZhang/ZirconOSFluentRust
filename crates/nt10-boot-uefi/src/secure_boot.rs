//! Optional UEFI security protocols — hooks only; verification logic is added with a project crypto stack.
//!
//! `EFI_SECURITY2_ARCHITECTURE_PROTOCOL` GUID is defined in the public UEFI specification.

use core::ffi::c_void;
use core::ptr;

use r_efi::efi;

/// Spec: `EFI_SECURITY2_ARCHITECTURE_PROTOCOL`.
pub const SECURITY2_ARCH_PROTOCOL_GUID: efi::Guid = efi::Guid::from_fields(
    0xc096_cadb,
    0x38a9,
    0x4238,
    0xb2,
    0x80,
    &[0x2c, 0xe0, 0x23, 0xf7, 0xfc, 0xe2],
);

/// If the firmware exposes `EFI_SECURITY2_ARCHITECTURE_PROTOCOL`, we record readiness for future
/// `FileAuthenticationState` integration. Currently a no-op success path.
pub fn optional_pre_boot_security_hook(st: *mut efi::SystemTable) -> Result<(), efi::Status> {
    let bs = unsafe { (*st).boot_services };
    if bs.is_null() {
        return Err(efi::Status::INVALID_PARAMETER);
    }
    let mut iface: *mut c_void = ptr::null_mut();
    let r = unsafe {
        ((*bs).locate_protocol)(
            &SECURITY2_ARCH_PROTOCOL_GUID as *const _ as *mut _,
            ptr::null_mut(),
            &mut iface,
        )
    };
    // Optional protocol: treat common "not available" statuses like NOT_FOUND so quirky firmware
    // does not abort boot before the kernel is loaded.
    if matches!(
        r,
        efi::Status::NOT_FOUND | efi::Status::UNSUPPORTED | efi::Status::ACCESS_DENIED
    ) {
        return Ok(());
    }
    if r != efi::Status::SUCCESS {
        return Err(r);
    }
    let _ = iface;
    Ok(())
}
