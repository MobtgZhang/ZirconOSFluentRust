//! Win32 message-driven shell overlay for UEFI desktop (Phase 4 close-out + Phase 5 bring-up).
//!
//! Uses a single bound desktop + thread id; `WndProc` entry points read [`UEFI_SESSION`] (UEFI is
//! single-threaded between polls).

use core::ptr::NonNull;

use crate::hal::Hal;
use crate::libs::win32_abi::{Hwnd, LParam, LResult, WParam};
use crate::ob::winsta::{WS_EX_TOOLWINDOW, WIN_EX_NO_HIT_TEST, WIN_EX_SHELL_POPUP};
use crate::subsystems::win32::compositor::{
    composite_desktop_to_framebuffer_filtered, CompositeDesktopFilter,
};
use crate::subsystems::win32::gdi32::{
    bringup_bitblt_bgra_to_slot, bringup_fill_rect_with_selected_brush, bringup_select_solid_brush,
    BringupHdc,
};
use crate::subsystems::win32::msg_dispatch::{
    self, dispatch_message_kernel, post_message_kernel, try_get_message_kernel,
};
use crate::subsystems::win32::text_bringup::text_out_ascii;
use crate::subsystems::win32::win32_paint::{begin_paint_bringup, end_paint_bringup, invalidate_rect_kernel};
use crate::subsystems::win32::window_surface::{
    self, clear_surface, downsample_bgra_nearest_to_slot, fill_rect_surface,
};
use crate::subsystems::win32::windowing::{self, create_window_ex_on_desktop, register_class_ex_bringup, wm};
use crate::subsystems::win32::windowing::ht;

use super::hosted_apps;
use super::resources::{
    desktop_icon_bgra, DEFAULT_WALLPAPER_BGRA, DEFAULT_WALLPAPER_HEIGHT, DEFAULT_WALLPAPER_WIDTH,
    START_MENU_ICON_H, START_MENU_ICON_W,
};
use super::session::DesktopSession;
use super::shell;
use super::taskbar::{TaskbarLayout, TASK_SLOT_COUNT};

/// Per-session Win32 shell HWNDs and interaction state (UEFI overlay).
#[derive(Clone, Debug)]
pub struct Win32ShellState {
    pub desktop_ready: bool,
    pub hwnd_wallpaper: Hwnd,
    pub hwnd_taskbar: Hwnd,
    pub hwnd_icon: Hwnd,
    pub hwnd_test: Hwnd,
    pub hwnd_clock: Hwnd,
    pub hwnd_menu: Hwnd,
    pub capture_hwnd: Option<Hwnd>,
    pub dragging: bool,
    pub drag_anchor_x: i32,
    pub drag_anchor_y: i32,
    pub test_win_x: u32,
    pub test_win_y: u32,
    pub test_win_w: u32,
    pub test_win_h: u32,
    pub icon_x: u32,
    pub icon_y: u32,
    pub win32_menu_open: bool,
    pub win32_menu_x: u32,
    pub win32_menu_y: u32,
    pub win32_menu_sel: usize,
    pub clock_open: bool,
    pub taskbar_timer_armed: bool,
    pub timer_last_poll: u32,
}

impl Default for Win32ShellState {
    fn default() -> Self {
        Self {
            desktop_ready: false,
            hwnd_wallpaper: 0,
            hwnd_taskbar: 0,
            hwnd_icon: 0,
            hwnd_test: 0,
            hwnd_clock: 0,
            hwnd_menu: 0,
            capture_hwnd: None,
            dragging: false,
            drag_anchor_x: 0,
            drag_anchor_y: 0,
            test_win_x: 0,
            test_win_y: 0,
            test_win_w: 200,
            test_win_h: 120,
            icon_x: 48,
            icon_y: 80,
            win32_menu_open: false,
            win32_menu_x: 0,
            win32_menu_y: 0,
            win32_menu_sel: 0,
            clock_open: false,
            taskbar_timer_armed: false,
            timer_last_poll: 0,
        }
    }
}

/// Dedicated Win32 bring-up thread id (avoid collision with unit tests using small tids).
pub const NT10_UEFI_WIN32_TID: u32 = 0x5A01_0000;

static mut UEFI_SESSION: *mut DesktopSession = core::ptr::null_mut();

pub unsafe fn arm_uefi_session(s: *mut DesktopSession) {
    UEFI_SESSION = s;
}

