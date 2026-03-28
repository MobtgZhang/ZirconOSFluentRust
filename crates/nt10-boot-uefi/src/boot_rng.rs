//! Fill `firmware_rng_seed` using `EFI_RNG_PROTOCOL` when present.

use core::ffi::c_void;
use core::ptr;

use r_efi::efi;
use r_efi::efi::protocols::rng;

pub unsafe fn try_fill_seed(st: *mut efi::SystemTable, out: &mut [u8; 16]) {
    let bs = (*st).boot_services;
    if bs.is_null() {
        return;
    }
    let mut iface: *mut c_void = ptr::null_mut();
    let r = ((*bs).locate_protocol)(
        &rng::PROTOCOL_GUID as *const _ as *mut _,
        ptr::null_mut(),
        &mut iface,
    );
    if r != efi::Status::SUCCESS {
        return;
    }
    let proto = iface.cast::<rng::Protocol>();
    let got = out.len();
    let rr = ((*proto).get_rng)(
        proto,
        &rng::ALGORITHM_RAW as *const _ as *mut _,
        got,
        out.as_mut_ptr(),
    );
    if rr != efi::Status::SUCCESS {
        out.fill(0);
    }
}
