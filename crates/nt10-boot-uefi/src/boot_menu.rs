//! UEFI boot selection: GOP tile UI (pointer + keyboard) or ConOut fallback (Win7-style text).

use core::ptr;

use r_efi::efi;
use r_efi::efi::protocols::graphics_output;
use r_efi::efi::protocols::simple_text_input;

use crate::boot_config::{load_zbm10_cfg, save_zbm10_cfg, Zbm10Cfg};
use crate::boot_nv;
use crate::boot_ui_gfx::{self, CursorOverlay};
use crate::chainload;
use crate::pointer_input::{
    absolute_pointer, open_absolute_pointer, open_simple_pointer, PointerAccum, SimplePointerState,
    TouchMap,
};

// UEFI scan codes (EDK2-style).
const SCAN_UP: u16 = 0x0001;
const SCAN_DOWN: u16 = 0x0002;
const SCAN_HOME: u16 = 0x0005;
const SCAN_END: u16 = 0x0006;
const SCAN_ESC: u16 = 0x0017;
/// EDK2 `SCAN_F10`.
const SCAN_F10: u16 = 0x0012;

const ATTR_NORMAL: usize = 0x07;
const ATTR_SEL: usize = 0x1f;
const ATTR_TITLE: usize = 0x9f;
const ATTR_HINT: usize = 0x70;

const POLL_STALL_US: usize = 50_000;
const POLL_STALL_IDLE_US: usize = 100_000;