pub unsafe fn disarm_uefi_session() {
    UEFI_SESSION = core::ptr::null_mut();
}

const TITLE_PX: u32 = 9;
const TIMER_POLL_INTERVAL: u32 = 1024;
/// Zircon bring-up: posted to the desktop (wallpaper) HWND when a shell popup item is chosen.
pub const ZR_WM_MENU_COMMAND: u32 = wm::WM_USER + 0x21;

fn taskbar_partition_fill(hdc: BringupHdc, layout: &TaskbarLayout) {
    let bar = layout.bar;
    if bar.w == 0 {
        return;
    }
    let surf_w = window_surface::SURF_W;
    let surf_h = window_surface::SURF_H;
    let to_x = |screen_x: u32| -> u32 {
        let rel = screen_x.saturating_sub(bar.x);
        ((rel as u64 * surf_w as u64) / bar.w as u64).min(surf_w as u64) as u32
    };
    let start_end = (layout.start_button.x + layout.start_button.w + 8).min(bar.x.saturating_add(bar.w));
    let clock_x = layout.clock_area().x.max(bar.x);
    let x0 = to_x(bar.x);
    let x_start_end = to_x(start_end);
    let x_clock = to_x(clock_x);

    bringup_select_solid_brush(hdc, [0x24, 0x24, 0x2c, 0xff]);
    bringup_fill_rect_with_selected_brush(hdc, x0, 0, x_start_end.saturating_sub(x0), surf_h);
    bringup_select_solid_brush(hdc, [0x2c, 0x2c, 0x36, 0xff]);
    bringup_fill_rect_with_selected_brush(
        hdc,
        x_start_end,
        0,
        x_clock.saturating_sub(x_start_end),
        surf_h,
    );
    bringup_select_solid_brush(hdc, [0x1c, 0x1c, 0x28, 0xff]);
    bringup_fill_rect_with_selected_brush(hdc, x_clock, 0, surf_w.saturating_sub(x_clock), surf_h);

    let slots = layout.task_slots();
    for i in 1..TASK_SLOT_COUNT {
        let vx = to_x(slots[i].x).min(surf_w.saturating_sub(1));
        bringup_select_solid_brush(hdc, [0x10, 0x10, 0x18, 0xff]);
        bringup_fill_rect_with_selected_brush(hdc, vx, 2, 1, surf_h.saturating_sub(4));
    }
}

fn wp_wallpaper(hwnd: Hwnd, msg: u32, wp: WParam, lp: LParam) -> LResult {
    let Some(s) = (unsafe { UEFI_SESSION.as_mut() }) else {
        return 0;
    };
    if msg == ZR_WM_MENU_COMMAND {
        let _ = (wp, lp);
        return 0;
    }
    if msg == wm::WM_PAINT {
        if let Some(ps) = begin_paint_bringup(&s.win32_desktop, hwnd) {
            let si = ps.hdc as usize;
            let _ = downsample_bgra_nearest_to_slot(
                si,
                DEFAULT_WALLPAPER_BGRA,
                DEFAULT_WALLPAPER_WIDTH,
                DEFAULT_WALLPAPER_HEIGHT,
            );
            end_paint_bringup(&s.win32_desktop, &ps);
        }
        return 0;
    }
    windowing::def_window_proc_bringup(hwnd, msg, wp, lp)
}

fn wp_taskbar(hwnd: Hwnd, msg: u32, wp: WParam, lp: LParam) -> LResult {
    let Some(s) = (unsafe { UEFI_SESSION.as_mut() }) else {
        return 0;
    };
    match msg {
        wm::WM_PAINT => {
            if let Some(ps) = begin_paint_bringup(&s.win32_desktop, hwnd) {
                let hdc = ps.hdc;
                let si = hdc as usize;
                taskbar_partition_fill(hdc, &s.layout);
                text_out_ascii(si, 4, 10, b"Task", [240, 240, 250, 255]);
                let tn = s.clock_time_n.min(s.clock_time.len() as u8) as usize;
                if tn > 0 {
                    text_out_ascii(si, 70, 10, &s.clock_time[..tn], [200, 220, 255, 255]);
                }
                end_paint_bringup(&s.win32_desktop, &ps);
            }
            0
        }
        wm::WM_TIMER => {
            let _ = invalidate_rect_kernel(&s.win32_desktop, hwnd, 0, 0, window_surface::SURF_W, window_surface::SURF_H);
            0
        }
        _ => windowing::def_window_proc_bringup(hwnd, msg, wp, lp),
    }
}

