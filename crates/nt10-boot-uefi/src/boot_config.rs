//! Optional `EFI/ZirconOSFluent/zbm10.cfg` (ASCII `KEY=value` lines) — avoids proprietary boot databases.

use core::ffi::c_void;
use core::ptr;

use r_efi::efi;
use r_efi::efi::protocols::file;
use r_efi::efi::protocols::loaded_image;
use r_efi::efi::protocols::simple_file_system;

const CFG_MAX: usize = 512;
pub const NAME_MAX: usize = 63;
pub const CHAIN_PATH_MAX: usize = 127;

const DEFAULT_CHAIN: &[u8] = br"EFI\Microsoft\Boot\bootmgfw.efi";

/// Parsed boot policy from `zbm10.cfg`.
#[derive(Clone, Copy, Debug)]
pub struct Zbm10Cfg {
    /// UTF-8 kernel file name within `EFI/ZirconOSFluent/` (e.g. `NT10KRNL.BIN`).
    pub kernel_name: [u8; NAME_MAX],
    pub kernel_name_len: usize,
    /// 0 = disable timed auto-boot; otherwise seconds (e.g. 10).
    pub auto_boot_seconds: u64,
    /// Initial selection index (0..ENTRY_COUNT).
    pub default_entry: u8,
    pub remember_last: bool,
    /// Offer menu row that chain-loads another UEFI loader (e.g. Microsoft Boot Manager).
    pub chainload_enabled: bool,
    /// Backslashes or slashes, relative to ESP root (ASCII).
    pub chainload_path: [u8; CHAIN_PATH_MAX],
    pub chainload_path_len: usize,
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
            auto_boot_seconds: 10,
            default_entry: 0,
            remember_last: true,
            chainload_enabled: true,
            chainload_path: [0; CHAIN_PATH_MAX],
            chainload_path_len: 0,
        }
    }

    pub fn apply_defaults(&mut self) {
        if self.chainload_path_len == 0 {
            let n = DEFAULT_CHAIN.len().min(CHAIN_PATH_MAX);
            self.chainload_path[..n].copy_from_slice(&DEFAULT_CHAIN[..n]);
            self.chainload_path_len = n;
        }
    }

    #[must_use]
    pub fn kernel_name_str(&self) -> Option<&str> {
        if self.kernel_name_len == 0 {
            return None;
        }
        core::str::from_utf8(&self.kernel_name[..self.kernel_name_len]).ok()
    }

    #[must_use]
    #[allow(dead_code)] // Reserved for future UI / diagnostics
    pub fn chainload_path_str(&self) -> Option<&str> {
        if self.chainload_path_len == 0 {
            return None;
        }
        core::str::from_utf8(&self.chainload_path[..self.chainload_path_len]).ok()
    }

    pub fn clamp_default_entry(&self, entry_count: u8) -> u8 {
        let max = entry_count.saturating_sub(1);
        self.default_entry.min(max)
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

unsafe fn open_zircon_dir(
    st: *mut efi::SystemTable,
    image: efi::Handle,
    rw: bool,
) -> Result<*mut file::Protocol, efi::Status> {
    let mode = if rw {
        file::MODE_READ | file::MODE_WRITE
    } else {
        file::MODE_READ
    };
    let li: *mut loaded_image::Protocol = handle_protocol(
        st,
        image,
        &loaded_image::PROTOCOL_GUID as *const _ as *mut _,
    )?;
    let dev = (*li).device_handle;
    if dev.is_null() {
        return Err(efi::Status::NOT_FOUND);
    }
    let sfs: *mut simple_file_system::Protocol = handle_protocol(
        st,
        dev,
        &simple_file_system::PROTOCOL_GUID as *const _ as *mut _,
    )?;

    let mut root: *mut file::Protocol = ptr::null_mut();
    let r = ((*sfs).open_volume)(sfs, &mut root);
    if r != efi::Status::SUCCESS || root.is_null() {
        return Err(r);
    }

    let mut efi_dir: *mut file::Protocol = ptr::null_mut();
    let mut name_efi: [efi::Char16; 4] = [0x0045, 0x0046, 0x0049, 0];
    if ((*root).open)(
        root,
        &mut efi_dir,
        name_efi.as_mut_ptr(),
        mode,
        0,
    ) != efi::Status::SUCCESS
    {
        let _ = ((*root).close)(root);
        return Err(efi::Status::NOT_FOUND);
    }
    let _ = ((*root).close)(root);

    let mut z_dir: *mut file::Protocol = ptr::null_mut();
    let mut name_zircon: [efi::Char16; 15] = [
        0x005a, 0x0069, 0x0072, 0x0063, 0x006f, 0x006e, 0x004f, 0x0053, 0x0046, 0x006c, 0x0075, 0x0065,
        0x006e, 0x0074, 0,
    ];
    if ((*efi_dir).open)(
        efi_dir,
        &mut z_dir,
        name_zircon.as_mut_ptr(),
        mode,
        0,
    ) != efi::Status::SUCCESS
    {
        let _ = ((*efi_dir).close)(efi_dir);
        return Err(efi::Status::NOT_FOUND);
    }
    let _ = ((*efi_dir).close)(efi_dir);

    Ok(z_dir)
}

/// Open `EFI/ZirconOSFluent/zbm10.cfg` from the same volume as the loaded image; ignore if missing.
pub unsafe fn load_zbm10_cfg(st: *mut efi::SystemTable, image: efi::Handle) -> Zbm10Cfg {
    let mut cfg = Zbm10Cfg::new();
    let z_dir = match open_zircon_dir(st, image, false) {
        Ok(d) => d,
        Err(_) => {
            cfg.apply_defaults();
            return cfg;
        }
    };

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
    if r != efi::Status::SUCCESS {
        cfg.apply_defaults();
        return cfg;
    }

    let mut buf = [0u8; CFG_MAX];
    let mut sz = buf.len();
    let rr = ((*cfg_fp).read)(cfg_fp, &mut sz, buf.as_mut_ptr().cast());
    let _ = ((*cfg_fp).close)(cfg_fp);
    if rr != efi::Status::SUCCESS {
        cfg = Zbm10Cfg::new();
        cfg.apply_defaults();
        return cfg;
    }
    parse_cfg(&buf[..sz], &mut cfg);
    cfg.apply_defaults();
    cfg
}

fn parse_ascii_u64(val: &[u8]) -> Option<u64> {
    if val.is_empty() {
        return None;
    }
    let mut n = 0u64;
    for &b in val {
        if !b.is_ascii_digit() {
            return None;
        }
        n = n.saturating_mul(10).saturating_add(u64::from(b - b'0'));
    }
    Some(n)
}

fn parse_ascii_u8(val: &[u8]) -> Option<u8> {
    let n = parse_ascii_u64(val)?;
    if n <= u8::MAX as u64 {
        Some(n as u8)
    } else {
        None
    }
}

fn parse_bool(val: &[u8]) -> Option<bool> {
    if val.eq_ignore_ascii_case(b"1")
        || val.eq_ignore_ascii_case(b"true")
        || val.eq_ignore_ascii_case(b"yes")
        || val.eq_ignore_ascii_case(b"on")
    {
        return Some(true);
    }
    if val.eq_ignore_ascii_case(b"0")
        || val.eq_ignore_ascii_case(b"false")
        || val.eq_ignore_ascii_case(b"no")
        || val.eq_ignore_ascii_case(b"off")
    {
        return Some(false);
    }
    None
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
        } else if key.eq_ignore_ascii_case(b"AUTO_BOOT_SECONDS") {
            if let Some(v) = parse_ascii_u64(val) {
                out.auto_boot_seconds = v.min(3600);
            }
        } else if key.eq_ignore_ascii_case(b"DEFAULT_ENTRY") {
            if let Some(v) = parse_ascii_u8(val) {
                out.default_entry = v;
            }
        } else if key.eq_ignore_ascii_case(b"REMEMBER_LAST") {
            if let Some(b) = parse_bool(val) {
                out.remember_last = b;
            }
        } else if key.eq_ignore_ascii_case(b"CHAINLOAD") {
            if let Some(b) = parse_bool(val) {
                out.chainload_enabled = b;
            }
        } else if key.eq_ignore_ascii_case(b"CHAINLOAD_PATH") {
            let n = val.len().min(CHAIN_PATH_MAX);
            out.chainload_path[..n].copy_from_slice(&val[..n]);
            out.chainload_path_len = n;
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

fn push_bytes(buf: &mut [u8], used: &mut usize, s: &[u8]) -> bool {
    if *used + s.len() > buf.len() {
        return false;
    }
    buf[*used..*used + s.len()].copy_from_slice(s);
    *used += s.len();
    true
}

fn push_line(buf: &mut [u8], used: &mut usize, line: &[u8]) -> bool {
    push_bytes(buf, used, line) && push_bytes(buf, used, b"\r\n")
}

fn fmt_u64(mut n: u64, tmp: &mut [u8; 24]) -> &[u8] {
    if n == 0 {
        tmp[0] = b'0';
        return &tmp[..1];
    }
    let mut i = tmp.len();
    while n > 0 {
        i -= 1;
        tmp[i] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    &tmp[i..]
}

/// Rewrite `zbm10.cfg` on the ESP (delete + create). Best-effort; ignores failures.
pub unsafe fn save_zbm10_cfg(st: *mut efi::SystemTable, image: efi::Handle, cfg: &Zbm10Cfg) {
    let z_dir = match open_zircon_dir(st, image, true) {
        Ok(d) => d,
        Err(_) => return,
    };

    let mut name_cfg: [efi::Char16; 10] = [
        0x007a, 0x0062, 0x006d, 0x0031, 0x0030, 0x002e, 0x0063, 0x0066, 0x0067, 0,
    ];

    let mut existing: *mut file::Protocol = ptr::null_mut();
    let open_existing = ((*z_dir).open)(
        z_dir,
        &mut existing,
        name_cfg.as_mut_ptr(),
        file::MODE_READ | file::MODE_WRITE,
        0,
    );
    if open_existing == efi::Status::SUCCESS && !existing.is_null() {
        let _ = ((*existing).delete)(existing);
    }

    let mut cfg_fp: *mut file::Protocol = ptr::null_mut();
    let r = ((*z_dir).open)(
        z_dir,
        &mut cfg_fp,
        name_cfg.as_mut_ptr(),
        file::MODE_READ | file::MODE_WRITE | file::MODE_CREATE,
        0,
    );
    let _ = ((*z_dir).close)(z_dir);
    if r != efi::Status::SUCCESS || cfg_fp.is_null() {
        return;
    }

    let mut out = [0u8; CFG_MAX];
    let mut u = 0usize;
    let _ = push_line(&mut out, &mut u, b"# Zircon Boot Manager (ZBM10) configuration");
    let _ = push_line(&mut out, &mut u, b"# ASCII KEY=value per line");

    let kn = if cfg.kernel_name_len > 0 {
        &cfg.kernel_name[..cfg.kernel_name_len]
    } else {
        b"NT10KRNL.BIN"
    };
    let _ = push_bytes(&mut out, &mut u, b"kernel=");
    let _ = push_bytes(&mut out, &mut u, kn);
    let _ = push_bytes(&mut out, &mut u, b"\r\n");

    let mut nbuf = [0u8; 24];
    let _ = push_bytes(&mut out, &mut u, b"AUTO_BOOT_SECONDS=");
    let _ = push_bytes(&mut out, &mut u, fmt_u64(cfg.auto_boot_seconds, &mut nbuf));
    let _ = push_bytes(&mut out, &mut u, b"\r\n");

    let _ = push_bytes(&mut out, &mut u, b"DEFAULT_ENTRY=");
    let _ = push_bytes(&mut out, &mut u, fmt_u64(u64::from(cfg.default_entry), &mut nbuf));
    let _ = push_bytes(&mut out, &mut u, b"\r\n");

    let _ = push_line(
        &mut out,
        &mut u,
        if cfg.remember_last {
            b"REMEMBER_LAST=1"
        } else {
            b"REMEMBER_LAST=0"
        },
    );
    let _ = push_line(
        &mut out,
        &mut u,
        if cfg.chainload_enabled {
            b"CHAINLOAD=1"
        } else {
            b"CHAINLOAD=0"
        },
    );

    let _ = push_bytes(&mut out, &mut u, b"CHAINLOAD_PATH=");
    let cp = if cfg.chainload_path_len > 0 {
        &cfg.chainload_path[..cfg.chainload_path_len]
    } else {
        DEFAULT_CHAIN
    };
    let _ = push_bytes(&mut out, &mut u, cp);
    let _ = push_bytes(&mut out, &mut u, b"\r\n");

    let mut wsz = u;
    let wr = ((*cfg_fp).write)(cfg_fp, &mut wsz, out.as_mut_ptr().cast());
    let _ = ((*cfg_fp).close)(cfg_fp);
    let _ = wr;
}
