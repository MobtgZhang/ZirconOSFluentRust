//! UEFI boot / OS selection UI: ConOut menu + optional GOP background, auto-boot countdown.

use core::ptr;

use r_efi::efi;
use r_efi::efi::protocols::graphics_output;
use r_efi::efi::protocols::simple_text_input;

// UEFI scan codes (EDK2-style).
const SCAN_UP: u16 = 0x0001;
const SCAN_DOWN: u16 = 0x0002;
const SCAN_HOME: u16 = 0x0005;
const SCAN_END: u16 = 0x0006;
const SCAN_ESC: u16 = 0x0017;

const ATTR_NORMAL: usize = 0x07;
const ATTR_SEL: usize = 0x1f;
const ATTR_TITLE: usize = 0x0f;

/// Seconds before default entry (ZirconOS NT10) boots without a keypress.
const AUTO_BOOT_SECONDS: u64 = 10;
const POLL_STALL_US: usize = 50_000;

#[derive(Clone, Copy, PartialEq, Eq)]
enum EntryId {
    ZirconNt10,
    Reserved,
    Reboot,
    Shutdown,
}

const ENTRY_COUNT: usize = 4;

#[derive(Clone, Copy)]
struct Entry {
    id: EntryId,
    line: &'static [u16],
}

const L0: &[u16] = &[
    0x0020, 0x0020, 0x0031, 0x002e, 0x0020, 0x005a, 0x0069, 0x0072, 0x0063, 0x006f, 0x006e, 0x004f,
    0x0053, 0x0020, 0x004e, 0x0054, 0x0031, 0x0030, 0x0000,
];
const L1: &[u16] = &[
    0x0020, 0x0020, 0x0032, 0x002e, 0x0020, 0x0052, 0x0065, 0x0073, 0x0065, 0x0072, 0x0076, 0x0065,
    0x0064, 0x0020, 0x0028, 0x006e, 0x006f, 0x0074, 0x0020, 0x0069, 0x006e, 0x0073, 0x0074, 0x0061,
    0x006c, 0x006c, 0x0065, 0x0064, 0x0029, 0x0000,
];
const L2: &[u16] = &[
    0x0020, 0x0020, 0x0033, 0x002e, 0x0020, 0x0052, 0x0065, 0x0062, 0x006f, 0x006f, 0x0074, 0x0020,
    0x0073, 0x0079, 0x0073, 0x0074, 0x0065, 0x006d, 0x0000,
];
const L3: &[u16] = &[
    0x0020, 0x0020, 0x0034, 0x002e, 0x0020, 0x0050, 0x006f, 0x0077, 0x0065, 0x0072, 0x0020, 0x006f,
    0x0066, 0x0066, 0x0000,
];

static ENTRIES: [Entry; ENTRY_COUNT] = [
    Entry {
        id: EntryId::ZirconNt10,
        line: L0,
    },
    Entry {
        id: EntryId::Reserved,
        line: L1,
    },
    Entry {
        id: EntryId::Reboot,
        line: L2,
    },
    Entry {
        id: EntryId::Shutdown,
        line: L3,
    },
];

const TITLE: &[u16] = &[
    0x005a, 0x0042, 0x004d, 0x0031, 0x0030, 0x0020, 0x002d, 0x0020, 0x004f, 0x0053, 0x0020, 0x0073,
    0x0065, 0x006c, 0x0065, 0x0063, 0x0074, 0x0069, 0x006f, 0x006e, 0x000a, 0x0000,
];

const HINT: &[u16] = &[
    0x0055, 0x0070, 0x002f, 0x0044, 0x006f, 0x0077, 0x006e, 0x0020, 0x006d, 0x006f, 0x0076, 0x0065,
    0x002c, 0x0020, 0x0045, 0x006e, 0x0074, 0x0065, 0x0072, 0x0020, 0x0062, 0x006f, 0x006f, 0x0074,
    0x002c, 0x0020, 0x0031, 0x002d, 0x0034, 0x0020, 0x006a, 0x0075, 0x006d, 0x0070, 0x002c, 0x0020,
    0x0042, 0x0020, 0x0069, 0x006d, 0x006d, 0x0065, 0x0064, 0x0069, 0x0061, 0x0074, 0x0065, 0x000a,
    0x0000,
];

const MSG_RESERVED: &[u16] = &[
    0x0052, 0x0065, 0x0073, 0x0065, 0x0072, 0x0076, 0x0065, 0x0064, 0x003a, 0x0020, 0x006e, 0x006f,
    0x0074, 0x0020, 0x0069, 0x006e, 0x0073, 0x0074, 0x0061, 0x006c, 0x006c, 0x0065, 0x0064, 0x000a,
    0x0000,
];

fn con_out(st: *mut efi::SystemTable, s: &[u16]) {
    unsafe {
        let _ = ((*(*st).con_out).output_string)((*st).con_out, s.as_ptr() as *mut efi::Char16);
    }
}

