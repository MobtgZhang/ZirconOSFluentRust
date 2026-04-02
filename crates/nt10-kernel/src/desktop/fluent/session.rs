//! UEFI bring-up: xHCI HID + PS/2, wallpaper/chrome redraw, Start menu (keyboard + mouse), cursor.
//!
//! Taskbar semantics (tray, clock, taskbar context menu) follow the same *roles* as in Microsoft’s
//! shell docs under `references/win32/desktop-src/shell/taskbar.md` and `shell/notification-area.md`
//! (notification area, optional clock, taskbar shortcut menu — we do not implement Win32 APIs).
//!
//! ## Pointer as top overlay (Win32-style)
//!
//! Like the system cursor drawn above the client area in Win32, the software pointer is a **separate
//! compositing layer**: paint the full desktop scene first ([`shell::redraw_uefi_desktop`]), then
//! `pointer_capture_under` / `pointer_paint_on_fb`. On moves, `pointer_remove_from_fb` restores pixels
//! before updating the hotspot.
//! Full-scene refreshes **do not** call remove_from_fb first (the repaint overwrites the surface, avoiding
//! stale save-under data).
//!
//! The pointer sprite is drawn in [`super::shell::paint_pointer_cursor`], respecting GOP `pixel_format`
//! via [`crate::drivers::video::display_mgr`]. Hotspot `(cx, cy)` is framebuffer pixels, top-left origin,
//! Y down — see `references/win32/desktop-src/LearnWin32/mouse-movement.md`. Keyboard routing aligns with
//! `references/win32/desktop-src/inputdev/keyboard-input.md`; raw HID with `raw-input.md`.

use core::sync::atomic::{AtomicBool, Ordering};

use nt10_boot_protocol::FramebufferInfo;

#[cfg(target_arch = "x86_64")]
use crate::drivers::bus::xhci::{self, XhciHidState};
use crate::drivers::input::i8042;
use crate::drivers::input::input_mgr::{InputManager, KeyEvent, PointerEvent};
use crate::drivers::input::ps2::{self, MouseStreamState, Ps2MousePacket, Ps2ScanDecoder};
#[cfg(target_arch = "x86_64")]
use crate::drivers::input::usb_hid;
use crate::drivers::input::vkey;
use crate::drivers::video::display_mgr;
use crate::hal::Hal;

use super::app_host::{AppId, WindowStack};
use super::dwm::{DirtyRect, DwmCompositor};
use super::explorer_view;
use super::hosted_apps::{self, stack_to_bytes, HostUiState};
use super::shell::{self, DesktopChromeState};
use super::taskbar::{TaskbarLayout, TASK_SLOT_COUNT, START_MENU_ROW_COUNT};
use super::session_win32;
use super::wall_clock;

use crate::ob::winsta::DesktopObject;

static FIRST_POINTER_MOTION: AtomicBool = AtomicBool::new(false);
static FIRST_RIGHT_BUTTON: AtomicBool = AtomicBool::new(false);

/// Serial (COM1) lines on pointer move or button change. Set to `false` to silence `nt10-mouse:` logs.
pub const MOUSE_POINTER_SERIAL_DEBUG: bool = true;

/// Minimum [`DesktopSession::poll_seq`] steps between accepted context-menu right-clicks (noise / chatter).
const RCLICK_DEBOUNCE_POLLS: u32 = 12;

/// PS/2 aux reports often set **middle** (bit 2) or **right+middle** spuriously under fast motion; we do not use middle.
/// Also reject impossible multi-button chords on the first frame of a large delta (sync / EMI glitches).
fn sanitize_ps2_mouse_buttons(buttons: u8, dx: i16, dy: i16, prev_l: bool, prev_r: bool) -> u8 {
    let mut b = buttons & 7;
    b &= !4;
    let motion = dx.unsigned_abs().saturating_add(dy.unsigned_abs());
    let n_down = ((b & 1) != 0) as u8 + ((b & 2) != 0) as u8;
    if n_down >= 2 && motion >= 6 && !prev_l && !prev_r {
        return 0;
    }
    if (b & 2) != 0 && !prev_r && motion > 12 {
        b &= !2;
    }
    b
}

/// xHCI [`xhci_init_hid`] runs only after this many [`DesktopSession::poll`] calls (PS/2 is drained first each time).
///
/// **Must be large:** `xhci_init_hid` can run for a very long time or appear hung; if it runs on poll 1 while the
/// PS/2 queue was empty, the main loop never reaches another poll and the pointer stays frozen.
/// Set to `0` only when you know USB init returns quickly on your machine.
///
/// See `docs/cn/Build-Test-Coding.md` (UEFI 桌面会话): compile with `NT10_SKIP_XHCI=1` to force PS/2-only, or tune this constant.
#[cfg(target_arch = "x86_64")]
pub const XHCI_INIT_AFTER_POLLS: u32 = 100_000;

/// If `true`, never call `xhci_init_hid` (PS/2 keyboard/mouse only). Set environment variable
/// **`NT10_SKIP_XHCI`** to any value when running `cargo build` / `cargo check` so `option_env!` picks it up
/// (e.g. `NT10_SKIP_XHCI=1 cargo build ...`). See `docs/cn/Build-Test-Coding.md`.
#[cfg(target_arch = "x86_64")]
pub const SKIP_XHCI_INIT: bool = option_env!("NT10_SKIP_XHCI").is_some();

/// Cursor XOR/save-under buffer — kept in BSS, not on the kernel stack (UEFI entry stack is small).
const CURSOR_SAVE_BYTES: usize = (shell::POINTER_CURSOR_SIZE as usize)
    * (shell::POINTER_CURSOR_SIZE as usize)
    * 4;
static mut DESKTOP_CURSOR_SAVE: [u8; CURSOR_SAVE_BYTES] = [0; CURSOR_SAVE_BYTES];

/// Tight BGRA wallpaper + desktop shortcuts; see [`shell::DESKTOP_BASE_LAYER_CAP_BYTES`].
static mut DESKTOP_BASE_CACHE: [u8; shell::DESKTOP_BASE_LAYER_CAP_BYTES] =
    [0u8; shell::DESKTOP_BASE_LAYER_CAP_BYTES];

pub fn run_uefi_desktop_poll_session<H: Hal + ?Sized>(hal: &H, fb: FramebufferInfo) -> ! {
    hal.debug_write(b"nt10-session: entered poll session (before desktop init)\r\n");
    let mut s = DesktopSession::new_uefi(hal, fb);
    let linear_cap = display_mgr::framebuffer_linear_byte_cap(&s.fb);
    hal.debug_write(b"nt10-pointer: FrameBufferSize=");
    debug_write_usize_dec(hal, s.fb.size);
    hal.debug_write(b" linear_byte_cap=");
    debug_write_usize_dec(hal, linear_cap);
    hal.debug_write(b" ppsl=");
    debug_write_u32_dec(hal, s.fb.pixels_per_scan_line);
    hal.debug_write(b" cursor_bgra_len=");
    debug_write_usize_dec(hal, shell::pointer_cursor_asset_len());
    hal.debug_write(b" expected=");
    debug_write_usize_dec(hal, (shell::POINTER_CURSOR_SIZE as usize).pow(2) * 4);
    hal.debug_write(b"\r\n");
    display_mgr::log_uefi_framebuffer_diag(hal, &s.fb);
    display_mgr::uefi_framebuffer_touch_selftest(hal, &s.fb);
    hal.debug_write(b"nt10-kernel: UEFI desktop session (wallpaper + Start menu + input)\r\n");
    hal.debug_write(b"nt10-mouse: PS/2 aux drained before USB each poll (i8042)\r\n");
    #[cfg(target_arch = "x86_64")]
    {
        hal.debug_write(b"nt10-mouse: USB xHCI init after poll tick ");
        debug_write_usize_dec(hal, XHCI_INIT_AFTER_POLLS as usize);
        hal.debug_write(b" (PS/2-only until then; set env NT10_SKIP_XHCI when building, or tune XHCI_INIT_AFTER_POLLS)\r\n");
    }
    #[cfg(not(target_arch = "x86_64"))]
    hal.debug_write(b"nt10-mouse: USB xHCI path not built for this arch\r\n");
    if MOUSE_POINTER_SERIAL_DEBUG {
        hal.debug_write(b"nt10-mouse: serial debug ON (see MOUSE_POINTER_SERIAL_DEBUG in session.rs)\r\n");
    }
    loop {
        s.poll(hal);
        unsafe {
            core::arch::asm!("pause", options(nomem, nostack));
        }
    }
}