fn wp_desktop_icon(hwnd: Hwnd, msg: u32, wp: WParam, lp: LParam) -> LResult {
    let Some(s) = (unsafe { UEFI_SESSION.as_mut() }) else {
        return 0;
    };
    if msg == wm::WM_PAINT {
        if let Some(ps) = begin_paint_bringup(&s.win32_desktop, hwnd) {
            let hdc = ps.hdc;
            let si = hdc as usize;
            clear_surface(si);
            bringup_select_solid_brush(hdc, [0x28, 0x28, 0x32, 0xff]);
            bringup_fill_rect_with_selected_brush(hdc, 0, 0, window_surface::SURF_W, window_surface::SURF_H);
            if let Some(icon) = desktop_icon_bgra(0) {
                let _ = bringup_bitblt_bgra_to_slot(
                    hdc,
                    4,
                    4,
                    icon,
                    START_MENU_ICON_W,
                    START_MENU_ICON_H,
                    0,
                    0,
                    START_MENU_ICON_W,
                    START_MENU_ICON_H,
                );
            }
            text_out_ascii(si, 4, 22, b"Icon", [255, 255, 255, 255]);
            end_paint_bringup(&s.win32_desktop, &ps);
        }
        return 0;
    }
    windowing::def_window_proc_bringup(hwnd, msg, wp, lp)
}

fn wp_test(hwnd: Hwnd, msg: u32, wp: WParam, lp: LParam) -> LResult {
    let Some(s) = (unsafe { UEFI_SESSION.as_mut() }) else {
        return 0;
    };
    match msg {
        wm::WM_NCHITTEST => {
            let px = (lp as u32) & 0xFFFF;
            let py = ((lp as u32) >> 16) & 0xFFFF;
            let tx = s.win32.test_win_x;
            let ty = s.win32.test_win_y;
            let tw = s.win32.test_win_w;
            let th = s.win32.test_win_h;
            if px < tx || py < ty || px >= tx.saturating_add(tw) || py >= ty.saturating_add(th) {
                return ht::HTNOWHERE;
            }
            let rel_x = px.saturating_sub(tx);
            let rel_y = py.saturating_sub(ty);
            const BORDER: u32 = 6;
            let on_edge = rel_x.saturating_add(BORDER) >= tw || rel_y.saturating_add(BORDER) >= th;
            let title_screen = (th.saturating_mul(TITLE_PX)) / window_surface::SURF_H.max(1);
            if rel_y < title_screen {
                ht::HTCAPTION
            } else if on_edge {
                ht::HTBORDER
            } else {
                ht::HTCLIENT
            }
        }
        wm::WM_PAINT => {
            if let Some(ps) = begin_paint_bringup(&s.win32_desktop, hwnd) {
                let hdc = ps.hdc;
                let si = hdc as usize;
                bringup_select_solid_brush(hdc, [0x40, 0x44, 0x4c, 0xff]);
                bringup_fill_rect_with_selected_brush(hdc, 0, 0, window_surface::SURF_W, window_surface::SURF_H);
                fill_rect_surface(
                    si,
                    0,
                    0,
                    window_surface::SURF_W,
                    TITLE_PX.min(window_surface::SURF_H),
                    [0, 92, 160, 255],
                );
                text_out_ascii(si, 4, 2, b"Win", [255, 255, 255, 255]);
                text_out_ascii(si, 4, 14, b"Client", [230, 230, 240, 255]);
                end_paint_bringup(&s.win32_desktop, &ps);
            }
            0
        }
        _ => windowing::def_window_proc_bringup(hwnd, msg, wp, lp),
    }
}