fn con_clear(st: *mut efi::SystemTable) {
    unsafe {
        let _ = ((*(*st).con_out).clear_screen)((*st).con_out);
    }
}

fn con_attr(st: *mut efi::SystemTable, a: usize) {
    unsafe {
        let _ = ((*(*st).con_out).set_attribute)((*st).con_out, a);
    }
}

fn con_cursor(st: *mut efi::SystemTable, col: usize, row: usize) {
    unsafe {
        let _ = ((*(*st).con_out).set_cursor_position)((*st).con_out, col, row);
    }
}

fn stall(st: *mut efi::SystemTable, us: usize) -> Result<(), efi::Status> {
    unsafe {
        let bs = (*st).boot_services;
        if bs.is_null() {
            return Err(efi::Status::INVALID_PARAMETER);
        }
        let r = ((*bs).stall)(us);
        if r != efi::Status::SUCCESS {
            return Err(r);
        }
    }
    Ok(())
}

unsafe fn read_key_nonblocking(
    st: *mut efi::SystemTable,
) -> Result<Option<simple_text_input::InputKey>, efi::Status> {
    let cin = (*st).con_in;
    if cin.is_null() {
        return Err(efi::Status::UNSUPPORTED);
    }
    let mut key = simple_text_input::InputKey {
        scan_code: 0,
        unicode_char: 0,
    };
    let r = ((*cin).read_key_stroke)(cin, &mut key);
    if r == efi::Status::NOT_READY {
        return Ok(None);
    }
    if r != efi::Status::SUCCESS {
        return Err(r);
    }
    Ok(Some(key))
}

fn ascii_kernel_line(kernel: &str, out: &mut [efi::Char16; 96]) -> usize {
    const PREFIX: &[u8] = b"     Kernel: EFI\\ZirconOS\\";
    let mut n = 0usize;
    let kb = kernel.as_bytes();
    if PREFIX.len() + kb.len() + 1 > out.len() {
        return 0;
    }
    for &b in PREFIX {
        out[n] = u16::from(b);
        n += 1;
    }
    for &b in kb {
        if b >= 0x80 {
            return 0;
        }
        out[n] = u16::from(b);
        n += 1;
    }
    out[n] = 0;
    n + 1
}

unsafe fn draw_gop_background(gop: *mut graphics_output::Protocol) {
    if gop.is_null() {
        return;
    }
    let mode_ptr = (*gop).mode;
    if mode_ptr.is_null() {
        return;
    }
    let h = (*mode_ptr).info;
    if h.is_null() {
        return;
    }
    let w = (*h).horizontal_resolution as usize;
    let vh = (*h).vertical_resolution as usize;
    let bar_h = (vh / 12).clamp(48, 120);
    let dark = graphics_output::BltPixel {
        blue: 0x30,
        green: 0x30,
        red: 0x30,
        reserved: 0,
    };
    let accent = graphics_output::BltPixel {
        blue: 0xd0,
        green: 0x60,
        red: 0x25,
        reserved: 0,
    };
    let mut px = dark;
    let _ = ((*gop).blt)(
        gop,
        &mut px,
        graphics_output::BLT_VIDEO_FILL,
        0,
        0,
        0,
        0,
        w,
        vh,
        0,
    );
    px = accent;
    let _ = ((*gop).blt)(
        gop,
        &mut px,
        graphics_output::BLT_VIDEO_FILL,
        0,
        0,
        0,
        0,
        w,
        bar_h,
        0,
    );
}

fn redraw(
    st: *mut efi::SystemTable,
    gop: *mut graphics_output::Protocol,
    sel: usize,
    kernel: &str,
    auto_enabled: bool,
    auto_left_secs: u64,
) {
    unsafe {
        draw_gop_background(gop);
    }
    con_clear(st);
    con_attr(st, ATTR_TITLE);
    con_cursor(st, 0, 0);
    con_out(st, TITLE);
    con_attr(st, ATTR_NORMAL);

    let mut kbuf = [0u16; 96];
    let kl = ascii_kernel_line(kernel, &mut kbuf);
    if kl > 0 {
        con_cursor(st, 0, 1);
        con_out(st, &kbuf[..kl]);
    }

    for i in 0..ENTRY_COUNT {
        con_cursor(st, 0, 3 + i);
        if i == sel {
            con_attr(st, ATTR_SEL);
        } else {
            con_attr(st, ATTR_NORMAL);
        }
        let prefix: &[u16] = if i == sel {
            &[0x003e, 0x0020, 0x0000]
        } else {
            &[0x0020, 0x0020, 0x0000]
        };
        con_out(st, prefix);
        con_out(st, ENTRIES[i].line);
    }
    con_attr(st, ATTR_NORMAL);
    con_cursor(st, 0, 3 + ENTRY_COUNT + 1);
    con_out(st, HINT);

    if auto_enabled {
        con_cursor(st, 0, 3 + ENTRY_COUNT + 3);
        let mut msg = [0u16; 52];
        let mut p = 0usize;
        for &c in b"Auto-boot (1) in " {
            msg[p] = u16::from(c);
            p += 1;
        }
        let s = auto_left_secs.min(99);
        if s >= 10 {
            msg[p] = b'0' as u16 + ((s / 10) as u16);
            p += 1;
        }
        msg[p] = b'0' as u16 + ((s % 10) as u16);
        p += 1;
        for &c in b" s" {
            msg[p] = u16::from(c);
            p += 1;
        }
        msg[p] = 0;
        con_out(st, &msg[..=p]);
    }
}