pub struct DesktopSession {
    pub fb: FramebufferInfo,
    pub cx: u32,
    pub cy: u32,
    dwm: DwmCompositor,
    pub(crate) layout: TaskbarLayout,
    #[cfg(target_arch = "x86_64")]
    xhci: Option<XhciHidState>,
    /// `xhci_init_hid` is deferred so PS/2 + pointer paint are not blocked by USB MMIO waits.
    #[cfg(target_arch = "x86_64")]
    xhci_probe_attempted: bool,
    #[cfg(target_arch = "x86_64")]
    poll_tick: u32,
    /// Incremented at the start of every [`DesktopSession::poll`] (all arches).
    pub(crate) poll_seq: u32,
    /// Last [`poll_seq`] when a context-menu right-click was accepted (debounce).
    last_rclick_poll_seq: u32,
    /// Consecutive reports with right button down (need several before accepting a click — PS/2 noise).
    ptr_right_stable_reports: u8,
    ps2_mouse: MouseStreamState,
    ps2_kbd: Ps2ScanDecoder,
    pub input: InputManager,
    menu_open: bool,
    menu_sel: usize,
    ptr_left_down: bool,
    ptr_right_down: bool,
    pub(crate) stack: WindowStack,
    host_ui: HostUiState,
    power_confirm_open: bool,
    /// 0 = desktop context, 1 = Files list context.
    ctx_from_files: bool,
    last_lclick_seq: u32,
    last_lclick_slot: Option<u8>,
    ctx_open: bool,
    ctx_sel: usize,
    ctx_x: u32,
    ctx_y: u32,
    usb_shift_down: bool,
    #[cfg(target_arch = "x86_64")]
    prev_usb_gui: bool,
    /// Wallpaper + shortcut layer cached for fast redraw (`shell::redraw_uefi_desktop_from_base_cache`).
    base_cache_active: bool,
    base_cache_w: u32,
    base_cache_h: u32,
    pub(crate) clock_time: [u8; 16],
    pub(crate) clock_date: [u8; 20],
    pub(crate) clock_time_n: u8,
    pub(crate) clock_date_n: u8,
    last_rtc_second: u8,
    uptime_secs: u32,
    last_uptime_adv_poll: u32,
    flyout: u8,
    flyout_anchor_x: u32,
    flyout_anchor_y: u32,
    tb_ctx_open: bool,
    tb_ctx_sel: usize,
    tb_ctx_x: u32,
    tb_ctx_y: u32,
    /// Win32 desktop object + HWND shell overlay (Phase 4/5 UEFI bring-up).
    pub win32_desktop: DesktopObject,
    pub win32: session_win32::Win32ShellState,
}

impl DesktopSession {
    pub fn has_xhci_hid(&self) -> bool {
        #[cfg(target_arch = "x86_64")]
        {
            return self.xhci.is_some();
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            false
        }
    }

    pub fn new_uefi<H: Hal + ?Sized>(hal: &H, fb: FramebufferInfo) -> Self {
        hal.debug_write(b"nt10-session: new_uefi begin\r\n");
        let w = fb.horizontal_resolution;
        let h = fb.vertical_resolution;
        let mut dwm = DwmCompositor::new();
        dwm.attach_framebuffer(w, h);
        let layout = TaskbarLayout::for_surface(w, h);
        let cx = w / 2;
        let cy = h / 2;
        hal.debug_write(b"nt10-session: PS/2 i8042 + mouse streaming\r\n");
        unsafe {
            i8042::init_ps2_ports_poll();
            ps2::enable_mouse_streaming();
        }
        hal.debug_write(b"nt10-session: PS/2 setup done; xHCI deferred\r\n");
        let mut s = Self {
            fb,
            cx,
            cy,
            dwm,
            layout,
            #[cfg(target_arch = "x86_64")]
            xhci: None,
            #[cfg(target_arch = "x86_64")]
            xhci_probe_attempted: false,
            #[cfg(target_arch = "x86_64")]
            poll_tick: 0,
            poll_seq: 0,
            last_rclick_poll_seq: 0u32.wrapping_sub(RCLICK_DEBOUNCE_POLLS),
            ptr_right_stable_reports: 0,
            ps2_mouse: MouseStreamState::new(),
            ps2_kbd: Ps2ScanDecoder::new(),
            input: InputManager::new(),
            menu_open: false,
            menu_sel: 0,
            ptr_left_down: false,
            ptr_right_down: false,
            stack: WindowStack::new(),
            host_ui: HostUiState::default(),
            power_confirm_open: false,
            ctx_from_files: false,
            last_lclick_seq: 0,
            last_lclick_slot: None,
            ctx_open: false,
            ctx_sel: 0,
            ctx_x: 0,
            ctx_y: 0,
            usb_shift_down: false,
            #[cfg(target_arch = "x86_64")]
            prev_usb_gui: false,
            base_cache_active: false,
            base_cache_w: 0,
            base_cache_h: 0,
            clock_time: [0; 16],
            clock_date: [0; 20],
            clock_time_n: 0,
            clock_date_n: 0,
            last_rtc_second: 0xff,
            uptime_secs: 0,
            last_uptime_adv_poll: 0,
            flyout: shell::FLYOUT_KIND_NONE,
            flyout_anchor_x: 0,
            flyout_anchor_y: 0,
            tb_ctx_open: false,
            tb_ctx_sel: 0,
            tb_ctx_x: 0,
            tb_ctx_y: 0,
            win32_desktop: DesktopObject::new(),
            win32: session_win32::Win32ShellState::default(),
        };
        hal.debug_write(b"nt10-session: redraw + software cursor\r\n");
        session_win32::init_uefi_win32(&mut s, hal);
        s.update_clock_strings();
        s.refresh_desktop();
        hal.debug_write(b"nt10-session: new_uefi complete\r\n");
        s
    }