fn wp_clock_popup(hwnd: Hwnd, msg: u32, wp: WParam, lp: LParam) -> LResult {
    let Some(s) = (unsafe { UEFI_SESSION.as_mut() }) else {
        return 0;
    };
    if msg == wm::WM_PAINT {
        // Serial keyword (verify-phase5): nt10-phase5: CLOCK_FLYOUT rtc-refresh
        if let Some(ps) = begin_paint_bringup(&s.win32_desktop, hwnd) {
            let hdc = ps.hdc;
            let si = hdc as usize;
            bringup_select_solid_brush(hdc, [0x22, 0x22, 0x2e, 0xff]);
            bringup_fill_rect_with_selected_brush(hdc, 0, 0, window_surface::SURF_W, window_surface::SURF_H);
            let tn = s.clock_time_n.min(s.clock_time.len() as u8) as usize;
            let dn = s.clock_date_n.min(s.clock_date.len() as u8) as usize;
            if tn > 0 {
                text_out_ascii(si, 4, 4, &s.clock_time[..tn], [255, 255, 255, 255]);
            }
            if dn > 0 {
                text_out_ascii(si, 4, 16, &s.clock_date[..dn.min(16)], [200, 210, 230, 255]);
            }
            end_paint_bringup(&s.win32_desktop, &ps);
        }
        return 0;
    }
    windowing::def_window_proc_bringup(hwnd, msg, wp, lp)
}

fn wp_menu_popup(hwnd: Hwnd, msg: u32, wp: WParam, lp: LParam) -> LResult {
    let Some(s) = (unsafe { UEFI_SESSION.as_mut() }) else {
        return 0;
    };
    if msg == wm::WM_PAINT {
        if let Some(ps) = begin_paint_bringup(&s.win32_desktop, hwnd) {
            let hdc = ps.hdc;
            let si = hdc as usize;
            bringup_select_solid_brush(hdc, [0x2a, 0x2a, 0x34, 0xff]);
            bringup_fill_rect_with_selected_brush(hdc, 0, 0, window_surface::SURF_W, window_surface::SURF_H);
            let rows = [b"Open".as_slice(), b"View".as_slice(), b"Sort".as_slice()];
            for (i, row) in rows.iter().enumerate() {
                let y = 4 + (i as u32) * 9;
                let sel = i == s.win32.win32_menu_sel.min(2);
                let fg = if sel {
                    [255, 255, 255, 255]
                } else {
                    [200, 200, 210, 255]
                };
                text_out_ascii(si, 6, y, row, fg);
            }
            end_paint_bringup(&s.win32_desktop, &ps);
        }
        return 0;
    }
    windowing::def_window_proc_bringup(hwnd, msg, wp, lp)
}

/// Pump posted messages for the UEFI Win32 desktop (arms session for `WndProc`).
pub fn pump_uefi_win32(session: &mut DesktopSession) {
    if !session.win32.desktop_ready {
        return;
    }
    unsafe {
        arm_uefi_session(session as *mut DesktopSession);
    }
    let tid = NT10_UEFI_WIN32_TID;
    while let Some(m) = try_get_message_kernel(tid) {
        let _ = dispatch_message_kernel(&session.win32_desktop, m);
    }
    unsafe {
        disarm_uefi_session();
    }
}

/// Fire `WM_TIMER` on the taskbar at ~[`TIMER_POLL_INTERVAL`] polls (UEFI has no reliable IRQ quanta).
pub fn maybe_post_timer(session: &mut DesktopSession) {
    if !session.win32.desktop_ready || !session.win32.taskbar_timer_armed {
        return;
    }
    if session
        .poll_seq
        .wrapping_sub(session.win32.timer_last_poll)
        < TIMER_POLL_INTERVAL
    {
        return;
    }
    session.win32.timer_last_poll = session.poll_seq;
    let _ = post_message_kernel(
        &session.win32_desktop,
        session.win32.hwnd_taskbar,
        wm::WM_TIMER,
        1,
        0,
    );
}

/// Composite only the bottom Z-layer (wallpaper HWND) into `dst_buf` (same shape as GOP).
pub fn composite_win32_wallpaper_only_to_buffer(
    session: &DesktopSession,
    dst_buf: &mut [u8],
) -> Result<(), ()> {
    if !session.win32.desktop_ready {
        return Ok(());
    }
    let dst_w = session.fb.horizontal_resolution;
    let dst_h = session.fb.vertical_resolution;
    let stride = session.fb.pixels_per_scan_line;
    composite_desktop_to_framebuffer_filtered(
        &session.win32_desktop,
        dst_buf,
        dst_w,
        dst_h,
        stride,
        0,
        0,
        CompositeDesktopFilter::BottomLayerOnly,
    )
}