fn reset_system(st: *mut efi::SystemTable, kind: efi::ResetType) -> ! {
    unsafe {
        let rt = (*st).runtime_services;
        if !rt.is_null() {
            let _ = ((*rt).reset_system)(kind, efi::Status::SUCCESS, 0, ptr::null_mut());
        }
    }
    loop {
        core::hint::spin_loop();
    }
}

/// Show OS selection; returns `Ok(())` to continue loading the primary kernel from `EFI\\ZirconOS\\`.
///
/// # Safety
/// `st` is the firmware system table; valid while Boot Services are active.
pub unsafe fn run_boot_menu(
    st: *mut efi::SystemTable,
    gop: *mut graphics_output::Protocol,
    kernel_path: &str,
) -> Result<(), efi::Status> {
    if (*st).con_in.is_null() {
        return Ok(());
    }

    let mut sel = 0usize;
    let mut auto_enabled = true;
    let mut auto_elapsed_us: u64 = 0;
    let auto_limit_us: u64 = AUTO_BOOT_SECONDS * 1_000_000;
    let mut last_drawn_left: u64 = AUTO_BOOT_SECONDS;

    redraw(st, gop, sel, kernel_path, true, last_drawn_left);

    loop {
        match read_key_nonblocking(st) {
            Ok(Some(k)) => {
                auto_enabled = false;
                if k.scan_code == SCAN_UP {
                    if sel > 0 {
                        sel -= 1;
                    }
                    redraw(st, gop, sel, kernel_path, false, 0);
                } else if k.scan_code == SCAN_DOWN {
                    if sel + 1 < ENTRY_COUNT {
                        sel += 1;
                    }
                    redraw(st, gop, sel, kernel_path, false, 0);
                } else if k.scan_code == SCAN_HOME {
                    sel = 0;
                    redraw(st, gop, sel, kernel_path, false, 0);
                } else if k.scan_code == SCAN_END {
                    sel = ENTRY_COUNT - 1;
                    redraw(st, gop, sel, kernel_path, false, 0);
                } else if k.scan_code == SCAN_ESC {
                    sel = ENTRY_COUNT - 1;
                    redraw(st, gop, sel, kernel_path, false, 0);
                } else if k.unicode_char == 0x000d || k.unicode_char == 0x000a {
                    match ENTRIES[sel].id {
                        EntryId::ZirconNt10 => return Ok(()),
                        EntryId::Reserved => {
                            con_cursor(st, 0, 3 + ENTRY_COUNT + 5);
                            con_out(st, MSG_RESERVED);
                            stall(st, 1_500_000)?;
                            redraw(st, gop, sel, kernel_path, auto_enabled, last_drawn_left);
                        }
                        EntryId::Reboot => reset_system(st, efi::RESET_COLD),
                        EntryId::Shutdown => reset_system(st, efi::RESET_SHUTDOWN),
                    }
                } else {
                    let u = k.unicode_char;
                    if u == u16::from(b'1') {
                        sel = 0;
                        redraw(st, gop, sel, kernel_path, false, 0);
                    } else if u == u16::from(b'2') {
                        sel = 1;
                        redraw(st, gop, sel, kernel_path, false, 0);
                    } else if u == u16::from(b'3') {
                        sel = 2;
                        redraw(st, gop, sel, kernel_path, false, 0);
                    } else if u == u16::from(b'4') {
                        sel = 3;
                        redraw(st, gop, sel, kernel_path, false, 0);
                    } else if u == u16::from(b'b') || u == u16::from(b'B') {
                        return Ok(());
                    }
                }
            }
            Ok(None) => {}
            Err(e) => return Err(e),
        }

        if auto_enabled {
            auto_elapsed_us += POLL_STALL_US as u64;
            if auto_elapsed_us >= auto_limit_us {
                return Ok(());
            }
            let left = (auto_limit_us - auto_elapsed_us).div_ceil(1_000_000);
            if left != last_drawn_left {
                last_drawn_left = left;
                redraw(st, gop, sel, kernel_path, true, left);
            }
        }

        stall(st, POLL_STALL_US)?;
    }
}