#[derive(Clone, Copy, PartialEq, Eq)]
struct GfxMenuSnapshot {
    focus: Focus,
    auto_left: u64,
    auto_enabled: bool,
    chain_muted: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct TextMenuSnapshot {
    sel: usize,
    auto_left: u64,
    auto_enabled: bool,
    chain_muted: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum EntryId {
    FluentNt10,
    Reserved,
    ChainBootMgr,
    Reboot,
    Shutdown,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Focus {
    Entry(usize),
    Footer,
}

#[derive(Clone, Copy)]
struct Entry {
    id: EntryId,
    line: &'static [u16],
    label_gfx: &'static [u8],
}

const L0: &[u16] = &[
    0x0020, 0x0020, 0x0031, 0x002e, 0x0020, 0x005a, 0x0069, 0x0072, 0x0063, 0x006f, 0x006e, 0x004f,
    0x0053, 0x0046, 0x006c, 0x0075, 0x0065, 0x006e, 0x0074, 0x0020, 0x004e, 0x0054, 0x0031, 0x0030,
    0x0000,
];
const L1: &[u16] = &[
    0x0020, 0x0020, 0x0032, 0x002e, 0x0020, 0x0052, 0x0065, 0x0073, 0x0065, 0x0072, 0x0076, 0x0065,
    0x0064, 0x0020, 0x0028, 0x006e, 0x006f, 0x0074, 0x0020, 0x0069, 0x006e, 0x0073, 0x0074, 0x0061,
    0x006c, 0x006c, 0x0065, 0x0064, 0x0029, 0x0000,
];
const L2: &[u16] = &[
    0x0020, 0x0020, 0x0033, 0x002e, 0x0020, 0x004f, 0x0074, 0x0068, 0x0065, 0x0072, 0x0020, 0x004f,
    0x0053, 0x0020, 0x0062, 0x006f, 0x006f, 0x0074, 0x0020, 0x006d, 0x0061, 0x006e, 0x0061, 0x0067,
    0x0065, 0x0072, 0x0000,
];
const L3: &[u16] = &[
    0x0020, 0x0020, 0x0034, 0x002e, 0x0020, 0x0052, 0x0065, 0x0073, 0x0074, 0x0061, 0x0072, 0x0074,
    0x0020, 0x0073, 0x0079, 0x0073, 0x0074, 0x0065, 0x006d, 0x0000,
];
const L4: &[u16] = &[
    0x0020, 0x0020, 0x0035, 0x002e, 0x0020, 0x0050, 0x006f, 0x0077, 0x0065, 0x0072, 0x0020, 0x006f,
    0x0066, 0x0066, 0x0000,
];

static ENTRIES: [Entry; boot_ui_gfx::ENTRY_COUNT] = [
    Entry {
        id: EntryId::FluentNt10,
        line: L0,
        label_gfx: b"ZirconOSFluent NT10",
    },
    Entry {
        id: EntryId::Reserved,
        line: L1,
        label_gfx: b"Reserved (not installed)",
    },
    Entry {
        id: EntryId::ChainBootMgr,
        line: L2,
        label_gfx: b"Other OS boot manager",
    },
    Entry {
        id: EntryId::Reboot,
        line: L3,
        label_gfx: b"Restart system",
    },
    Entry {
        id: EntryId::Shutdown,
        line: L4,
        label_gfx: b"Power off",
    },
];

const TITLE_TXT: &[u16] = &[
    0x005a, 0x0069, 0x0072, 0x0063, 0x006f, 0x006e, 0x0020, 0x0042, 0x006f, 0x006f, 0x0074, 0x0020,
    0x004d, 0x0061, 0x006e, 0x0061, 0x0067, 0x0065, 0x0072, 0x0020, 0x002d, 0x0020, 0x0043, 0x0068,
    0x006f, 0x006f, 0x0073, 0x0065, 0x0020, 0x0061, 0x0020, 0x0073, 0x0074, 0x0061, 0x0072, 0x0074,
    0x0075, 0x0070, 0x0020, 0x006f, 0x0070, 0x0074, 0x0069, 0x006f, 0x006e, 0x000a, 0x0000,
];
const HINT_TXT: &[u16] = &[
    0x0055, 0x0070, 0x002f, 0x0044, 0x006f, 0x0077, 0x006e, 0x002f, 0x0054, 0x0061, 0x0062, 0x0020,
    0x006d, 0x006f, 0x0076, 0x0065, 0x0020, 0x0020, 0x0045, 0x006e, 0x0074, 0x0065, 0x0072, 0x0020,
    0x0073, 0x0065, 0x006c, 0x0065, 0x0063, 0x0074, 0x0020, 0x0020, 0x0043, 0x0020, 0x0064, 0x0065,
    0x0066, 0x0061, 0x0075, 0x006c, 0x0074, 0x0073, 0x0020, 0x0020, 0x0042, 0x0020, 0x0062, 0x006f,
    0x006f, 0x0074, 0x0020, 0x004e, 0x0054, 0x0031, 0x0030, 0x0020, 0x006e, 0x006f, 0x0077, 0x000a,
    0x0000,
];
const FOOTER_TXT: &[u16] = &[
    0x0043, 0x0068, 0x0061, 0x006e, 0x0067, 0x0065, 0x0020, 0x0064, 0x0065, 0x0066, 0x0061, 0x0075,
    0x006c, 0x0074, 0x0073, 0x003a, 0x0020, 0x0070, 0x0072, 0x0065, 0x0073, 0x0073, 0x0020, 0x0043,
    0x0020, 0x006f, 0x0072, 0x0020, 0x0046, 0x0031, 0x0030, 0x000a, 0x0000,
];

const MSG_RESERVED: &[u16] = &[
    0x0052, 0x0065, 0x0073, 0x0065, 0x0072, 0x0076, 0x0065, 0x0064, 0x003a, 0x0020, 0x006e, 0x006f,
    0x0074, 0x0020, 0x0069, 0x006e, 0x0073, 0x0074, 0x0061, 0x006c, 0x006c, 0x0065, 0x0064, 0x000a,
    0x0000,
];
const MSG_CHAIN_OFF: &[u16] = &[
    0x004f, 0x0074, 0x0068, 0x0065, 0x0072, 0x0020, 0x004f, 0x0053, 0x0020, 0x0062, 0x006f, 0x006f,
    0x0074, 0x0020, 0x006d, 0x0061, 0x006e, 0x0061, 0x0067, 0x0065, 0x0072, 0x0020, 0x0064, 0x0069,
    0x0073, 0x0061, 0x0062, 0x006c, 0x0065, 0x0064, 0x0020, 0x0069, 0x006e, 0x0020, 0x007a, 0x0062,
    0x006d, 0x0031, 0x0030, 0x002e, 0x0063, 0x0066, 0x0067, 0x000a, 0x0000,
];
const MSG_CHAIN_ERR: &[u16] = &[
    0x0043, 0x0068, 0x0061, 0x0069, 0x006e, 0x0020, 0x006c, 0x006f, 0x0061, 0x0064, 0x0020, 0x0066,
    0x0061, 0x0069, 0x006c, 0x0065, 0x0064, 0x0020, 0x0028, 0x0073, 0x0065, 0x0065, 0x0020, 0x0073,
    0x0074, 0x0061, 0x0074, 0x0075, 0x0073, 0x0029, 0x000a, 0x0000,
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
    const PREFIX: &[u8] = b"     Kernel: EFI\\ZirconOSFluent\\";
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

unsafe fn gop_dims(gop: *mut graphics_output::Protocol) -> Option<(usize, usize)> {
    if gop.is_null() {
        return None;
    }
    let mode_ptr = (*gop).mode;
    if mode_ptr.is_null() {
        return None;
    }
    let h = (*mode_ptr).info;
    if h.is_null() {
        return None;
    }
    Some((
        (*h).horizontal_resolution as usize,
        (*h).vertical_resolution as usize,
    ))
}

unsafe fn paint_gfx_menu(
    gop: *mut graphics_output::Protocol,
    layout: &boot_ui_gfx::TileLayout,
    focus: Focus,
    cfg: &Zbm10Cfg,
    auto_enabled: bool,
    auto_left: u64,
    labels: [&'static [u8]; boot_ui_gfx::ENTRY_COUNT],
) {
    let (focused_entry, footer_selected) = match focus {
        Focus::Entry(i) => (Some(i), false),
        Focus::Footer => (None, true),
    };
    boot_ui_gfx::paint_main_menu_content(
        gop,
        layout,
        focused_entry,
        footer_selected,
        !cfg.chainload_enabled,
        auto_left,
        auto_enabled && cfg.auto_boot_seconds > 0,
        labels,
    );
}

unsafe fn hit_test(layout: &boot_ui_gfx::TileLayout, x: i32, y: i32) -> Option<Focus> {
    if layout.footer.contains(x, y) {
        return Some(Focus::Footer);
    }
    for i in 0..boot_ui_gfx::ENTRY_COUNT {
        if layout.tiles[i].contains(x, y) {
            return Some(Focus::Entry(i));
        }
    }
    None
}

fn initial_selection_fixed(st: *mut efi::SystemTable, cfg: &Zbm10Cfg) -> Focus {
    let def = cfg.clamp_default_entry(boot_ui_gfx::ENTRY_COUNT as u8) as usize;
    let idx = if cfg.remember_last {
        unsafe { boot_nv::read_last_entry(st) }
            .map(|v| (v as usize).min(boot_ui_gfx::ENTRY_COUNT - 1))
            .unwrap_or(def)
    } else {
        def
    };
    let idx = idx.min(boot_ui_gfx::ENTRY_COUNT - 1);
    Focus::Entry(idx)
}

unsafe fn run_gfx_menu(
    st: *mut efi::SystemTable,
    image: efi::Handle,
    gop: *mut graphics_output::Protocol,
    _kernel_path: &str,
    cfg: &mut Zbm10Cfg,
) -> Result<(), efi::Status> {
    let Some((fw, fh)) = gop_dims(gop) else {
        return Err(efi::Status::NOT_READY);
    };
    let layout = boot_ui_gfx::layout(fw, fh);
    let mut focus = initial_selection_fixed(st, cfg);
    let mut auto_enabled = cfg.auto_boot_seconds > 0;
    let mut auto_elapsed_us: u64 = 0;
    let auto_limit_us: u64 = cfg.auto_boot_seconds.saturating_mul(1_000_000);
    let mut last_drawn_left: u64 = cfg.auto_boot_seconds;
    let mut accum = PointerAccum::new(fw as i32 - 1, fh as i32 - 1);
    let simple = open_simple_pointer(st);
    let abs = open_absolute_pointer(st);
    let touch_map = if let Some(ap) = abs {
        let m = (*ap).mode;
        if m.is_null() {
            None
        } else {
            Some(TouchMap::from_mode(&*m))
        }
    } else {
        None
    };

    let mut simple_was_down = false;
    let mut touch_was_down = false;
    let has_pointer = simple.is_some() || abs.is_some();
    let mut cursor_ov = CursorOverlay::new();
    let labels = [
        ENTRIES[0].label_gfx,
        ENTRIES[1].label_gfx,
        ENTRIES[2].label_gfx,
        ENTRIES[3].label_gfx,
        ENTRIES[4].label_gfx,
    ];
    let mut prev_snap: Option<GfxMenuSnapshot> = None;
    let mut force_menu_redraw = false;
    let mut prev_ax = accum.x;
    let mut prev_ay = accum.y;
    let mut prev_stall_us: u64 = 0;

    loop {
        if auto_enabled && cfg.auto_boot_seconds > 0 && prev_stall_us > 0 {
            auto_elapsed_us += prev_stall_us;
            if auto_elapsed_us >= auto_limit_us {
                persist_choice(st, cfg, 0);
                return Ok(());
            }
            let left = (auto_limit_us - auto_elapsed_us).div_ceil(1_000_000);
            if left != last_drawn_left {
                last_drawn_left = left;
            }
        }

        if let Some(sp) = simple {
            let mut pst = SimplePointerState::default();
            let r = ((*sp).get_state)(sp, &mut pst);
            if r == efi::Status::SUCCESS {
                let mode = (*sp).mode;
                accum.feed_simple(&pst, mode);
                let down = bool::from(pst.left_button);
                if down {
                    if let Some(h) = hit_test(&layout, accum.x, accum.y) {
                        focus = h;
                    }
                } else if simple_was_down {
                    if let Some(h) = hit_test(&layout, accum.x, accum.y) {
                        focus = h;
                        let mut menu_repaint = false;
                        let res = activate_focus(
                            st,
                            image,
                            gop,
                            &layout,
                            focus,
                            cfg,
                            fw,
                            fh,
                            &mut auto_enabled,
                            &mut last_drawn_left,
                            cfg.auto_boot_seconds,
                            &mut menu_repaint,
                        )?;
                        if menu_repaint {
                            force_menu_redraw = true;
                        }
                        if res.is_some() {
                            return Ok(());
                        }
                    }
                }
                simple_was_down = down;
            }
        }
        if let (Some(ap), Some(ref tm)) = (abs, touch_map.as_ref()) {
            let mut st_abs = absolute_pointer::State::default();
            let r = ((*ap).get_state)(ap, &mut st_abs);
            if r == efi::Status::SUCCESS {
                let down = (st_abs.active_buttons & absolute_pointer::TOUCH_ACTIVE) != 0;
                if down {
                    let (x, y) =
                        tm.to_screen(st_abs.current_x, st_abs.current_y, accum.max_x, accum.max_y);
                    accum.x = x;
                    accum.y = y;
                    if let Some(h) = hit_test(&layout, accum.x, accum.y) {
                        focus = h;
                    }
                } else if touch_was_down {
                    if let Some(h) = hit_test(&layout, accum.x, accum.y) {
                        focus = h;
                        let mut menu_repaint = false;
                        let res = activate_focus(
                            st,
                            image,
                            gop,
                            &layout,
                            focus,
                            cfg,
                            fw,
                            fh,
                            &mut auto_enabled,
                            &mut last_drawn_left,
                            cfg.auto_boot_seconds,
                            &mut menu_repaint,
                        )?;
                        if menu_repaint {
                            force_menu_redraw = true;
                        }
                        if res.is_some() {
                            return Ok(());
                        }
                    }
                }
                touch_was_down = down;
            }
        }

        match read_key_nonblocking(st) {
            Ok(Some(k)) => {
                auto_enabled = false;
                if k.scan_code == SCAN_UP {
                    focus = match focus {
                        Focus::Entry(i) if i > 0 => Focus::Entry(i - 1),
                        Focus::Footer => Focus::Entry(boot_ui_gfx::ENTRY_COUNT - 1),
                        _ => focus,
                    };
                } else if k.scan_code == SCAN_DOWN {
                    focus = match focus {
                        Focus::Entry(i) if i + 1 < boot_ui_gfx::ENTRY_COUNT => Focus::Entry(i + 1),
                        Focus::Entry(_) => Focus::Footer,
                        _ => focus,
                    };
                } else if k.scan_code == SCAN_HOME {
                    focus = Focus::Entry(0);
                } else if k.scan_code == SCAN_END {
                    focus = Focus::Footer;
                } else if k.unicode_char == b'\t' as u16 {
                    focus = match focus {
                        Focus::Entry(i) => {
                            if i + 1 < boot_ui_gfx::ENTRY_COUNT {
                                Focus::Entry(i + 1)
                            } else {
                                Focus::Footer
                            }
                        }
                        Focus::Footer => Focus::Entry(0),
                    };
                } else if k.unicode_char == 0x000d || k.unicode_char == 0x000a {
                    let mut menu_repaint = false;
                    let res = activate_focus(
                        st,
                        image,
                        gop,
                        &layout,
                        focus,
                        cfg,
                        fw,
                        fh,
                        &mut auto_enabled,
                        &mut last_drawn_left,
                        cfg.auto_boot_seconds,
                        &mut menu_repaint,
                    )?;
                    if menu_repaint {
                        force_menu_redraw = true;
                    }
                    if res.is_some() {
                        return Ok(());
                    }
                } else if k.scan_code == SCAN_ESC {
                    focus = Focus::Footer;
                } else if k.unicode_char == b'c' as u16 || k.unicode_char == b'C' as u16 {
                    let _ = run_settings(st, image, gop, cfg, fw, fh)?;
                    focus = initial_selection_fixed(st, cfg);
                    force_menu_redraw = true;
                } else if k.scan_code == SCAN_F10 {
                    let _ = run_settings(st, image, gop, cfg, fw, fh)?;
                    focus = initial_selection_fixed(st, cfg);
                    force_menu_redraw = true;
                } else {
                    let u = k.unicode_char;
                    if u == u16::from(b'1') {
                        focus = Focus::Entry(0);
                    } else if u == u16::from(b'2') {
                        focus = Focus::Entry(1);
                    } else if u == u16::from(b'3') {
                        focus = Focus::Entry(2);
                    } else if u == u16::from(b'4') {
                        focus = Focus::Entry(3);
                    } else if u == u16::from(b'5') {
                        focus = Focus::Entry(4);
                    } else if u == u16::from(b'b') || u == u16::from(b'B') {
                        persist_choice(st, cfg, 0);
                        return Ok(());
                    }
                }
            }
            Ok(None) => {}
            Err(e) => return Err(e),
        }

        let snap = GfxMenuSnapshot {
            focus,
            auto_left: last_drawn_left,
            auto_enabled,
            chain_muted: !cfg.chainload_enabled,
        };
        let content_dirty =
            force_menu_redraw || prev_snap.map(|p| p != snap).unwrap_or(true);
        let cursor_moved = has_pointer && (accum.x != prev_ax || accum.y != prev_ay);

        if content_dirty {
            paint_gfx_menu(
                gop,
                &layout,
                focus,
                cfg,
                auto_enabled,
                last_drawn_left,
                labels,
            );
            if has_pointer {
                cursor_ov.place_after_full_paint(gop, accum.x, accum.y, fw, fh);
            }
            prev_ax = accum.x;
            prev_ay = accum.y;
            force_menu_redraw = false;
        } else if has_pointer && cursor_moved {
            cursor_ov.update(gop, accum.x, accum.y, fw, fh);
            prev_ax = accum.x;
            prev_ay = accum.y;
        }

        prev_snap = Some(snap);

        let stall_us = if has_pointer || content_dirty || cursor_moved {
            POLL_STALL_US
        } else {
            POLL_STALL_IDLE_US
        };
        prev_stall_us = stall_us as u64;
        stall(st, stall_us)?;
    }
}

fn persist_choice(st: *mut efi::SystemTable, cfg: &Zbm10Cfg, entry_idx: u8) {
    if cfg.remember_last {
        unsafe { boot_nv::write_last_entry(st, entry_idx) };
    }
}

unsafe fn activate_focus(
    st: *mut efi::SystemTable,
    image: efi::Handle,
    gop: *mut graphics_output::Protocol,
    _layout: &boot_ui_gfx::TileLayout,
    focus: Focus,
    cfg: &mut Zbm10Cfg,
    fw: usize,
    fh: usize,
    auto_enabled: &mut bool,
    last_drawn_left: &mut u64,
    auto_secs: u64,
    menu_repaint: &mut bool,
) -> Result<Option<()>, efi::Status> {
    match focus {
        Focus::Footer => {
            let _ = run_settings(st, image, gop, cfg, fw, fh)?;
            *auto_enabled = cfg.auto_boot_seconds > 0;
            *last_drawn_left = cfg.auto_boot_seconds;
            *menu_repaint = true;
            Ok(None)
        }
        Focus::Entry(i) => match ENTRIES[i].id {
            EntryId::FluentNt10 => {
                persist_choice(st, cfg, i as u8);
                Ok(Some(()))
            }
            EntryId::Reserved => {
                con_clear(st);
                con_attr(st, ATTR_NORMAL);
                con_out(st, MSG_RESERVED);
                stall(st, 1_500_000)?;
                *menu_repaint = true;
                Ok(None)
            }
            EntryId::ChainBootMgr => {
                if !cfg.chainload_enabled {
                    con_clear(st);
                    con_attr(st, ATTR_NORMAL);
                    con_out(st, MSG_CHAIN_OFF);
                    stall(st, 1_500_000)?;
                    *menu_repaint = true;
                    return Ok(None);
                }
                let path = if cfg.chainload_path_len > 0 {
                    &cfg.chainload_path[..cfg.chainload_path_len]
                } else {
                    b"EFI\\Microsoft\\Boot\\bootmgfw.efi"
                };
                let r = chainload::chainload_efi_path(st, image, path);
                if r != efi::Status::SUCCESS {
                    con_clear(st);
                    con_attr(st, ATTR_NORMAL);
                    con_out(st, MSG_CHAIN_ERR);
                    stall(st, 2_000_000)?;
                    *menu_repaint = true;
                }
                *auto_enabled = cfg.auto_boot_seconds > 0;
                *last_drawn_left = auto_secs;
                Ok(None)
            }
            EntryId::Reboot => reset_system(st, efi::RESET_COLD),
            EntryId::Shutdown => reset_system(st, efi::RESET_SHUTDOWN),
        },
    }
}

/// Settings UI. Returns true if cfg may have changed (caller should refresh selection).
unsafe fn run_settings(
    st: *mut efi::SystemTable,
    image: efi::Handle,
    gop: *mut graphics_output::Protocol,
    cfg: &mut Zbm10Cfg,
    fw: usize,
    fh: usize,
) -> Result<bool, efi::Status> {
    let mut line: usize = 0;
    const LINES: usize = 5;
    let mut needs_paint = true;
    let _ = (fw, fh);
    loop {
        match read_key_nonblocking(st)? {
            Some(k) => {
                if k.scan_code == SCAN_ESC {
                    *cfg = load_zbm10_cfg(st, image);
                    return Ok(false);
                }
                if k.scan_code == SCAN_UP && line > 0 {
                    line -= 1;
                    needs_paint = true;
                } else if k.scan_code == SCAN_DOWN && line + 1 < LINES {
                    line += 1;
                    needs_paint = true;
                } else if k.unicode_char == b'+' as u16
                    || k.unicode_char == b'=' as u16
                    || k.unicode_char == u16::from(b']')
                {
                    adjust_setting(cfg, line, true);
                    needs_paint = true;
                } else if k.unicode_char == b'-' as u16 || k.unicode_char == u16::from(b'[') {
                    adjust_setting(cfg, line, false);
                    needs_paint = true;
                } else if k.unicode_char == 0x000d || k.unicode_char == 0x000a {
                    if line == 4 {
                        save_zbm10_cfg(st, image, cfg);
                        return Ok(true);
                    }
                }
            }
            None => {}
        }

        if needs_paint {
            boot_ui_gfx::fill_screen(gop, boot_ui_gfx::COL_BG);
            let title =
                b"Change defaults  (Up/Down  move   +/- adjust   Enter save & exit   Esc cancel)";
            boot_ui_gfx::draw_ascii(gop, 32, 40, title, 1, boot_ui_gfx::COL_WHITE);
            let y0 = 80usize;
            let dy = 28usize;
            draw_setting_line(gop, 40, y0, line == 0, b"AUTO_BOOT_SECONDS", cfg.auto_boot_seconds);
            draw_setting_line(
                gop,
                40,
                y0 + dy,
                line == 1,
                b"DEFAULT_ENTRY",
                u64::from(cfg.default_entry),
            );
            draw_setting_line(
                gop,
                40,
                y0 + dy * 2,
                line == 2,
                b"REMEMBER_LAST",
                if cfg.remember_last { 1 } else { 0 },
            );
            draw_setting_line(
                gop,
                40,
                y0 + dy * 3,
                line == 3,
                b"CHAINLOAD",
                if cfg.chainload_enabled { 1 } else { 0 },
            );
            boot_ui_gfx::draw_ascii(
                gop,
                40,
                y0 + dy * 4,
                if line == 4 {
                    b"> Save and return"
                } else {
                    b"  Save and return"
                },
                2,
                boot_ui_gfx::COL_WHITE,
            );
            needs_paint = false;
            stall(st, POLL_STALL_US)?;
        } else {
            stall(st, POLL_STALL_IDLE_US)?;
        }
    }
}

unsafe fn draw_setting_line(
    gop: *mut graphics_output::Protocol,
    x: usize,
    y: usize,
    sel: bool,
    key: &[u8],
    val: u64,
) {
    let mut buf = [0u8; 80];
    let mut n = 0usize;
    if sel {
        buf[n] = b'>';
        n += 1;
        buf[n] = b' ';
        n += 1;
    } else {
        buf[n] = b' ';
        n += 1;
        buf[n] = b' ';
        n += 1;
    }
    for &b in key {
        if n + 1 >= buf.len() {
            break;
        }
        buf[n] = b;
        n += 1;
    }
    for &b in b" = " {
        buf[n] = b;
        n += 1;
    }
    let mut tmp = [0u8; 20];
    let vs = fmt_u64_to_buf(val, &mut tmp);
    for &b in vs {
        buf[n] = b;
        n += 1;
    }
    boot_ui_gfx::draw_ascii(gop, x, y, &buf[..n], 2, boot_ui_gfx::COL_WHITE);
}

fn fmt_u64_to_buf(mut v: u64, tmp: &mut [u8; 20]) -> &[u8] {
    if v == 0 {
        tmp[0] = b'0';
        return &tmp[..1];
    }
    let mut i = tmp.len();
    while v > 0 {
        i -= 1;
        tmp[i] = b'0' + (v % 10) as u8;
        v /= 10;
    }
    &tmp[i..]
}

fn adjust_setting(cfg: &mut Zbm10Cfg, line: usize, inc: bool) {
    match line {
        0 => {
            if inc {
                cfg.auto_boot_seconds = (cfg.auto_boot_seconds + 5).min(3600);
            } else {
                cfg.auto_boot_seconds = cfg.auto_boot_seconds.saturating_sub(5);
            }
        }
        1 => {
            let max = boot_ui_gfx::ENTRY_COUNT as u8 - 1;
            if inc {
                cfg.default_entry = (cfg.default_entry + 1).min(max);
            } else {
                cfg.default_entry = cfg.default_entry.saturating_sub(1);
            }
        }
        2 => cfg.remember_last = !cfg.remember_last,
        3 => cfg.chainload_enabled = !cfg.chainload_enabled,
        _ => {}
    }
}

unsafe fn redraw_text(
    st: *mut efi::SystemTable,
    gop: *mut graphics_output::Protocol,
    sel: usize,
    kernel: &str,
    cfg: &Zbm10Cfg,
    auto_enabled: bool,
    auto_left: u64,
) {
    boot_ui_gfx::fill_screen(gop, boot_ui_gfx::COL_BG);
    con_clear(st);
    con_attr(st, ATTR_TITLE);
    con_cursor(st, 0, 0);
    con_out(st, TITLE_TXT);
    con_attr(st, ATTR_NORMAL);
    let mut kbuf = [0u16; 96];
    let kl = ascii_kernel_line(kernel, &mut kbuf);
    if kl > 0 {
        con_cursor(st, 0, 1);
        con_out(st, &kbuf[..kl]);
    }
    for i in 0..boot_ui_gfx::ENTRY_COUNT {
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
    con_attr(st, ATTR_HINT);
    con_cursor(st, 0, 3 + boot_ui_gfx::ENTRY_COUNT + 1);
    con_out(st, HINT_TXT);
    con_cursor(st, 0, 3 + boot_ui_gfx::ENTRY_COUNT + 3);
    con_out(st, FOOTER_TXT);
    con_attr(st, ATTR_NORMAL);
    if auto_enabled && cfg.auto_boot_seconds > 0 {
        con_cursor(st, 0, 3 + boot_ui_gfx::ENTRY_COUNT + 5);
        let mut msg = [0u16; 56];
        let mut p = 0usize;
        for &c in b"Auto-boot in " {
            msg[p] = u16::from(c);
            p += 1;
        }
        let s = auto_left.min(99);
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

unsafe fn run_text_menu(
    st: *mut efi::SystemTable,
    image: efi::Handle,
    gop: *mut graphics_output::Protocol,
    kernel_path: &str,
    cfg: &mut Zbm10Cfg,
) -> Result<(), efi::Status> {
    if (*st).con_in.is_null() {
        return Ok(());
    }

    let mut sel = match initial_selection_fixed(st, cfg) {
        Focus::Entry(i) => i,
        Focus::Footer => 0,
    };
    let mut auto_enabled = cfg.auto_boot_seconds > 0;
    let mut auto_elapsed_us: u64 = 0;
    let auto_limit_us: u64 = cfg.auto_boot_seconds.saturating_mul(1_000_000);
    let mut last_drawn_left: u64 = cfg.auto_boot_seconds;
    let mut prev_text: Option<TextMenuSnapshot> = None;
    let mut force_text_redraw = false;
    let mut prev_stall_us: u64 = 0;

    loop {
        if auto_enabled && cfg.auto_boot_seconds > 0 && prev_stall_us > 0 {
            auto_elapsed_us += prev_stall_us;
            if auto_elapsed_us >= auto_limit_us {
                persist_choice(st, cfg, 0);
                return Ok(());
            }
            let left = (auto_limit_us - auto_elapsed_us).div_ceil(1_000_000);
            if left != last_drawn_left {
                last_drawn_left = left;
            }
        }

        match read_key_nonblocking(st) {
            Ok(Some(k)) => {
                auto_enabled = false;
                if k.scan_code == SCAN_UP {
                    if sel > 0 {
                        sel -= 1;
                    }
                } else if k.scan_code == SCAN_DOWN {
                    if sel + 1 < boot_ui_gfx::ENTRY_COUNT {
                        sel += 1;
                    }
                } else if k.scan_code == SCAN_HOME {
                    sel = 0;
                } else if k.scan_code == SCAN_END {
                    sel = boot_ui_gfx::ENTRY_COUNT - 1;
                } else if k.unicode_char == b'c' as u16
                    || k.unicode_char == b'C' as u16
                    || k.scan_code == SCAN_F10
                {
                    let (fw, fh) = gop_dims(gop).unwrap_or((800, 600));
                    let _ = run_settings(st, image, gop, cfg, fw, fh)?;
                    sel = match initial_selection_fixed(st, cfg) {
                        Focus::Entry(i) => i,
                        Focus::Footer => 0,
                    };
                    force_text_redraw = true;
                } else if k.unicode_char == 0x000d || k.unicode_char == 0x000a {
                    match ENTRIES[sel].id {
                        EntryId::FluentNt10 => {
                            persist_choice(st, cfg, sel as u8);
                            return Ok(());
                        }
                        EntryId::Reserved => {
                            con_cursor(st, 0, 3 + boot_ui_gfx::ENTRY_COUNT + 7);
                            con_out(st, MSG_RESERVED);
                            stall(st, 1_500_000)?;
                            force_text_redraw = true;
                        }
                        EntryId::ChainBootMgr => {
                            if !cfg.chainload_enabled {
                                con_cursor(st, 0, 3 + boot_ui_gfx::ENTRY_COUNT + 7);
                                con_out(st, MSG_CHAIN_OFF);
                                stall(st, 1_500_000)?;
                                force_text_redraw = true;
                            } else {
                                let path = if cfg.chainload_path_len > 0 {
                                    &cfg.chainload_path[..cfg.chainload_path_len]
                                } else {
                                    b"EFI\\Microsoft\\Boot\\bootmgfw.efi"
                                };
                                let r = chainload::chainload_efi_path(st, image, path);
                                if r != efi::Status::SUCCESS {
                                    con_cursor(st, 0, 3 + boot_ui_gfx::ENTRY_COUNT + 7);
                                    con_out(st, MSG_CHAIN_ERR);
                                    stall(st, 2_000_000)?;
                                    force_text_redraw = true;
                                }
                            }
                        }
                        EntryId::Reboot => reset_system(st, efi::RESET_COLD),
                        EntryId::Shutdown => reset_system(st, efi::RESET_SHUTDOWN),
                    }
                } else {
                    let u = k.unicode_char;
                    if u == u16::from(b'1') {
                        sel = 0;
                    } else if u == u16::from(b'2') {
                        sel = 1;
                    } else if u == u16::from(b'3') {
                        sel = 2;
                    } else if u == u16::from(b'4') {
                        sel = 3;
                    } else if u == u16::from(b'5') {
                        sel = 4;
                    } else if u == u16::from(b'b') || u == u16::from(b'B') {
                        persist_choice(st, cfg, 0);
                        return Ok(());
                    }
                }
            }
            Ok(None) => {}
            Err(e) => return Err(e),
        }

        let tsnap = TextMenuSnapshot {
            sel,
            auto_left: last_drawn_left,
            auto_enabled,
            chain_muted: !cfg.chainload_enabled,
        };
        let text_dirty =
            force_text_redraw || prev_text.map(|p| p != tsnap).unwrap_or(true);
        if text_dirty {
            redraw_text(
                st,
                gop,
                sel,
                kernel_path,
                cfg,
                auto_enabled,
                last_drawn_left,
            );
            force_text_redraw = false;
        }
        prev_text = Some(tsnap);

        let stall_us = if text_dirty {
            POLL_STALL_US
        } else {
            POLL_STALL_IDLE_US
        };
        prev_stall_us = stall_us as u64;
        stall(st, stall_us)?;
    }
}

/// # Safety
/// `st` valid while Boot Services are active.
pub unsafe fn run_boot_menu(
    st: *mut efi::SystemTable,
    image: efi::Handle,
    gop: *mut graphics_output::Protocol,
    kernel_path: &str,
    cfg: &mut Zbm10Cfg,
) -> Result<(), efi::Status> {
    if !gop.is_null() && gop_dims(gop).is_some() {
        run_gfx_menu(st, image, gop, kernel_path, cfg)
    } else {
        run_text_menu(st, image, gop, kernel_path, cfg)
    }
}