/// Composite Win32 layers above the wallpaper (after shell draws shortcuts/chrome on top of wallpaper).
/// [`DesktopSession::refresh_desktop`](super::session::DesktopSession::refresh_desktop) draws the pointer **after** this.
pub fn composite_win32_above_wallpaper_to_gop(session: &mut DesktopSession) {
    if !session.win32.desktop_ready || session.fb.base == 0 {
        return;
    }
    let cap = crate::drivers::video::display_mgr::framebuffer_linear_byte_cap(&session.fb);
    let dst_w = session.fb.horizontal_resolution;
    let dst_h = session.fb.vertical_resolution;
    let stride = session.fb.pixels_per_scan_line;
    unsafe {
        let buf = core::slice::from_raw_parts_mut(session.fb.base as *mut u8, cap);
        let super::session::DesktopSession {
            ref mut dwm,
            ref win32_desktop,
            ..
        } = session;
        let _ = super::dwm::composite_desktop_with_dwm_overlay(
            dwm,
            win32_desktop,
            buf,
            dst_w,
            dst_h,
            stride,
            CompositeDesktopFilter::ExcludeBottomLayer,
        );
    }
}

/// Composite Win32 desktop into the linear GOP mapping after the Fluent shell has drawn.
/// Caller must paint the software pointer on top afterward (see `refresh_desktop` / pointer move paths).
pub fn composite_win32_to_gop(session: &mut DesktopSession) {
    if !session.win32.desktop_ready || session.fb.base == 0 {
        return;
    }
    let cap = crate::drivers::video::display_mgr::framebuffer_linear_byte_cap(&session.fb);
    let dst_w = session.fb.horizontal_resolution;
    let dst_h = session.fb.vertical_resolution;
    let stride = session.fb.pixels_per_scan_line;
    unsafe {
        let buf = core::slice::from_raw_parts_mut(session.fb.base as *mut u8, cap);
        let super::session::DesktopSession {
            ref mut dwm,
            ref win32_desktop,
            ..
        } = session;
        let _ = super::dwm::composite_desktop_with_dwm_overlay(
            dwm,
            win32_desktop,
            buf,
            dst_w,
            dst_h,
            stride,
            CompositeDesktopFilter::All,
        );
    }
}

/// Alt+Tab style candidate HWNDs (top-most first); empty when Win32 desktop is not ready.
#[allow(dead_code)]
pub fn alt_tab_visible_hwnds(session: &DesktopSession, out: &mut [Hwnd]) -> usize {
    if !session.win32.desktop_ready {
        return 0;
    }
    session
        .win32_desktop
        .collect_visible_switcher_hwnds(out)
}

