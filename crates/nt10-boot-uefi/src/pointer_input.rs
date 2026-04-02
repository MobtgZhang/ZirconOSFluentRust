//! UEFI Simple Pointer (mouse) and optional Absolute Pointer (touch) — bindings local to this crate.

use core::ffi::c_void;
use core::ptr;

use r_efi::efi;
use r_efi::eficall;
use r_efi::eficall_abi;

// EFI_SIMPLE_POINTER_PROTOCOL_GUID
pub const SIMPLE_POINTER_GUID: efi::Guid = efi::Guid::from_fields(
    0x3187_8c87,
    0x0b7a,
    0x4f03,
    0x9f,
    0x60,
    &[0x5d, 0xe2, 0x3e, 0x32, 0x71, 0x36],
);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SimplePointerMode {
    pub resolution_x: u64,
    pub resolution_y: u64,
    pub resolution_z: u64,
    pub left_button: efi::Boolean,
    pub right_button: efi::Boolean,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct SimplePointerState {
    pub relative_movement_x: i32,
    pub relative_movement_y: i32,
    pub relative_movement_z: i32,
    pub left_button: efi::Boolean,
    pub right_button: efi::Boolean,
}

pub type SimplePointerReset = eficall! {fn(
    this: *mut SimplePointerProtocol,
    extended_verification: bool,
) -> efi::Status};

pub type SimplePointerGetState = eficall! {fn(
    this: *mut SimplePointerProtocol,
    state: *mut SimplePointerState,
) -> efi::Status};

#[repr(C)]
pub struct SimplePointerProtocol {
    pub reset: SimplePointerReset,
    pub get_state: SimplePointerGetState,
    pub wait_for_input: efi::Event,
    pub mode: *mut SimplePointerMode,
}

pub struct PointerAccum {
    pub x: i32,
    pub y: i32,
    pub max_x: i32,
    pub max_y: i32,
}

impl PointerAccum {
    pub fn new(max_x: i32, max_y: i32) -> Self {
        Self {
            x: max_x / 2,
            y: max_y / 2,
            max_x,
            max_y,
        }
    }

    pub fn feed_simple(&mut self, st: &SimplePointerState, _mode: *mut SimplePointerMode) {
        let dx = st.relative_movement_x / 2;
        let dy = st.relative_movement_y / 2;
        self.x = (self.x + dx).clamp(0, self.max_x);
        self.y = (self.y + dy).clamp(0, self.max_y);
    }

}

unsafe fn locate_first_protocol<T>(
    st: *mut efi::SystemTable,
    guid: *mut efi::Guid,
) -> Result<*mut T, efi::Status> {
    let bs = (*st).boot_services;
    if bs.is_null() {
        return Err(efi::Status::INVALID_PARAMETER);
    }
    let mut handles: *mut efi::Handle = ptr::null_mut();
    let mut n: usize = 0;
    let r = ((*bs).locate_handle_buffer)(
        efi::BY_PROTOCOL,
        guid,
        ptr::null_mut(),
        &mut n,
        &mut handles,
    );
    if r != efi::Status::SUCCESS || n == 0 || handles.is_null() {
        if !handles.is_null() {
            let _ = ((*bs).free_pool)(handles.cast());
        }
        return Err(efi::Status::NOT_FOUND);
    }
    let mut out: *mut T = ptr::null_mut();
    for i in 0..n {
        let h = *handles.add(i);
        let mut iface: *mut c_void = ptr::null_mut();
        let hr = ((*bs).handle_protocol)(h, guid, &mut iface);
        if hr == efi::Status::SUCCESS && !iface.is_null() {
            out = iface.cast();
            break;
        }
    }
    let _ = ((*bs).free_pool)(handles.cast());
    if out.is_null() {
        Err(efi::Status::NOT_FOUND)
    } else {
        Ok(out)
    }
}

pub unsafe fn open_simple_pointer(st: *mut efi::SystemTable) -> Option<*mut SimplePointerProtocol> {
    locate_first_protocol(st, &SIMPLE_POINTER_GUID as *const _ as *mut _).ok()
}

/// Absolute pointer (touch) — reuse r_efi definitions.
pub use r_efi::efi::protocols::absolute_pointer;

pub unsafe fn open_absolute_pointer(
    st: *mut efi::SystemTable,
) -> Option<*mut absolute_pointer::Protocol> {
    locate_first_protocol(
        st,
        &absolute_pointer::PROTOCOL_GUID as *const _ as *mut _,
    )
    .ok()
}

pub struct TouchMap {
    pub min_x: u64,
    pub max_x: u64,
    pub min_y: u64,
    pub max_y: u64,
}

impl TouchMap {
    pub fn from_mode(m: &absolute_pointer::Mode) -> Self {
        Self {
            min_x: m.absolute_min_x,
            max_x: m.absolute_max_x,
            min_y: m.absolute_min_y,
            max_y: m.absolute_max_y,
        }
    }

    pub fn to_screen(&self, x: u64, y: u64, sw: i32, sh: i32) -> (i32, i32) {
        let rw = self.max_x.saturating_sub(self.min_x).max(1);
        let rh = self.max_y.saturating_sub(self.min_y).max(1);
        let fx = ((x.saturating_sub(self.min_x)) as i128 * sw as i128 / rw as i128) as i32;
        let fy = ((y.saturating_sub(self.min_y)) as i128 * sh as i128 / rh as i128) as i32;
        (fx.clamp(0, sw), fy.clamp(0, sh))
    }
}
