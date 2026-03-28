//! Optional `EFI/ZirconOS/zbm10.cfg` (ASCII `KEY=value` lines) — avoids proprietary boot databases.

use core::ffi::c_void;
use core::ptr;

use r_efi::efi;
use r_efi::efi::protocols::file;
use r_efi::efi::protocols::loaded_image;
use r_efi::efi::protocols::simple_file_system;

const CFG_MAX: usize = 512;
const NAME_MAX: usize = 63;

/// Parsed boot policy from `zbm10.cfg`.
#[derive(Clone, Copy, Debug)]
pub struct Zbm10Cfg {
    /// UTF-8 kernel file name within `EFI/ZirconOS/` (e.g. `NT10KRNL.BIN`).
    pub kernel_name: [u8; NAME_MAX],
    pub kernel_name_len: usize,
}

impl Default for Zbm10Cfg {
    fn default() -> Self {
        Self::new()
    }
}

impl Zbm10Cfg {
    pub const fn new() -> Self {
        Self {
            kernel_name: [0; NAME_MAX],
            kernel_name_len: 0,
        }
    }

    #[must_use]
    pub fn kernel_name_str(&self) -> Option<&str> {
        if self.kernel_name_len == 0 {
            return None;
        }
        core::str::from_utf8(&self.kernel_name[..self.kernel_name_len]).ok()
    }
}

unsafe fn handle_protocol<T>(
    st: *mut efi::SystemTable,
    handle: efi::Handle,
    guid: *mut efi::Guid,
) -> Result<*mut T, efi::Status> {
    let bs = (*st).boot_services;
    if bs.is_null() {
        return Err(efi::Status::INVALID_PARAMETER);
    }
    let mut iface: *mut c_void = ptr::null_mut();
    let r = ((*bs).handle_protocol)(handle, guid, &mut iface);
    if r != efi::Status::SUCCESS {
        return Err(r);
    }
    Ok(iface.cast())
}

/// Open `EFI/ZirconOS/zbm10.cfg` from the same volume as the loaded image; ignore if missing.
pub unsafe fn load_zbm10_cfg(st: *mut efi::SystemTable, image: efi::Handle) -> Zbm10Cfg {
    let mut cfg = Zbm10Cfg::new();
    let li: *mut loaded_image::Protocol = match handle_protocol(
        st,
        image,
        &loaded_image::PROTOCOL_GUID as *const _ as *mut _,
    ) {
        Ok(p) => p,
        Err(_) => return cfg,
    };
    let dev = (*li).device_handle;
    if dev.is_null() {
        return cfg;
    }
    let sfs: *mut simple_file_system::Protocol = match handle_protocol(
        st,
        dev,
        &simple_file_system::PROTOCOL_GUID as *const _ as *mut _,
    ) {
        Ok(p) => p,
        Err(_) => return cfg,
    };
    let mut root: *mut file::Protocol = ptr::null_mut();
    if ((*sfs).open_volume)(sfs, &mut root) != efi::Status::SUCCESS || root.is_null() {
        return cfg;
    }

    let mut efi_dir: *mut file::Protocol = ptr::null_mut();
    let mut name_efi: [efi::Char16; 4] = [0x0045, 0x0046, 0x0049, 0];
    if ((*root).open)(
        root,
        &mut efi_dir,
        name_efi.as_mut_ptr(),
        file::MODE_READ,
        0,
    ) != efi::Status::SUCCESS
    {
        let _ = ((*root).close)(root);
        return cfg;
    }

    let mut z_dir: *mut file::Protocol = ptr::null_mut();
    let mut name_zircon: [efi::Char16; 9] = [
        0x005a, 0x0069, 0x0072, 0x0063, 0x006f, 0x006e, 0x004f, 0x0053, 0,
    ];
    if ((*efi_dir).open)(
        efi_dir,
        &mut z_dir,
        name_zircon.as_mut_ptr(),
        file::MODE_READ,
        0,
    ) != efi::Status::SUCCESS
    {
        let _ = ((*efi_dir).close)(efi_dir);
        let _ = ((*root).close)(root);
        return cfg;
    }

    let mut cfg_fp: *mut file::Protocol = ptr::null_mut();
    let mut name_cfg: [efi::Char16; 10] = [
        0x007a, 0x0062, 0x006d, 0x0031, 0x0030, 0x002e, 0x0063, 0x0066, 0x0067, 0,
    ];
    let r = ((*z_dir).open)(
        z_dir,
        &mut cfg_fp,
        name_cfg.as_mut_ptr(),
        file::MODE_READ,
        0,
    );
    let _ = ((*z_dir).close)(z_dir);
    let _ = ((*efi_dir).close)(efi_dir);
    let _ = ((*root).close)(root);
    if r != efi::Status::SUCCESS {
        return cfg;
    }

    let mut buf = [0u8; CFG_MAX];
    let mut sz = buf.len();
    let rr = ((*cfg_fp).read)(cfg_fp, &mut sz, buf.as_mut_ptr().cast());
    let _ = ((*cfg_fp).close)(cfg_fp);
    if rr != efi::Status::SUCCESS {
        return cfg;
    }
    parse_cfg(&buf[..sz], &mut cfg);
    cfg
}

fn parse_cfg(data: &[u8], out: &mut Zbm10Cfg) {
    for line in data.split(|b| *b == b'\n' || *b == b'\r') {
        let line = trim_ascii(line);
        if line.is_empty() || line[0] == b'#' {
            continue;
        }
        let Some(eq) = line.iter().position(|c| *c == b'=') else {
            continue;
        };
        let key = trim_ascii(&line[..eq]);
        let val = trim_ascii(&line[eq + 1..]);
        if key.eq_ignore_ascii_case(b"kernel") {
            let n = val.len().min(NAME_MAX);
            out.kernel_name[..n].copy_from_slice(&val[..n]);
            out.kernel_name_len = n;
        }
    }
}

fn trim_ascii(s: &[u8]) -> &[u8] {
    let mut start = 0;
    let mut end = s.len();
    while start < end && matches!(s[start], b' ' | b'\t') {
        start += 1;
    }
    while end > start && matches!(s[end - 1], b' ' | b'\t') {
        end -= 1;
    }
    &s[start..end]
}