pub fn init_uefi_win32<H: Hal + ?Sized>(session: &mut DesktopSession, hal: &H) {
    if session.win32.desktop_ready {
        return;
    }
    let tid = NT10_UEFI_WIN32_TID;
    msg_dispatch::set_current_thread_for_win32(tid);
    let dptr = NonNull::from(&mut session.win32_desktop);
    msg_dispatch::thread_bind_desktop(tid, dptr);
    let desktop = unsafe { dptr.as_ref() };

    let aw = register_class_ex_bringup(0, 0x7E01).expect("wall class");
    let at = register_class_ex_bringup(0, 0x7E02).expect("tb class");
    let ai = register_class_ex_bringup(0, 0x7E03).expect("icon class");
    let ak = register_class_ex_bringup(0, 0x7E04).expect("test class");
    let ac = register_class_ex_bringup(0, 0x7E05).expect("clock class");
    let am = register_class_ex_bringup(0, 0x7E06).expect("menu class");

    let w = session.fb.horizontal_resolution;
    let h = session.fb.vertical_resolution;
    let bar = session.layout.bar;

    let hwnd_wall = create_window_ex_on_desktop(desktop, aw, 0, tid, wp_wallpaper).expect("wall");
    let hwnd_icon = create_window_ex_on_desktop(desktop, ai, 0, tid, wp_desktop_icon).expect("icon");
    let hwnd_task = create_window_ex_on_desktop(desktop, at, 0, tid, wp_taskbar).expect("task");
    let hwnd_test = create_window_ex_on_desktop(desktop, ak, 0, tid, wp_test).expect("test");
    let hwnd_clock = create_window_ex_on_desktop(desktop, ac, 0, tid, wp_clock_popup).expect("clk");
    let hwnd_menu = create_window_ex_on_desktop(desktop, am, 0, tid, wp_menu_popup).expect("menu");

    let _ = desktop.set_window_ex_style(hwnd_wall, WIN_EX_NO_HIT_TEST);
    let _ = desktop.set_window_ex_style(hwnd_task, WS_EX_TOOLWINDOW);
    let _ = desktop.set_window_ex_style(hwnd_menu, WIN_EX_SHELL_POPUP);

    let _ = desktop.set_window_placement(hwnd_wall, 0, 0, w, h);
    let _ = desktop.set_window_placement(hwnd_task, bar.x, bar.y, bar.w, bar.h);

    session.win32.icon_x = 48;
    session.win32.icon_y = 80;
    let iw = 96u32;
    let ih = 56u32;
    let _ = desktop.set_window_placement(hwnd_icon, session.win32.icon_x, session.win32.icon_y, iw, ih);

    session.win32.test_win_x = 120;
    session.win32.test_win_y = 90;
    session.win32.test_win_w = 220;
    session.win32.test_win_h = 130;
    let _ = desktop.set_window_placement(
        hwnd_test,
        session.win32.test_win_x,
        session.win32.test_win_y,
        session.win32.test_win_w,
        session.win32.test_win_h,
    );

    let _ = desktop.set_window_placement(hwnd_clock, w.saturating_sub(140), bar.y.saturating_sub(52), 128, 44);
    let _ = desktop.set_window_minimized(hwnd_clock, true);

    let _ = desktop.set_window_placement(hwnd_menu, 0, 0, 1, 1);
    let _ = desktop.set_window_minimized(hwnd_menu, true);

    session.win32.hwnd_wallpaper = hwnd_wall;
    session.win32.hwnd_taskbar = hwnd_task;
    session.win32.hwnd_icon = hwnd_icon;
    session.win32.hwnd_test = hwnd_test;
    session.win32.hwnd_clock = hwnd_clock;
    session.win32.hwnd_menu = hwnd_menu;
    session.win32.desktop_ready = true;
    session.win32.taskbar_timer_armed = true;
    session.win32.timer_last_poll = session.poll_seq;

    for &hh in &[
        hwnd_wall,
        hwnd_icon,
        hwnd_task,
        hwnd_test,
        hwnd_clock,
        hwnd_menu,
    ] {
        let _ = invalidate_rect_kernel(desktop, hh, 0, 0, window_surface::SURF_W, window_surface::SURF_H);
    }

    unsafe {
        arm_uefi_session(session as *mut DesktopSession);
    }
    while let Some(m) = try_get_message_kernel(tid) {
        let _ = dispatch_message_kernel(desktop, m);
    }
    unsafe {
        disarm_uefi_session();
    }

    hal.debug_write(b"nt10-phase4: GOP_COMPOSITE win32 layer init OK\r\n");
}

/// `SetTimer` bring-up: arm periodic `WM_TIMER` for taskbar repaint (syscall / host wiring).
#[allow(dead_code)]
pub fn set_timer_taskbar_bringup(session: &mut DesktopSession) {
    session.win32.taskbar_timer_armed = true;
}

/// `KillTimer` bring-up.
#[allow(dead_code)]
pub fn kill_timer_taskbar_bringup(session: &mut DesktopSession) {
    session.win32.taskbar_timer_armed = false;
}

pub fn set_capture_bringup(session: &mut DesktopSession, hwnd: Hwnd) {
    session.win32.capture_hwnd = Some(hwnd);
}

pub fn release_capture_bringup(session: &mut DesktopSession) {
    session.win32.capture_hwnd = None;
}

