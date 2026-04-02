//! Non-volatile EFI variable for "remember last boot entry" (avoids FAT truncate quirks).

use core::ffi::c_void;

use r_efi::efi;

/// Vendor GUID for ZBM10 boot preferences (not a Microsoft GUID).
pub const ZBM_NV_GUID: efi::Guid = efi::Guid::from_fields(
    0x7c9e_2b1a,
    0x3f4d,
    0x5e6f,
    0xa7,
    0xb8,
    &[0xc9, 0x0d, 0x1e, 0x2f, 0x3a, 0x4b],
);

// UCS-2 "ZbmLastSel" + NUL
const NAME_ZBM_LAST_SEL: [efi::Char16; 11] = [
    0x005a, 0x0062, 0x006d, 0x004c, 0x0061, 0x0073, 0x0074, 0x0053, 0x0065, 0x006c, 0,
];

pub unsafe fn read_last_entry(st: *mut efi::SystemTable) -> Option<u8> {
    let rt = (*st).runtime_services;
    if rt.is_null() {
        return None;
    }
    let mut data = 0u8;
    let mut sz = core::mem::size_of::<u8>();
    let mut attrs = 0u32;
    let r = ((*rt).get_variable)(
        NAME_ZBM_LAST_SEL.as_ptr() as *mut efi::Char16,
        &ZBM_NV_GUID as *const _ as *mut _,
        &mut attrs,
        &mut sz,
        (&mut data) as *mut u8 as *mut c_void,
    );
    if r != efi::Status::SUCCESS || sz != 1 {
        return None;
    }
    Some(data)
}

pub unsafe fn write_last_entry(st: *mut efi::SystemTable, entry: u8) {
    let rt = (*st).runtime_services;
    if rt.is_null() {
        return;
    }
    let mut v = entry;
    let attrs = efi::VARIABLE_NON_VOLATILE | efi::VARIABLE_BOOTSERVICE_ACCESS;
    let _ = ((*rt).set_variable)(
        NAME_ZBM_LAST_SEL.as_ptr() as *mut efi::Char16,
        &ZBM_NV_GUID as *const _ as *mut _,
        attrs,
        1,
        (&mut v) as *mut u8 as *mut c_void,
    );
}