    #[cfg(target_arch = "x86_64")]
    fn maybe_bringup_xhci_after_ps2<H: Hal + ?Sized>(&mut self, hal: &H) {
        self.poll_tick = self.poll_tick.wrapping_add(1);
        if self.xhci_probe_attempted {
            return;
        }
        if SKIP_XHCI_INIT {
            self.xhci_probe_attempted = true;
            hal.debug_write(b"nt10-session: SKIP_XHCI_INIT=true - USB xHCI not started\r\n");
            return;
        }
        if self.poll_tick <= XHCI_INIT_AFTER_POLLS {
            return;
        }
        self.xhci_probe_attempted = true;
        hal.debug_write(b"nt10-session: xHCI HID init starting (after PS/2 slice)\r\n");
        self.xhci = unsafe { xhci::xhci_init_hid().ok() };
        match &self.xhci {
            Some(xh) => {
                hal.debug_write(b"nt10-kernel: xHCI HID ready\r\n");
                if xh.mouse_iface.is_some() {
                    hal.debug_write(b"nt10-mouse: USB HID pointer iface OK (QEMU tablet uses protocol 0)\r\n");
                } else {
                    hal.debug_write(b"nt10-mouse: USB xHCI up but no HID pointer interface\r\n");
                }
            }
            None => {
                hal.debug_write(b"nt10-kernel: xHCI HID init failed (PS/2 may still work)\r\n");
            }
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    fn maybe_bringup_xhci_after_ps2<H: Hal + ?Sized>(&mut self, _hal: &H) {}

    fn chrome_state(&self) -> DesktopChromeState {
        let (win_stack_len, win_stack) = stack_to_bytes(&self.stack);
        DesktopChromeState {
            menu_open: self.menu_open,
            menu_sel: self.menu_sel,
            ctx_open: self.ctx_open,
            ctx_sel: self.ctx_sel,
            ctx_x: self.ctx_x,
            ctx_y: self.ctx_y,
            win_stack_len,
            win_stack,
            power_confirm_open: self.power_confirm_open,
            clock_time: self.clock_time,
            clock_time_n: self.clock_time_n,
            clock_date: self.clock_date,
            clock_date_n: self.clock_date_n,
            flyout: self.flyout,
            flyout_x: self.flyout_anchor_x,
            flyout_y: self.flyout_anchor_y,
            tb_ctx_open: self.tb_ctx_open,
            tb_ctx_sel: self.tb_ctx_sel,
            tb_ctx_x: self.tb_ctx_x,
            tb_ctx_y: self.tb_ctx_y,
        }
    }

    fn update_clock_strings(&mut self) {
        let rtc = wall_clock::try_read_rtc();
        let (tn, dn) = wall_clock::format_clock_lines(
            rtc,
            self.uptime_secs,
            &mut self.clock_time,
            &mut self.clock_date,
        );
        self.clock_time_n = (tn.min(self.clock_time.len())) as u8;
        self.clock_date_n = (dn.min(self.clock_date.len())) as u8;
    }

    fn advance_uptime_if_no_rtc(&mut self) {
        if wall_clock::try_read_rtc().is_some() {
            return;
        }
        if self
            .poll_seq
            .wrapping_sub(self.last_uptime_adv_poll)
            >= 48_000
        {
            self.last_uptime_adv_poll = self.poll_seq;
            self.uptime_secs = self.uptime_secs.wrapping_add(1);
        }
    }

    fn ensure_base_cache(&mut self) {
        // Win32 wallpaper HWND owns the resource wallpaper; shell must not duplicate it in the
        // tight base cache (see `refresh_desktop` / `redraw_uefi_desktop_skip_wallpaper`).
        if self.win32.desktop_ready {
            self.base_cache_active = false;
            return;
        }
        let w = self.fb.horizontal_resolution;
        let h = self.fb.vertical_resolution;
        let Some(need) = shell::desktop_base_layer_byte_len(w, h) else {
            self.base_cache_active = false;
            return;
        };
        if need > shell::DESKTOP_BASE_LAYER_CAP_BYTES {
            self.base_cache_active = false;
            return;
        }
        if self.base_cache_active && self.base_cache_w == w && self.base_cache_h == h {
            return;
        }
        let ok = unsafe {
            let sl = &mut DESKTOP_BASE_CACHE[..need];
            shell::rebuild_desktop_base_layer(sl, w, h, self.fb.pixel_format, &self.layout)
        };
        if ok {
            self.base_cache_active = true;
            self.base_cache_w = w;
            self.base_cache_h = h;
        } else {
            self.base_cache_active = false;
        }
    }

    pub(crate) fn refresh_desktop(&mut self) {
        session_win32::pump_uefi_win32(self);
        // Full scene repaint: pointer is last layer.
        // - No Win32 desktop: cached wallpaper+shortcuts when it fits [`shell::DESKTOP_BASE_LAYER_CAP_BYTES`].
        // - Win32 desktop: wallpaper from `resources` is painted only in the bottom HWND; shell draws
        //   shortcuts + chrome on top, then remaining Win32 layers composite above (Phase 5).
        self.update_clock_strings();
        let w = self.fb.horizontal_resolution;
        let h = self.fb.vertical_resolution;
        self.dwm.mark_dirty(DirtyRect {
            x0: 0,
            y0: 0,
            x1: w,
            y1: h,
        });
        self.ensure_base_cache();
        let st = self.chrome_state();
        let win32 = self.win32.desktop_ready;
        if win32 {
            let cap = display_mgr::framebuffer_linear_byte_cap(&self.fb);
            unsafe {
                let buf = core::slice::from_raw_parts_mut(self.fb.base as *mut u8, cap);
                let _ = session_win32::composite_win32_wallpaper_only_to_buffer(self, buf);
            }
            shell::redraw_uefi_desktop_skip_wallpaper(
                &self.fb,
                &self.layout,
                &st,
                &self.stack,
                &self.host_ui,
            );
        } else if self.base_cache_active {
            let need = shell::desktop_base_layer_byte_len(self.base_cache_w, self.base_cache_h)
                .unwrap_or(0);
            unsafe {
                let slice = &DESKTOP_BASE_CACHE[..need];
                shell::redraw_uefi_desktop_from_base_cache(
                    &self.fb,
                    slice,
                    self.base_cache_w,
                    self.base_cache_h,
                    &self.layout,
                    &st,
                    &self.stack,
                    &self.host_ui,
                );
            }
        } else {
            shell::redraw_uefi_desktop(
                &self.fb,
                &self.layout,
                &st,
                &self.stack,
                &self.host_ui,
            );
        }
        session_win32::pump_uefi_win32(self);
        if win32 {
            session_win32::composite_win32_above_wallpaper_to_gop(self);
        } else {
            session_win32::composite_win32_to_gop(self);
        }
        self.pointer_capture_under();
        self.pointer_paint_on_fb();
    }

    fn activate_tb_ctx_item<H: Hal + ?Sized>(&mut self, hal: &H) {
        match self.tb_ctx_sel {
            0 => {
                self.open_app(AppId::TaskMgr);
                hal.debug_write(b"nt10: taskbar menu > Task Manager\r\n");
            }
            1 => {
                hal.debug_write(b"nt10: taskbar menu > Cascade (stub)\r\n");
            }
            2 => {
                self.stack.clear();
                hal.debug_write(b"nt10: taskbar menu > Show desktop\r\n");
            }
            3 => {
                hal.debug_write(b"nt10: taskbar menu > Taskbar settings (stub)\r\n");
            }
            _ => {}
        }
        self.tb_ctx_open = false;
        self.refresh_desktop();
    }

    fn open_app(&mut self, id: AppId) {
        self.stack.push_front(id);
        if id == AppId::Files {
            self.host_ui.files.refresh_from_vfs();
        }
    }

    pub fn poll<H: Hal + ?Sized>(&mut self, hal: &H) {
        self.poll_seq = self.poll_seq.wrapping_add(1);
        let mut r = [0u8; 16];
        unsafe {
            while let Some((aux, b)) = i8042::try_read() {
                if !aux {
                    if let Some(ev) = self.ps2_kbd.feed(b) {
                        self.input.push_key(ev);
                    }
                } else if let Some(p) = self.ps2_mouse.feed_aux_byte(b) {
                    self.apply_ps2_mouse(&p, hal);
                }
            }
        }
        self.maybe_bringup_xhci_after_ps2(hal);
        #[cfg(target_arch = "x86_64")]
        if let Some(ref xh) = self.xhci {
            let n = unsafe { xhci::xhci_poll_hid(xh, &mut r) }.unwrap_or(0);
            if n > 0 {
                self.apply_usb_report(&r[..n], hal);
            }
        }
        while let Some(ev) = self.input.pop_key() {
            self.handle_key(ev, hal);
        }
        self.advance_uptime_if_no_rtc();
        if self.poll_seq.wrapping_rem(512) == 0 {
            if let Some(r) = wall_clock::try_read_rtc() {
                if r.second != self.last_rtc_second {
                    self.last_rtc_second = r.second;
                    self.update_clock_strings();
                    self.refresh_desktop();
                }
            }
        }
        session_win32::maybe_post_timer(self);
        session_win32::pump_uefi_win32(self);
        self.pointer_remove_from_fb();
        session_win32::composite_win32_to_gop(self);
        self.pointer_capture_under();
        self.pointer_paint_on_fb();
    }

    #[cfg(target_arch = "x86_64")]
    fn apply_usb_report<H: Hal + ?Sized>(&mut self, report: &[u8], hal: &H) {
        let Some(ref xh) = self.xhci else {
            return;
        };
        if xh.keyboard_iface.is_some() && report.len() >= 8 {
            let mods = report[0];
            self.usb_shift_down = (mods & 0x03) != 0;
            let gui = (mods & 0x08) != 0;
            if gui && !self.prev_usb_gui {
                self.input.push_key(KeyEvent {
                    code: vkey::VK_WIN,
                    down: true,
                });
            }
            if !gui && self.prev_usb_gui {
                self.input.push_key(KeyEvent {
                    code: vkey::VK_WIN,
                    down: false,
                });
            }
            self.prev_usb_gui = gui;
            let mut tmp = [KeyEvent::default(); 8];
            let n = usb_hid::boot_keyboard_fill(&mut tmp, report);
            for i in 0..n {
                self.input.push_key(tmp[i]);
            }
        }
        if xh.mouse_iface.is_some() {
            let sw = self.fb.horizontal_resolution;
            let sh = self.fb.vertical_resolution;
            if let Some((b, px, py)) = usb_hid::qemu_usb_tablet_pointer(report, sw, sh) {
                let moved = px != self.cx || py != self.cy;
                self.pointer_set_absolute(b, px, py, hal, MouseSource::Usb, moved);
            } else if let Some(pe) = usb_hid::boot_mouse_report(report) {
                self.move_pointer(pe, hal, MouseSource::Usb);
            }
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    fn apply_usb_report<H: Hal + ?Sized>(&mut self, _report: &[u8], _hal: &H) {}

    fn apply_ps2_mouse<H: Hal + ?Sized>(&mut self, p: &Ps2MousePacket, hal: &H) {
        let b = sanitize_ps2_mouse_buttons(
            p.buttons,
            p.dx,
            p.dy,
            self.ptr_left_down,
            self.ptr_right_down,
        );
        self.move_pointer(
            PointerEvent {
                dx: p.dx,
                dy: p.dy,
                buttons: b,
            },
            hal,
            MouseSource::Ps2,
        );
    }

    fn handle_key<H: Hal + ?Sized>(&mut self, ev: KeyEvent, hal: &H) {
        if !ev.down {
            return;
        }
        let max_menu = START_MENU_ROW_COUNT.saturating_sub(1);
        let nctx = shell::CONTEXT_MENU_ROWS;

        if self.power_confirm_open {
            if ev.code == vkey::VK_ESC {
                self.power_confirm_open = false;
                self.refresh_desktop();
            }
            return;
        }

        let ntbar = shell::TASKBAR_CTX_MENU_ROWS;
        if self.tb_ctx_open {
            match ev.code {
                vkey::VK_ESC => {
                    self.tb_ctx_open = false;
                    self.refresh_desktop();
                }
                vkey::VK_TAB => {
                    if self.usb_shift_down {
                        self.tb_ctx_sel = (self.tb_ctx_sel + ntbar - 1) % ntbar;
                    } else {
                        self.tb_ctx_sel = (self.tb_ctx_sel + 1) % ntbar;
                    }
                    self.refresh_desktop();
                }
                vkey::VK_ENTER => self.activate_tb_ctx_item(hal),
                _ => {}
            }
            return;
        }

        if self.flyout != shell::FLYOUT_KIND_NONE {
            if ev.code == vkey::VK_ESC {
                self.flyout = shell::FLYOUT_KIND_NONE;
                self.refresh_desktop();
            }
            return;
        }

        if self.ctx_open {
            match ev.code {
                vkey::VK_ESC => {
                    self.ctx_open = false;
                    self.refresh_desktop();
                }
                vkey::VK_TAB => {
                    if self.usb_shift_down {
                        self.ctx_sel = (self.ctx_sel + nctx - 1) % nctx;
                    } else {
                        self.ctx_sel = (self.ctx_sel + 1) % nctx;
                    }
                    self.refresh_desktop();
                }
                vkey::VK_ENTER => self.activate_ctx_item(hal),
                _ => {}
            }
            return;
        }
        if self.menu_open {
            match ev.code {
                vkey::VK_ESC => {
                    self.menu_open = false;
                    self.refresh_desktop();
                }
                vkey::VK_UP => {
                    self.menu_sel = self.menu_sel.saturating_sub(1);
                    self.refresh_desktop();
                }
                vkey::VK_DOWN => {
                    self.menu_sel = (self.menu_sel + 1).min(max_menu);
                    self.refresh_desktop();
                }
                vkey::VK_TAB => {
                    if self.usb_shift_down {
                        self.menu_sel = (self.menu_sel + START_MENU_ROW_COUNT - 1) % START_MENU_ROW_COUNT;
                    } else {
                        self.menu_sel = (self.menu_sel + 1) % START_MENU_ROW_COUNT;
                    }
                    self.refresh_desktop();
                }
                vkey::VK_ENTER => {
                    self.activate_menu_item(self.menu_sel, hal);
                }
                _ => {}
            }
            return;
        }
        if ev.code == vkey::VK_WIN {
            self.menu_open = true;
            self.menu_sel = 0;
            self.flyout = shell::FLYOUT_KIND_NONE;
            self.tb_ctx_open = false;
            self.refresh_desktop();
            return;
        }
        if let Some(top_id) = self.stack.top() {
            if self.handle_top_window_key(top_id, ev, hal) {
                return;
            }
        }
    }

    /// `true` if the key was consumed by the focused hosted app.
    fn handle_top_window_key<H: Hal + ?Sized>(
        &mut self,
        id: AppId,
        ev: KeyEvent,
        hal: &H,
    ) -> bool {
        match id {
            AppId::Files => {
                if ev.code == vkey::VK_ESC {
                    let _ = self.stack.pop_top();
                    self.refresh_desktop();
                    return true;
                }
                let n = self.host_ui.files.row_count.max(1);
                let max_r = n.saturating_sub(1);
                match ev.code {
                    vkey::VK_UP => {
                        self.host_ui.files.sel = self.host_ui.files.sel.saturating_sub(1);
                        self.refresh_desktop();
                        true
                    }
                    vkey::VK_DOWN => {
                        self.host_ui.files.sel = (self.host_ui.files.sel + 1).min(max_r);
                        self.refresh_desktop();
                        true
                    }
                    vkey::VK_TAB => {
                        if self.usb_shift_down {
                            self.host_ui.files.sel =
                                (self.host_ui.files.sel + max_r) % (max_r + 1).max(1);
                        } else {
                            self.host_ui.files.sel = (self.host_ui.files.sel + 1) % (max_r + 1).max(1);
                        }
                        self.refresh_desktop();
                        true
                    }
                    _ => false,
                }
            }
            AppId::TaskMgr => {
                if ev.code == vkey::VK_ESC {
                    let _ = self.stack.pop_top();
                    self.refresh_desktop();
                    return true;
                }
                if ev.code == vkey::VK_TAB {
                    self.host_ui.taskmgr.tab ^= 1;
                    self.refresh_desktop();
                    return true;
                }
                if self.host_ui.taskmgr.tab == 0 {
                    let mut stubs = [None::<crate::ke::sched::ThreadStub>; 8];
                    let n = crate::ke::sched::rr_ready_snapshot(&mut stubs);
                    let max_r = n.saturating_sub(1).max(0);
                    match ev.code {
                        vkey::VK_UP => {
                            self.host_ui.taskmgr.sel = self.host_ui.taskmgr.sel.saturating_sub(1);
                            self.refresh_desktop();
                            true
                        }
                        vkey::VK_DOWN => {
                            self.host_ui.taskmgr.sel = (self.host_ui.taskmgr.sel + 1).min(max_r);
                            self.refresh_desktop();
                            true
                        }
                        _ => false,
                    }
                } else {
                    false
                }
            }
            AppId::Settings => {
                if ev.code == vkey::VK_ESC {
                    let _ = self.stack.pop_top();
                    self.refresh_desktop();
                    return true;
                }
                match ev.code {
                    vkey::VK_UP => {
                        self.host_ui.settings.cat = self.host_ui.settings.cat.saturating_sub(1);
                        self.refresh_desktop();
                        true
                    }
                    vkey::VK_DOWN => {
                        self.host_ui.settings.cat = (self.host_ui.settings.cat + 1).min(5);
                        self.refresh_desktop();
                        true
                    }
                    _ => false,
                }
            }
            AppId::ControlPanel => {
                if self.host_ui.control.open_sub.is_some() && ev.code == vkey::VK_ESC {
                    self.host_ui.control.open_sub = None;
                    self.refresh_desktop();
                    return true;
                }
                if ev.code == vkey::VK_ESC {
                    let _ = self.stack.pop_top();
                    self.refresh_desktop();
                    return true;
                }
                false
            }
            AppId::Run => {
                if ev.code == vkey::VK_ESC {
                    let _ = self.stack.pop_top();
                    self.refresh_desktop();
                    return true;
                }
                if ev.code == vkey::VK_ENTER {
                    hal.debug_write(b"nt10: Run > OK (command buffer stub)\r\n");
                    let _ = self.stack.pop_top();
                    self.refresh_desktop();
                    return true;
                }
                if let Some(ch) = scancode_printable(ev.code, self.usb_shift_down) {
                    let l = self.host_ui.run.len;
                    if l < self.host_ui.run.buf.len() {
                        self.host_ui.run.buf[l] = ch;
                        self.host_ui.run.len += 1;
                        self.refresh_desktop();
                    }
                    return true;
                }
                if ev.code == 0x0Eu8 && self.host_ui.run.len > 0 {
                    self.host_ui.run.len -= 1;
                    self.refresh_desktop();
                    return true;
                }
                false
            }
            AppId::Notepad => {
                if ev.code == vkey::VK_ESC {
                    let _ = self.stack.pop_top();
                    self.refresh_desktop();
                    return true;
                }
                if ev.code == vkey::VK_ENTER {
                    let l = self.host_ui.notepad.len;
                    if l < self.host_ui.notepad.buf.len() {
                        self.host_ui.notepad.buf[l] = b'\n';
                        self.host_ui.notepad.len += 1;
                        self.refresh_desktop();
                    }
                    return true;
                }
                if let Some(ch) = scancode_printable(ev.code, self.usb_shift_down) {
                    let l = self.host_ui.notepad.len;
                    if l < self.host_ui.notepad.buf.len() {
                        self.host_ui.notepad.buf[l] = ch;
                        self.host_ui.notepad.len += 1;
                        self.refresh_desktop();
                    }
                    return true;
                }
                if ev.code == 0x0Eu8 && self.host_ui.notepad.len > 0 {
                    self.host_ui.notepad.len -= 1;
                    self.refresh_desktop();
                    return true;
                }
                false
            }
            AppId::Calculator => {
                if ev.code == vkey::VK_ESC {
                    let _ = self.stack.pop_top();
                    self.refresh_desktop();
                    return true;
                }
                if self.handle_calc_scancode(ev.code, hal) {
                    return true;
                }
                false
            }
            AppId::About | AppId::Properties => {
                if ev.code == vkey::VK_ESC {
                    let _ = self.stack.pop_top();
                    self.refresh_desktop();
                    return true;
                }
                false
            }
        }
    }

    fn handle_calc_scancode<H: Hal + ?Sized>(&mut self, code: u8, hal: &H) -> bool {
        let d = match code {
            0x0B => Some(0i64),
            0x02 => Some(1),
            0x03 => Some(2),
            0x04 => Some(3),
            0x05 => Some(4),
            0x06 => Some(5),
            0x07 => Some(6),
            0x08 => Some(7),
            0x09 => Some(8),
            0x0A => Some(9),
            _ => None,
        };
        if let Some(d) = d {
            let c = &mut self.host_ui.calculator;
            if c.fresh {
                c.entry = d;
                c.fresh = false;
            } else {
                c.entry = c.entry.saturating_mul(10).saturating_add(d);
            }
            self.refresh_desktop();
            return true;
        }
        match code {
            0x0D => {
                self.host_ui.calculator.op = 3;
                self.host_ui.calculator.acc = self.host_ui.calculator.entry;
                self.host_ui.calculator.fresh = true;
                hal.debug_write(b"nt10: calc /\r\n");
                self.refresh_desktop();
                true
            }
            0x0C => {
                self.host_ui.calculator.op = 2;
                self.host_ui.calculator.acc = self.host_ui.calculator.entry;
                self.host_ui.calculator.fresh = true;
                self.refresh_desktop();
                true
            }
            0x0E => {
                self.host_ui.calculator.op = 1;
                self.host_ui.calculator.acc = self.host_ui.calculator.entry;
                self.host_ui.calculator.fresh = true;
                self.refresh_desktop();
                true
            }
            0x4A => {
                self.host_ui.calculator.op = 0;
                self.host_ui.calculator.acc = self.host_ui.calculator.entry;
                self.host_ui.calculator.fresh = true;
                self.refresh_desktop();
                true
            }
            0x1C => {
                let c = &mut self.host_ui.calculator;
                let res = match c.op {
                    0 => c.acc.saturating_add(c.entry),
                    1 => c.acc.saturating_sub(c.entry),
                    2 => c.acc.saturating_mul(c.entry),
                    3 => {
                        if c.entry == 0 {
                            c.acc
                        } else {
                            c.acc / c.entry
                        }
                    }
                    _ => c.entry,
                };
                c.entry = res;
                c.fresh = true;
                self.refresh_desktop();
                true
            }
            0x01 | 0x2Eu8 => {
                self.host_ui.calculator = crate::desktop::fluent::hosted_apps::CalcState::default();
                self.refresh_desktop();
                true
            }
            _ => false,
        }
    }

    fn activate_ctx_item<H: Hal + ?Sized>(&mut self, hal: &H) {
        match self.ctx_sel {
            0 => {
                if self.ctx_from_files {
                    self.open_app(AppId::Files);
                } else if let Some(slot) = shell::desktop_shortcut_hit(self.cx, self.cy, &self.layout) {
                    match slot {
                        0 | 3 => self.open_app(AppId::Files),
                        1 => self.open_app(AppId::Files),
                        2 => hal.debug_write(b"nt10: Open Recycle (stub)\r\n"),
                        _ => self.open_app(AppId::Files),
                    }
                } else {
                    self.open_app(AppId::Files);
                }
            }
            1 => {
                self.host_ui.properties.target_line = if self.ctx_from_files {
                    self.host_ui.files.sel
                } else {
                    0
                };
                self.open_app(AppId::Properties);
            }
            2 => {
                let _ = self.stack.pop_top();
            }
            _ => {}
        }
        self.ctx_open = false;
        self.refresh_desktop();
    }

    fn activate_menu_item<H: Hal + ?Sized>(&mut self, idx: usize, hal: &H) {
        match idx {
            0 => hal.debug_write(b"nt10: Start > Terminal (stub)\r\n"),
            1 => {
                self.open_app(AppId::Files);
                hal.debug_write(b"nt10: Start > Files\r\n");
            }
            2 => {
                self.open_app(AppId::TaskMgr);
                hal.debug_write(b"nt10: Start > Task Manager\r\n");
            }
            3 => {
                self.open_app(AppId::Settings);
                hal.debug_write(b"nt10: Start > Settings\r\n");
            }
            4 => {
                self.open_app(AppId::ControlPanel);
                hal.debug_write(b"nt10: Start > Control Panel\r\n");
            }
            5 => {
                self.open_app(AppId::Run);
                hal.debug_write(b"nt10: Start > Run\r\n");
            }
            6 => {
                self.open_app(AppId::Notepad);
                hal.debug_write(b"nt10: Start > Notepad\r\n");
            }
            7 => {
                self.open_app(AppId::Calculator);
                hal.debug_write(b"nt10: Start > Calculator\r\n");
            }
            8 => {
                self.open_app(AppId::Files);
                hal.debug_write(b"nt10: Start > Documents (opens Files)\r\n");
            }
            9 => {
                self.open_app(AppId::About);
                hal.debug_write(b"nt10: Start > About\r\n");
            }
            10 => {
                self.power_confirm_open = true;
                hal.debug_write(b"nt10: Start > Power (stub)\r\n");
            }
            _ => {}
        }
        self.menu_open = false;
        self.refresh_desktop();
    }

    fn on_left_down<H: Hal + ?Sized>(&mut self, hal: &H) {
        let px = self.cx;
        let py = self.cy;
        let sw = self.fb.horizontal_resolution;
        let sh = self.fb.vertical_resolution;

        if session_win32::handle_left_down(self, hal) {
            return;
        }

        if self.power_confirm_open {
            self.power_confirm_open = false;
            self.refresh_desktop();
            return;
        }

        if self.tb_ctx_open {
            let sw = self.fb.horizontal_resolution;
            let sh = self.fb.vertical_resolution;
            if let Some(row) = shell::taskbar_ctx_hit_row(
                px,
                py,
                self.tb_ctx_x,
                self.tb_ctx_y,
                sw,
                sh,
            ) {
                self.tb_ctx_sel = row;
                self.activate_tb_ctx_item(hal);
                return;
            }
            if shell::taskbar_ctx_panel_contains(px, py, self.tb_ctx_x, self.tb_ctx_y, sw, sh) {
                return;
            }
            self.tb_ctx_open = false;
            self.refresh_desktop();
            return;
        }

        if self.flyout != shell::FLYOUT_KIND_NONE {
            let sw = self.fb.horizontal_resolution;
            let sh = self.fb.vertical_resolution;
            if shell::flyout_panel_contains(
                self.flyout,
                self.flyout_anchor_x,
                self.flyout_anchor_y,
                px,
                py,
                sw,
                sh,
            ) {
                return;
            }
            self.flyout = shell::FLYOUT_KIND_NONE;
            self.refresh_desktop();
            return;
        }

        if self.ctx_open {
            if let Some(row) =
                shell::context_menu_hit_row(px, py, self.ctx_x, self.ctx_y, sw, sh)
            {
                self.ctx_sel = row;
                self.activate_ctx_item(hal);
                return;
            }
            if shell::context_menu_panel_contains(px, py, self.ctx_x, self.ctx_y, sw, sh) {
                return;
            }
            self.ctx_open = false;
            self.refresh_desktop();
            return;
        }

        if self.menu_open {
            let sm = self.layout.start_menu();
            if !sm.panel.contains(px, py) {
                self.menu_open = false;
                self.refresh_desktop();
                return;
            }
            for i in 0..START_MENU_ROW_COUNT {
                if sm.items[i].contains(px, py) {
                    self.activate_menu_item(i, hal);
                    return;
                }
            }
            return;
        }

        if let Some((depth, id)) = hosted_apps::hit_window_at(px, py, &self.layout, &self.stack) {
            let el = explorer_view::layout_for_stack_depth(&self.layout, depth);
            if hosted_apps::hit_close(&el, px, py) {
                self.stack.remove(id);
                self.refresh_desktop();
                return;
            }
            self.stack.push_front(id);
            match id {
                AppId::Files => {
                    if let Some(row) =
                        hosted_apps::hit_files_row(&el, px, py, self.host_ui.files.row_count)
                    {
                        self.host_ui.files.sel = row;
                        self.refresh_desktop();
                        return;
                    }
                }
                AppId::TaskMgr => {
                    if let Some(t) = hosted_apps::hit_taskmgr_tab(&el, px, py) {
                        self.host_ui.taskmgr.tab = t;
                        self.refresh_desktop();
                        return;
                    }
                    if self.host_ui.taskmgr.tab == 0 {
                        let mut stubs = [None::<crate::ke::sched::ThreadStub>; 8];
                        let n = crate::ke::sched::rr_ready_snapshot(&mut stubs);
                        if let Some(r) = hosted_apps::hit_taskmgr_row(&el, px, py, n) {
                            self.host_ui.taskmgr.sel = r;
                            self.refresh_desktop();
                            return;
                        }
                    }
                }
                AppId::Settings => {
                    if let Some(c) = hosted_apps::hit_settings_cat(&el, px, py) {
                        self.host_ui.settings.cat = c;
                        self.refresh_desktop();
                        return;
                    }
                    if let Some(t) = hosted_apps::hit_settings_toggle(&el, px, py) {
                        if let Some(x) = self.host_ui.settings.toggles.get_mut(t) {
                            *x = !*x;
                        }
                        self.refresh_desktop();
                        return;
                    }
                }
                AppId::ControlPanel => {
                    if self.host_ui.control.open_sub.is_none() {
                        if let Some(tile) = hosted_apps::hit_controlpanel_tile(&el, px, py) {
                            self.host_ui.control.sel = tile;
                            self.host_ui.control.open_sub = Some(tile as u8);
                            self.refresh_desktop();
                            return;
                        }
                    }
                }
                AppId::Run => {
                    if let Some(ok) = hosted_apps::hit_run_ok_cancel(&el, px, py) {
                        if ok {
                            hal.debug_write(b"nt10: Run OK (mouse)\r\n");
                        }
                        let _ = self.stack.pop_top();
                        self.refresh_desktop();
                        return;
                    }
                }
                _ => {}
            }
            self.refresh_desktop();
            return;
        }

        let slots = self.layout.task_slots();
        for s in 0..TASK_SLOT_COUNT {
            if slots[s].contains(px, py) {
                if let Some(aid) = hosted_apps::task_slot_app(&self.stack, s) {
                    self.stack.push_front(aid);
                    self.refresh_desktop();
                }
                return;
            }
        }

        if shell::hit_test_show_desktop_corner(&self.layout, px, py) {
            self.stack.clear();
            self.flyout = shell::FLYOUT_KIND_NONE;
            self.refresh_desktop();
            hal.debug_write(b"nt10: Show desktop (corner)\r\n");
            return;
        }

        if shell::hit_test_clock_display(&self.layout, px, py) {
            self.flyout = if self.flyout == shell::FLYOUT_KIND_CALENDAR_STUB {
                shell::FLYOUT_KIND_NONE
            } else {
                shell::FLYOUT_KIND_CALENDAR_STUB
            };
            let c = self.layout.clock_display_area();
            self.flyout_anchor_x = c.x + c.w / 2;
            self.flyout_anchor_y = c.y;
            self.refresh_desktop();
            return;
        }

        if shell::hit_test_tray_volume(&self.layout, px, py) {
            self.flyout = if self.flyout == shell::FLYOUT_KIND_VOLUME_STUB {
                shell::FLYOUT_KIND_NONE
            } else {
                shell::FLYOUT_KIND_VOLUME_STUB
            };
            if let Some(r) = self.layout.tray_icon_rect(1) {
                self.flyout_anchor_x = r.x + r.w / 2;
                self.flyout_anchor_y = r.y;
            }
            self.refresh_desktop();
            return;
        }

        if self.layout.start_button.contains(px, py) {
            self.menu_open = true;
            self.menu_sel = 0;
            self.refresh_desktop();
            hal.debug_write(b"nt10-kernel: Start opened (mouse)\r\n");
            return;
        }

        if let Some(si) = shell::desktop_shortcut_hit(px, py, &self.layout) {
            let dbl = self.last_lclick_slot == Some(si as u8)
                && self.poll_seq.wrapping_sub(self.last_lclick_seq) < 48;
            self.last_lclick_seq = self.poll_seq;
            self.last_lclick_slot = Some(si as u8);
            if dbl {
                match si {
                    0 => self.open_app(AppId::Files),
                    1 => self.open_app(AppId::Files),
                    2 => hal.debug_write(b"nt10: desktop Recycle (stub)\r\n"),
                    3 => self.open_app(AppId::Files),
                    _ => {}
                }
                self.refresh_desktop();
            }
        }
    }

    fn on_right_down<H: Hal + ?Sized>(&mut self, hal: &H) {
        if !FIRST_RIGHT_BUTTON.swap(true, Ordering::Relaxed) {
            hal.debug_write(b"nt10-kernel: first right button\r\n");
        }
        let px = self.cx;
        let py = self.cy;
        let sw = self.fb.horizontal_resolution;
        let sh = self.fb.vertical_resolution;
        if self.menu_open {
            return;
        }
        if session_win32::handle_right_down(self) {
            return;
        }
        if self.ctx_open
            && shell::context_menu_panel_contains(px, py, self.ctx_x, self.ctx_y, sw, sh)
        {
            return;
        }
        if self.layout.bar.contains(px, py) {
            self.tb_ctx_open = true;
            self.tb_ctx_sel = 0;
            let mw = shell::TASKBAR_CTX_MENU_W;
            let mh = shell::TASKBAR_CTX_MENU_ROW_H * shell::TASKBAR_CTX_MENU_ROWS as u32;
            self.tb_ctx_x = px.min(sw.saturating_sub(mw));
            // Open upward from the click so rows are not off the bottom edge.
            self.tb_ctx_y = py.saturating_sub(mh).min(sh.saturating_sub(mh));
            self.ctx_open = false;
            self.flyout = shell::FLYOUT_KIND_NONE;
            self.refresh_desktop();
            return;
        }
        self.tb_ctx_open = false;
        self.ctx_sel = 0;
        self.ctx_x = px.min(sw.saturating_sub(shell::CONTEXT_MENU_W));
        let menu_h = shell::CONTEXT_MENU_ROW_H * shell::CONTEXT_MENU_ROWS as u32;
        let work_bottom = self.layout.bar.y;
        // Keep the menu in the work area above the taskbar so it is not clipped or covered.
        let max_y = work_bottom.saturating_sub(menu_h);
        self.ctx_y = if py <= max_y {
            py
        } else {
            max_y
        };
        self.ctx_y = self
            .ctx_y
            .min(sh.saturating_sub(menu_h));
        self.ctx_open = true;
        self.ctx_from_files = false;
        if let Some((depth, id)) = hosted_apps::hit_window_at(px, py, &self.layout, &self.stack) {
            if id == AppId::Files {
                let el = explorer_view::layout_for_stack_depth(&self.layout, depth);
                if let Some(i) =
                    hosted_apps::hit_files_row(&el, px, py, self.host_ui.files.row_count)
                {
                    self.host_ui.files.sel = i;
                    self.ctx_from_files = true;
                    hal.debug_write(b"nt10: context on Files row\r\n");
                }
            }
        }
        self.refresh_desktop();
    }

    /// USB tablet / touchscreen style absolute `(px, py)` in framebuffer pixels.
    /// `mark_first_motion`: set when the host reported movement intent (relative delta or absolute position change).
    fn pointer_set_absolute<H: Hal + ?Sized>(
        &mut self,
        buttons: u8,
        nx: u32,
        ny: u32,
        hal: &H,
        source: MouseSource,
        mark_first_motion: bool,
    ) {
        let w = self.fb.horizontal_resolution;
        let h = self.fb.vertical_resolution;
        if w == 0 || h == 0 {
            return;
        }
        let nx = nx.min(w.saturating_sub(1));
        let ny = ny.min(h.saturating_sub(1));
        let prev_l = self.ptr_left_down;
        let prev_r = self.ptr_right_down;
        let left = (buttons & 1) != 0;
        let right = (buttons & 2) != 0;
        let btn_changed = left != prev_l || right != prev_r;
        let moved = nx != self.cx || ny != self.cy;
        if mark_first_motion && !FIRST_POINTER_MOTION.swap(true, Ordering::Relaxed) {
            hal.debug_write(b"nt10-kernel: first pointer motion\r\n");
        }
        let click = left && !prev_l;
        let prev_stable = self.ptr_right_stable_reports;
        // USB HID tends to report clean edges; PS/2 may glitch — require more stable frames there.
        let stable_max: u8 = match source {
            MouseSource::Usb => 2,
            MouseSource::Ps2 => 4,
        };
        let stable = if right {
            (prev_stable + 1).min(stable_max)
        } else {
            0
        };
        let rclick_edge = right && stable == stable_max && prev_stable < stable_max;
        self.ptr_right_stable_reports = stable;
        let debounce_ok = self
            .poll_seq
            .wrapping_sub(self.last_rclick_poll_seq)
            >= RCLICK_DEBOUNCE_POLLS;
        let rclick = rclick_edge && debounce_ok;
        self.ptr_left_down = left;
        self.ptr_right_down = right;

        let ox = self.cx;
        let oy = self.cy;
        let p_log = PointerEvent {
            dx: (nx as i32 - ox as i32) as i16,
            dy: (ny as i32 - oy as i32) as i16,
            buttons,
        };
        // Avoid logging every poll while a phantom button is held: log moves only when no button pressed.
        let log_motion = moved && buttons == 0;
        if MOUSE_POINTER_SERIAL_DEBUG && (btn_changed || mark_first_motion || log_motion) {
            mouse_serial_debug_line(hal, source, &p_log, nx, ny, moved);
        }
        if moved {
            self.pointer_remove_from_fb();
            self.cx = nx;
            self.cy = ny;
            session_win32::handle_pointer_move(self);
            self.pointer_capture_under();
            self.pointer_paint_on_fb();
            let (osx, osy) = shell::pointer_sprite_top_left(ox, oy);
            let (nsx, nsy) = shell::pointer_sprite_top_left(self.cx, self.cy);
            self.dwm.mark_dirty(DirtyRect {
                x0: osx.min(nsx),
                y0: osy.min(nsy),
                x1: osx
                    .max(nsx)
                    .saturating_add(shell::POINTER_CURSOR_SIZE)
                    .min(w),
                y1: osy
                    .max(nsy)
                    .saturating_add(shell::POINTER_CURSOR_SIZE)
                    .min(h),
            });
        }
        if rclick {
            self.last_rclick_poll_seq = self.poll_seq;
            self.on_right_down(hal);
        }
        if click {
            self.on_left_down(hal);
        }
    }

    fn move_pointer<H: Hal + ?Sized>(&mut self, p: PointerEvent, hal: &H, source: MouseSource) {
        let w = self.fb.horizontal_resolution;
        let h = self.fb.vertical_resolution;
        if w == 0 || h == 0 {
            return;
        }
        let nx = (self.cx as i32 + p.dx as i32).clamp(0, w.saturating_sub(1) as i32) as u32;
        let ny = (self.cy as i32 + p.dy as i32).clamp(0, h.saturating_sub(1) as i32) as u32;
        let mark_fm = p.dx != 0 || p.dy != 0;
        self.pointer_set_absolute(p.buttons, nx, ny, hal, source, mark_fm);
    }

    #[inline]
    fn pointer_sprite_xy(&self) -> (u32, u32) {
        shell::pointer_sprite_top_left(self.cx, self.cy)
    }

    /// Restore framebuffer under the pointer sprite (before moving hotspot).
    fn pointer_remove_from_fb(&mut self) {
        let (sx, sy) = self.pointer_sprite_xy();
        let buf = unsafe {
            core::slice::from_raw_parts(
                core::ptr::addr_of_mut!(DESKTOP_CURSOR_SAVE).cast(),
                CURSOR_SAVE_BYTES,
            )
        };
        blit_buf_to_region(&self.fb, sx, sy, shell::POINTER_CURSOR_SIZE, buf);
    }

    /// Save pixels under the pointer sprite (scene already drawn; next: paint_on_fb).
    fn pointer_capture_under(&mut self) {
        let (sx, sy) = self.pointer_sprite_xy();
        let buf = unsafe {
            core::slice::from_raw_parts_mut(
                core::ptr::addr_of_mut!(DESKTOP_CURSOR_SAVE).cast(),
                CURSOR_SAVE_BYTES,
            )
        };
        blit_region_to_buf(&self.fb, sx, sy, shell::POINTER_CURSOR_SIZE, buf);
    }

    /// Top z-order: draw pointer sprite at [`Self::pointer_sprite_xy`].
    fn pointer_paint_on_fb(&self) {
        let (sx, sy) = self.pointer_sprite_xy();
        let fb = FramebufferInfo {
            base: self.fb.base,
            size: self.fb.size,
            horizontal_resolution: self.fb.horizontal_resolution,
            vertical_resolution: self.fb.vertical_resolution,
            pixels_per_scan_line: self.fb.pixels_per_scan_line,
            pixel_format: self.fb.pixel_format,
        };
        shell::paint_pointer_cursor(&fb, sx, sy);
    }
}

fn blit_region_to_buf(fb: &FramebufferInfo, x: u32, y: u32, side: u32, buf: &mut [u8]) {
    if fb.base == 0 || fb.size == 0 {
        return;
    }
    let need = (side as usize)
        .checked_mul(side as usize)
        .and_then(|n| n.checked_mul(4))
        .unwrap_or(0);
    if buf.len() < need {
        return;
    }
    let stride = fb.pixels_per_scan_line as usize * 4;
    let sw = fb.horizontal_resolution as usize;
    let sh = fb.vertical_resolution as usize;
    let cap = display_mgr::framebuffer_linear_byte_cap(fb);
    unsafe {
        let raw = core::slice::from_raw_parts(fb.base as *const u8, cap);
        for row in 0..side as usize {
            let yy = y as usize + row;
            if yy >= sh {
                break;
            }
            for col in 0..side as usize {
                let xx = x as usize + col;
                if xx >= sw {
                    break;
                }
                let o = yy * stride + xx * 4;
                let bo = (row * side as usize + col) * 4;
                if o + 4 <= raw.len() && bo + 4 <= buf.len() {
                    buf[bo..bo + 4].copy_from_slice(&raw[o..o + 4]);
                }
            }
        }
    }
}

fn blit_buf_to_region(fb: &FramebufferInfo, x: u32, y: u32, side: u32, buf: &[u8]) {
    if fb.base == 0 || fb.size == 0 {
        return;
    }
    let need = (side as usize)
        .checked_mul(side as usize)
        .and_then(|n| n.checked_mul(4))
        .unwrap_or(0);
    if buf.len() < need {
        return;
    }
    let stride = fb.pixels_per_scan_line as usize * 4;
    let sw = fb.horizontal_resolution as usize;
    let sh = fb.vertical_resolution as usize;
    let cap = display_mgr::framebuffer_linear_byte_cap(fb);
    unsafe {
        let raw = core::slice::from_raw_parts_mut(fb.base as *mut u8, cap);
        for row in 0..side as usize {
            let yy = y as usize + row;
            if yy >= sh {
                break;
            }
            for col in 0..side as usize {
                let xx = x as usize + col;
                if xx >= sw {
                    break;
                }
                let o = yy * stride + xx * 4;
                let bo = (row * side as usize + col) * 4;
                if o + 4 <= raw.len() && bo + 4 <= buf.len() {
                    raw[o..o + 4].copy_from_slice(&buf[bo..bo + 4]);
                }
            }
        }
    }
}

/// PS/2 set 1 make code → ASCII (subset for Run / Notepad).
fn scancode_printable(code: u8, shift: bool) -> Option<u8> {
    let conv = |lc: u8| {
        if shift {
            lc.to_ascii_uppercase()
        } else {
            lc
        }
    };
    match code {
        0x39 => Some(b' '),
        0x02..=0x0a => Some(b'1' + (code - 2)),
        0x0b => Some(b'0'),
        0x10 => Some(conv(b'q')),
        0x11 => Some(conv(b'w')),
        0x12 => Some(conv(b'e')),
        0x13 => Some(conv(b'r')),
        0x14 => Some(conv(b't')),
        0x15 => Some(conv(b'y')),
        0x16 => Some(conv(b'u')),
        0x17 => Some(conv(b'i')),
        0x18 => Some(conv(b'o')),
        0x19 => Some(conv(b'p')),
        0x1e => Some(conv(b'a')),
        0x1f => Some(conv(b's')),
        0x20 => Some(conv(b'd')),
        0x21 => Some(conv(b'f')),
        0x22 => Some(conv(b'g')),
        0x23 => Some(conv(b'h')),
        0x24 => Some(conv(b'j')),
        0x25 => Some(conv(b'k')),
        0x26 => Some(conv(b'l')),
        0x2c => Some(conv(b'z')),
        0x2d => Some(conv(b'x')),
        0x2e => Some(conv(b'c')),
        0x2f => Some(conv(b'v')),
        0x30 => Some(conv(b'b')),
        0x31 => Some(conv(b'n')),
        0x32 => Some(conv(b'm')),
        0x33 => Some(if shift { b'<' } else { b',' }),
        0x34 => Some(if shift { b'>' } else { b'.' }),
        0x35 => Some(if shift { b'?' } else { b'/' }),
        _ => None,
    }
}

#[derive(Clone, Copy)]
enum MouseSource {
    Ps2,
    Usb,
}

fn mouse_serial_debug_line<H: Hal + ?Sized>(
    hal: &H,
    src: MouseSource,
    p: &PointerEvent,
    cx: u32,
    cy: u32,
    pos_moved: bool,
) {
    hal.debug_write(b"nt10-mouse: ");
    hal.debug_write(match src {
        MouseSource::Ps2 => b"ps2",
        MouseSource::Usb => b"usb",
    });
    hal.debug_write(b" dx=");
    debug_write_i16(hal, p.dx);
    hal.debug_write(b" dy=");
    debug_write_i16(hal, p.dy);
    hal.debug_write(b" buttons=0x");
    debug_write_hex_u8(hal, p.buttons);
    hal.debug_write(b" pos=");
    debug_write_u32_dec(hal, cx);
    hal.debug_write(b",");
    debug_write_u32_dec(hal, cy);
    hal.debug_write(b" redraw=");
    hal.debug_write(if pos_moved { b"1" } else { b"0" });
    hal.debug_write(b"\r\n");
}

fn debug_write_hex_u8<H: Hal + ?Sized>(hal: &H, b: u8) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    hal.debug_write(&[HEX[(b >> 4) as usize], HEX[(b & 0xf) as usize]]);
}

fn debug_write_u32_dec<H: Hal + ?Sized>(hal: &H, mut n: u32) {
    let mut buf = [0u8; 12];
    let mut i = buf.len();
    if n == 0 {
        i -= 1;
        buf[i] = b'0';
    } else {
        while n > 0 && i > 0 {
            i -= 1;
            buf[i] = b'0' + (n % 10) as u8;
            n /= 10;
        }
    }
    hal.debug_write(&buf[i..]);
}

fn debug_write_usize_dec<H: Hal + ?Sized>(hal: &H, mut n: usize) {
    let mut buf = [0u8; 24];
    let mut i = buf.len();
    if n == 0 {
        i -= 1;
        buf[i] = b'0';
    } else {
        while n > 0 && i > 0 {
            i -= 1;
            buf[i] = b'0' + (n % 10) as u8;
            n /= 10;
        }
    }
    hal.debug_write(&buf[i..]);
}

fn debug_write_i16<H: Hal + ?Sized>(hal: &H, n: i16) {
    let v = n as i32;
    if v < 0 {
        hal.debug_write(b"-");
        debug_write_u32_dec(hal, (-v) as u32);
    } else {
        debug_write_u32_dec(hal, v as u32);
    }
}