/// Returns `true` if the shell should skip further left-down handling for this click.
pub fn handle_left_down<H: Hal + ?Sized>(session: &mut DesktopSession, hal: &H) -> bool {
    if !session.win32.desktop_ready {
        return false;
    }
    let px = session.cx;
    let py = session.cy;

    if session.win32.win32_menu_open {
        let mx = session.win32.win32_menu_x;
        let my = session.win32.win32_menu_y;
        let mw = 180u32;
        let mh = 120u32;
        if px >= mx && px < mx + mw && py >= my && py < my + mh {
            let row = ((py - my) / 40).min(2) as usize;
            session.win32.win32_menu_sel = row;
            let _ = invalidate_rect_kernel(
                &session.win32_desktop,
                session.win32.hwnd_menu,
                0,
                0,
                window_surface::SURF_W,
                window_surface::SURF_H,
            );
            let _ = post_message_kernel(
                &session.win32_desktop,
                session.win32.hwnd_wallpaper,
                ZR_WM_MENU_COMMAND,
                row as WParam,
                0,
            );
            if row == 0 {
                hal.debug_write(b"nt10-phase5: MENU_CMD Open\r\n");
            }
            return true;
        }
        session.win32.win32_menu_open = false;
        let _ = session
            .win32_desktop
            .set_window_minimized(session.win32.hwnd_menu, true);
        session.refresh_desktop();
        return true;
    }

    if let Some(cap) = session.win32.capture_hwnd {
        if cap == session.win32.hwnd_test && session.win32.dragging {
            let _ = post_message_kernel(&session.win32_desktop, cap, wm::WM_LBUTTONUP, 0, 0);
            session.win32.dragging = false;
            release_capture_bringup(session);
            let _ = session
                .win32_desktop
                .bring_hwnd_to_top(session.win32.hwnd_test);
            session.refresh_desktop();
            return true;
        }
        release_capture_bringup(session);
    }

    if let Some(hit) = session.win32_desktop.hit_test_screen_topmost(px, py) {
        if hit == session.win32.hwnd_menu {
            return true;
        }
        if hit == session.win32.hwnd_clock
            && !session
                .win32_desktop
                .is_window_minimized(session.win32.hwnd_clock)
        {
            session.win32.clock_open = false;
            let _ = session
                .win32_desktop
                .set_window_minimized(session.win32.hwnd_clock, true);
            hal.debug_write(b"nt10-phase5: CLOCK_POP close\r\n");
            session.refresh_desktop();
            return true;
        }
        if hit == session.win32.hwnd_taskbar {
            if shell::hit_test_clock_display(&session.layout, px, py) {
                let was_min = session
                    .win32_desktop
                    .is_window_minimized(session.win32.hwnd_clock);
                let _ = session
                    .win32_desktop
                    .set_window_minimized(session.win32.hwnd_clock, !was_min);
                if was_min {
                    hal.debug_write(b"nt10-phase5: CLOCK_POP open\r\n");
                    let c = session.layout.clock_display_area();
                    let _ = session.win32_desktop.set_window_placement(
                        session.win32.hwnd_clock,
                        c.x.saturating_sub(8),
                        session.layout.bar.y.saturating_sub(56),
                        128,
                        48,
                    );
                } else {
                    hal.debug_write(b"nt10-phase5: CLOCK_POP close\r\n");
                }
                session.win32.clock_open = !session
                    .win32_desktop
                    .is_window_minimized(session.win32.hwnd_clock);
                let _ = invalidate_rect_kernel(
                    &session.win32_desktop,
                    session.win32.hwnd_clock,
                    0,
                    0,
                    window_surface::SURF_W,
                    window_surface::SURF_H,
                );
                session.refresh_desktop();
                return true;
            }
            let slots = session.layout.task_slots();
            for i in 0..TASK_SLOT_COUNT {
                if slots[i].contains(px, py) {
                    if let Some(aid) = hosted_apps::task_slot_app(&session.stack, i) {
                        if session.stack.top() == Some(aid) {
                            session.stack.remove(aid);
                            hal.debug_write(b"nt10-phase5: SW_MINIMIZE slot\r\n");
                        } else {
                            session.stack.push_front(aid);
                            hal.debug_write(b"nt10-phase5: SW_RESTORE slot\r\n");
                        }
                        let _ = invalidate_rect_kernel(
                            &session.win32_desktop,
                            session.win32.hwnd_taskbar,
                            0,
                            0,
                            window_surface::SURF_W,
                            window_surface::SURF_H,
                        );
                        session.refresh_desktop();
                        return true;
                    }
                    if i == 0 {
                        let min = session
                            .win32_desktop
                            .is_window_minimized(session.win32.hwnd_test);
                        let _ = session
                            .win32_desktop
                            .set_window_minimized(session.win32.hwnd_test, !min);
                        if !min {
                            hal.debug_write(b"nt10-phase5: SW_MINIMIZE test\r\n");
                        } else {
                            hal.debug_write(b"nt10-phase5: SW_RESTORE test\r\n");
                            let _ = session
                                .win32_desktop
                                .bring_hwnd_to_top(session.win32.hwnd_test);
                        }
                        let _ = invalidate_rect_kernel(
                            &session.win32_desktop,
                            session.win32.hwnd_taskbar,
                            0,
                            0,
                            window_surface::SURF_W,
                            window_surface::SURF_H,
                        );
                        session.refresh_desktop();
                        return true;
                    }
                    return false;
                }
            }
        }
        if hit == session.win32.hwnd_test {
            let code = nc_hit_test_test_window(session, px, py);
            if code == ht::HTCAPTION {
                session.win32.dragging = true;
                set_capture_bringup(session, session.win32.hwnd_test);
                session.win32.drag_anchor_x = px as i32;
                session.win32.drag_anchor_y = py as i32;
                return true;
            }
            if code == ht::HTCLIENT {
                let _ = post_message_kernel(
                    &session.win32_desktop,
                    session.win32.hwnd_test,
                    wm::WM_LBUTTONDOWN,
                    0,
                    0,
                );
                hal.debug_write(b"nt10-phase4: WM_LBUTTONDOWN client\r\n");
                session.refresh_desktop();
                return true;
            }
        }
    }
    false
}

/// Returns `true` if the desktop context menu was opened (Win32 popup).
pub fn handle_right_down(session: &mut DesktopSession) -> bool {
    if !session.win32.desktop_ready {
        return false;
    }
    let px = session.cx;
    let py = session.cy;
    if session.layout.bar.contains(px, py) {
        return false;
    }
    if session
        .win32_desktop
        .hit_test_screen_topmost(px, py)
        .is_some()
    {
        return false;
    }
    let d = &session.win32_desktop;
    session.win32.win32_menu_open = true;
    session.win32.win32_menu_sel = 0;
    session.win32.win32_menu_x = px.min(session.fb.horizontal_resolution.saturating_sub(200));
    session.win32.win32_menu_y = py.min(session.fb.vertical_resolution.saturating_sub(160));
    let _ = d.set_window_placement(
        session.win32.hwnd_menu,
        session.win32.win32_menu_x,
        session.win32.win32_menu_y,
        180,
        120,
    );
    let _ = d.set_window_minimized(session.win32.hwnd_menu, false);
    let _ = d.bring_hwnd_to_top(session.win32.hwnd_menu);
    let _ = invalidate_rect_kernel(d, session.win32.hwnd_menu, 0, 0, window_surface::SURF_W, window_surface::SURF_H);
    session.refresh_desktop();
    true
}

pub fn handle_pointer_move(session: &mut DesktopSession) {
    if !session.win32.desktop_ready {
        return;
    }
    if !session.win32.dragging {
        return;
    }
    let px = session.cx as i32;
    let py = session.cy as i32;
    let dx = px - session.win32.drag_anchor_x;
    let dy = py - session.win32.drag_anchor_y;
    if dx == 0 && dy == 0 {
        return;
    }
    session.win32.drag_anchor_x = px;
    session.win32.drag_anchor_y = py;
    let w = session.fb.horizontal_resolution;
    let h = session.fb.vertical_resolution;
    let tw = session.win32.test_win_w;
    let th = session.win32.test_win_h;
    let nx = (session.win32.test_win_x as i32 + dx)
        .clamp(0, w.saturating_sub(tw) as i32) as u32;
    let ny = (session.win32.test_win_y as i32 + dy)
        .clamp(0, h.saturating_sub(th) as i32) as u32;
    session.win32.test_win_x = nx;
    session.win32.test_win_y = ny;
    let _ = session.win32_desktop.set_window_placement(session.win32.hwnd_test, nx, ny, tw, th);
    let _ = post_message_kernel(
        &session.win32_desktop,
        session.win32.hwnd_test,
        wm::WM_MOUSEMOVE,
        0,
        ((py as i64) << 16 | (px as i64 & 0xFFFF)) as LParam,
    );
}

fn nc_hit_test_test_window(session: &DesktopSession, px: u32, py: u32) -> LResult {
    let tx = session.win32.test_win_x;
    let ty = session.win32.test_win_y;
    let tw = session.win32.test_win_w;
    let th = session.win32.test_win_h;
    if px < tx || py < ty || px >= tx.saturating_add(tw) || py >= ty.saturating_add(th) {
        return ht::HTNOWHERE;
    }
    let rel_x = px.saturating_sub(tx);
    let rel_y = py.saturating_sub(ty);
    const BORDER: u32 = 6;
    let on_edge = rel_x.saturating_add(BORDER) >= tw || rel_y.saturating_add(BORDER) >= th;
    let title_screen = (th.saturating_mul(TITLE_PX)) / window_surface::SURF_H.max(1);
    if rel_y < title_screen {
        ht::HTCAPTION
    } else if on_edge {
        ht::HTBORDER
    } else {
        ht::HTCLIENT
    }
}
