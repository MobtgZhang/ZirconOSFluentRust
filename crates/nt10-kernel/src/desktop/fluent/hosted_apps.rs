//! Per-app UI state and painting for kernel-hosted Fluent windows.
//!
//! **Ring3**: Replace `paint_*_client` with user-mode `CreateWindow` + GDI once csrss/user32 own the
//! framebuffer and message pump (`Loader-Win32k-Desktop.md` §5).

use nt10_boot_protocol::FramebufferInfo;

use super::app_host::{AppId, WindowStack};
use super::explorer_view::{self, ExplorerLayout, MAX_FILE_ROWS};
use super::shell;
use super::taskbar::{HitRect, TaskbarLayout};

use crate::ke::sched::{rr_ready_snapshot, ThreadStub, timer_quanta};

/// Files / This PC listing (`crate::fs::vfs::fill_desktop_file_list`).
#[derive(Clone, Debug)]
pub struct FilesState {
    pub sel: usize,
    pub row_count: usize,
    pub rows: [[u8; 80]; MAX_FILE_ROWS],
    pub row_lens: [usize; MAX_FILE_ROWS],
}

impl Default for FilesState {
    fn default() -> Self {
        Self {
            sel: 0,
            row_count: explorer_view::STATIC_ENTRY_COUNT,
            rows: [[0; 80]; MAX_FILE_ROWS],
            row_lens: [0; MAX_FILE_ROWS],
        }
    }
}

impl FilesState {
    pub fn refresh_from_vfs(&mut self) {
        crate::fs::vfs::fill_desktop_file_list(&mut self.rows, &mut self.row_lens, &mut self.row_count);
        self.sel = self.sel.min(self.row_count.saturating_sub(1).max(0));
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TaskMgrState {
    pub tab: u8,
    pub sel: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct SettingsState {
    pub cat: usize,
    pub toggles: [bool; 6],
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            cat: 0,
            toggles: [false, true, false, true, false, false],
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ControlPanelState {
    pub open_sub: Option<u8>,
    pub sel: usize,
}

#[derive(Clone, Debug)]
pub struct RunState {
    pub buf: [u8; 128],
    pub len: usize,
}

impl Default for RunState {
    fn default() -> Self {
        Self {
            buf: [0; 128],
            len: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct NotepadState {
    pub buf: [u8; 512],
    pub len: usize,
}

impl Default for NotepadState {
    fn default() -> Self {
        Self {
            buf: [0; 512],
            len: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CalcState {
    pub acc: i64,
    pub entry: i64,
    pub op: u8,
    pub fresh: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct AboutState {}

#[derive(Clone, Copy, Debug, Default)]
pub struct PropertiesState {
    pub target_line: usize,
}

/// All hosted-app state embedded in [`super::session::DesktopSession`].
#[derive(Clone, Debug)]
pub struct HostUiState {
    pub files: FilesState,
    pub taskmgr: TaskMgrState,
    pub settings: SettingsState,
    pub control: ControlPanelState,
    pub run: RunState,
    pub notepad: NotepadState,
    pub calculator: CalcState,
    pub about: AboutState,
    pub properties: PropertiesState,
}

impl Default for HostUiState {
    fn default() -> Self {
        let mut files = FilesState::default();
        files.refresh_from_vfs();
        Self {
            files,
            taskmgr: TaskMgrState::default(),
            settings: SettingsState::default(),
            control: ControlPanelState::default(),
            run: RunState::default(),
            notepad: NotepadState::default(),
            calculator: CalcState::default(),
            about: AboutState::default(),
            properties: PropertiesState::default(),
        }
    }
}

pub fn stack_from_bytes(len: u8, bytes: &[u8; 8]) -> WindowStack {
    let mut s = WindowStack::new();
    let n = (len as usize).min(bytes.len()).min(super::app_host::MAX_HOSTED_WINDOWS);
    for i in 0..n {
        s.ids[i] = AppId::from_u8(bytes[i]).unwrap_or(AppId::Files);
    }
    s.len = n as u8;
    s
}

pub fn stack_to_bytes(stack: &WindowStack) -> (u8, [u8; 8]) {
    let mut b = [0u8; 8];
    let n = stack.len as usize;
    for i in 0..n.min(8) {
        b[i] = stack.ids[i].to_u8();
    }
    (stack.len, b)
}

/// Paint every hosted window back-to-front.
pub fn paint_window_stack(
    fb: &FramebufferInfo,
    layout: &TaskbarLayout,
    stack: &WindowStack,
    ui: &HostUiState,
) {
    let n = stack.len as usize;
    for i in 0..n {
        let id = stack.ids[i];
        let focused = i + 1 == n;
        let el = explorer_view::layout_for_stack_depth(layout, i);
        match id {
            AppId::Files => {
                explorer_view::paint_window_chrome(fb, &el, id.title_idx(), focused);
                explorer_view::paint_files_client(
                    fb,
                    el.client,
                    ui.files.sel,
                    ui.files.row_count,
                    &ui.files.rows,
                    &ui.files.row_lens,
                );
            }
            AppId::TaskMgr => {
                explorer_view::paint_window_chrome(fb, &el, id.title_idx(), focused);
                paint_taskmgr_client(fb, &el, ui);
            }
            AppId::Settings => {
                explorer_view::paint_window_chrome(fb, &el, id.title_idx(), focused);
                paint_settings_client(fb, &el, ui);
            }
            AppId::ControlPanel => {
                explorer_view::paint_window_chrome(fb, &el, id.title_idx(), focused);
                paint_controlpanel_client(fb, &el, ui);
            }
            AppId::Run => {
                explorer_view::paint_window_chrome(fb, &el, id.title_idx(), focused);
                paint_run_client(fb, &el, ui);
            }
            AppId::Notepad => {
                explorer_view::paint_window_chrome(fb, &el, id.title_idx(), focused);
                paint_notepad_client(fb, &el, ui);
            }
            AppId::Calculator => {
                explorer_view::paint_window_chrome(fb, &el, id.title_idx(), focused);
                paint_calc_client(fb, &el, ui);
            }
            AppId::About => {
                explorer_view::paint_window_chrome(fb, &el, id.title_idx(), focused);
                paint_about_client(fb, &el);
            }
            AppId::Properties => {
                explorer_view::paint_window_chrome(fb, &el, id.title_idx(), focused);
                paint_properties_client(fb, &el, ui);
            }
        }
    }
}

fn paint_taskmgr_client(fb: &FramebufferInfo, el: &ExplorerLayout, ui: &HostUiState) {
    let cl = el.client;
    shell::fill_rect_bgra(fb, cl.x, cl.y, cl.w, cl.h, 0x24, 0x24, 0x2e);
    let tab_h = 28u32;
    shell::fill_rect_bgra(fb, cl.x, cl.y, cl.w, tab_h, 0x1e, 0x1e, 0x28);
    let t0 = ui.taskmgr.tab == 0;
    shell::fill_rect_bgra(
        fb,
        cl.x + 6,
        cl.y + 4,
        100,
        tab_h - 8,
        if t0 {
            0x00
        } else {
            0x40
        },
        if t0 {
            0x78
        } else {
            0x40
        },
        if t0 {
            0xd7
        } else {
            0x48
        },
    );
    shell::draw_ascii_line_clipped(
        fb,
        cl.x + 12,
        cl.y + 8,
        90,
        b"Processes",
        0xff,
        0xff,
        0xff,
        1,
    );
    shell::fill_rect_bgra(
        fb,
        cl.x + 112,
        cl.y + 4,
        110,
        tab_h - 8,
        if !t0 {
            0x00
        } else {
            0x40
        },
        if !t0 {
            0x78
        } else {
            0x40
        },
        if !t0 {
            0xd7
        } else {
            0x48
        },
    );
    shell::draw_ascii_line_clipped(
        fb,
        cl.x + 118,
        cl.y + 8,
        100,
        b"Performance",
        0xff,
        0xff,
        0xff,
        1,
    );

    let body_y = cl.y + tab_h + 6;
    if ui.taskmgr.tab == 0 {
        shell::draw_ascii_line_clipped(
            fb,
            cl.x + 8,
            body_y,
            cl.w - 16,
            b"Name          PID   State",
            0xc8,
            0xc8,
            0xd0,
            1,
        );
        let mut stubs: [Option<ThreadStub>; 8] = [None; 8];
        let n = rr_ready_snapshot(&mut stubs);
        let row = 22u32;
        for i in 0..n.min(8) {
            if let Some(st) = stubs[i] {
                let ry = body_y + 18 + (i as u32) * row;
                let sel = i == ui.taskmgr.sel.min(n.saturating_sub(1).max(0));
                let bg = if sel {
                    (0x00u8, 0x58u8, 0xa0u8)
                } else {
                    (0x32u8, 0x32u8, 0x3cu8)
                };
                shell::fill_rect_bgra(fb, cl.x + 4, ry, cl.w - 8, row - 2, bg.0, bg.1, bg.2);
                let mut line = [0u8; 48];
                let head = b"Thread ";
                line[..head.len()].copy_from_slice(head);
                let mut p = head.len();
                let mut tid = st.id.0;
                if tid == 0 {
                    line[p] = b'0';
                    p += 1;
                } else {
                    let mut tmp = [0u8; 12];
                    let mut k = 12usize;
                    while tid > 0 && k > 0 {
                        k -= 1;
                        tmp[k] = b'0' + (tid % 10) as u8;
                        tid /= 10;
                    }
                    for b in tmp[k..].iter() {
                        if p < line.len() {
                            line[p] = *b;
                            p += 1;
                        }
                    }
                }
                let tail = b"  Ready";
                for &b in tail {
                    if p < line.len() {
                        line[p] = b;
                        p += 1;
                    }
                }
                shell::draw_ascii_line_clipped(
                    fb,
                    cl.x + 10,
                    ry + 6,
                    cl.w - 20,
                    &line[..p],
                    0xee,
                    0xee,
                    0xf4,
                    1,
                );
            }
        }
        if n == 0 {
            shell::draw_ascii_line_clipped(
                fb,
                cl.x + 8,
                body_y + 24,
                cl.w - 16,
                b"(no RR threads - run non-UEFI timer path)",
                0x88,
                0x88,
                0x90,
                1,
            );
        }
    } else {
        let q = timer_quanta();
        let mut line = [0u8; 40];
        let prefix = b"Timer ticks: ";
        let mut p = 0usize;
        for &b in prefix {
            line[p] = b;
            p += 1;
        }
        let mut v = q;
        if v == 0 {
            line[p] = b'0';
            p += 1;
        } else {
            let mut tmp = [0u8; 12];
            let mut k = 12usize;
            while v > 0 && k > 0 {
                k -= 1;
                tmp[k] = b'0' + (v % 10) as u8;
                v /= 10;
            }
            for b in tmp[k..].iter() {
                line[p] = *b;
                p += 1;
            }
        }
        shell::draw_ascii_line_clipped(
            fb,
            cl.x + 12,
            body_y + 12,
            cl.w - 24,
            &line[..p],
            0xdd,
            0xdd,
            0xe6,
            1,
        );
        shell::fill_rect_bgra(
            fb,
            cl.x + 12,
            body_y + 40,
            (cl.w - 24).min(200),
            8,
            0x00,
            0x78,
            0xd7,
        );
    }
}

fn paint_settings_client(fb: &FramebufferInfo, el: &ExplorerLayout, ui: &HostUiState) {
    let cl = el.client;
    shell::fill_rect_bgra(fb, cl.x, cl.y, cl.w, cl.h, 0x26, 0x26, 0x30);
    let split = 140u32.min(cl.w / 3);
    shell::fill_rect_bgra(fb, cl.x, cl.y, split, cl.h, 0x1c, 0x1c, 0x26);
    let cats = [
        b"System" as &[u8],
        b"Devices",
        b"Personalization",
        b"Privacy",
        b"Update",
        b"Network",
    ];
    let row = 32u32;
    for (i, c) in cats.iter().enumerate() {
        let ry = cl.y + 8 + (i as u32) * row;
        let sel = i == ui.settings.cat;
        shell::fill_rect_bgra(
            fb,
            cl.x + 4,
            ry,
            split - 8,
            row - 4,
            if sel {
                0x00
            } else {
                0x2e
            },
            if sel {
                0x78
            } else {
                0x2e
            },
            if sel {
                0xd7
            } else {
                0x36
            },
        );
        shell::draw_ascii_line_clipped(
            fb,
            cl.x + 12,
            ry + 10,
            split - 20,
            c,
            0xf0,
            0xf0,
            0xf8,
            1,
        );
    }
    let rx = cl.x + split + 8;
    let rw = cl.w.saturating_sub(split + 16);
    shell::draw_ascii_line_clipped(
        fb,
        rx,
        cl.y + 12,
        rw,
        b"Category options (stub toggles)",
        0xd0,
        0xd0,
        0xd8,
        1,
    );
    for i in 0..4usize {
        let ry = cl.y + 40 + (i as u32) * 36;
        let on = ui.settings.toggles.get(i).copied().unwrap_or(false);
        shell::fill_rect_bgra(fb, rx, ry, rw, 30, 0x32, 0x32, 0x3c);
        shell::fill_rect_bgra(fb, rx + rw - 48, ry + 6, 36, 18, if on { 0x00 } else { 0x55 }, 0x78, 0xd7);
        let label = match i {
            0 => b"Night mode" as &[u8],
            1 => b"Bluetooth",
            2 => b"Notifications",
            _ => b"Location",
        };
        shell::draw_ascii_line_clipped(fb, rx + 8, ry + 8, rw - 60, label, 0xe8, 0xe8, 0xf0, 1);
    }
}

fn paint_controlpanel_client(fb: &FramebufferInfo, el: &ExplorerLayout, ui: &HostUiState) {
    let cl = el.client;
    shell::fill_rect_bgra(fb, cl.x, cl.y, cl.w, cl.h, 0x26, 0x26, 0x30);
    if let Some(sub) = ui.control.open_sub {
        shell::fill_rect_bgra(fb, cl.x + 8, cl.y + 8, cl.w - 16, cl.h - 16, 0x1a, 0x1a, 0x24);
        let msg = match sub {
            0 => b"Display settings (stub)" as &[u8],
            1 => b"Sound settings (stub)",
            2 => b"Network (stub)",
            _ => b"Programs (stub)",
        };
        shell::draw_ascii_line_clipped(fb, cl.x + 20, cl.y + 24, cl.w - 40, msg, 0xdd, 0xdd, 0xe6, 1);
        return;
    }
    let labels = [
        b"Display" as &[u8],
        b"Sound",
        b"Network",
        b"Programs",
    ];
    let gw = (cl.w - 40) / 2;
    let gh = (cl.h - 40) / 2;
    for i in 0..4usize {
        let col = (i % 2) as u32;
        let row = (i / 2) as u32;
        let gx = cl.x + 12 + col * (gw + 8);
        let gy = cl.y + 12 + row * (gh + 8);
        let sel = i == ui.control.sel;
        shell::fill_rect_bgra(
            fb,
            gx,
            gy,
            gw,
            gh,
            if sel {
                0x00
            } else {
                0x36
            },
            if sel {
                0x78
            } else {
                0x36
            },
            if sel {
                0xd7
            } else {
                0x40
            },
        );
        shell::draw_ascii_line_clipped(
            fb,
            gx + gw / 2 - 40,
            gy + gh / 2 - 4,
            gw - 8,
            labels[i],
            0xff,
            0xff,
            0xff,
            1,
        );
    }
}

fn run_button_rects(el: &ExplorerLayout) -> (HitRect, HitRect) {
    let cl = el.client;
    let bw = 72u32;
    let bh = 28u32;
    let y = cl.y + cl.h.saturating_sub(bh + 12);
    let ok_x = cl.x + cl.w.saturating_sub(bw * 2 + 24);
    let cancel_x = ok_x + bw + 8;
    let ok = HitRect {
        x: ok_x,
        y,
        w: bw,
        h: bh,
    };
    let cancel = HitRect {
        x: cancel_x,
        y,
        w: bw,
        h: bh,
    };
    (ok, cancel)
}

fn paint_run_client(fb: &FramebufferInfo, el: &ExplorerLayout, ui: &HostUiState) {
    let cl = el.client;
    shell::fill_rect_bgra(fb, cl.x, cl.y, cl.w, cl.h, 0x28, 0x28, 0x32);
    shell::draw_ascii_line_clipped(
        fb,
        cl.x + 12,
        cl.y + 16,
        cl.w - 24,
        b"Open:",
        0xcc,
        0xcc,
        0xd4,
        1,
    );
    let iy = cl.y + 40;
    shell::fill_rect_bgra(fb, cl.x + 12, iy, cl.w - 24, 28, 0x10, 0x10, 0x18);
    if ui.run.len > 0 {
        shell::draw_ascii_line_clipped(
            fb,
            cl.x + 16,
            iy + 8,
            cl.w - 32,
            &ui.run.buf[..ui.run.len.min(ui.run.buf.len())],
            0xee,
            0xee,
            0xf6,
            1,
        );
    }
    let (ok, cancel) = run_button_rects(el);
    shell::fill_rect_bgra(fb, ok.x, ok.y, ok.w, ok.h, 0x00, 0x78, 0xd7);
    shell::draw_ascii_line_clipped(fb, ok.x + 24, ok.y + 8, ok.w, b"OK", 0xff, 0xff, 0xff, 1);
    shell::fill_rect_bgra(fb, cancel.x, cancel.y, cancel.w, cancel.h, 0x44, 0x44, 0x50);
    shell::draw_ascii_line_clipped(
        fb,
        cancel.x + 12,
        cancel.y + 8,
        cancel.w,
        b"Cancel",
        0xee,
        0xee,
        0xf4,
        1,
    );
}

fn paint_notepad_client(fb: &FramebufferInfo, el: &ExplorerLayout, ui: &HostUiState) {
    let cl = el.client;
    shell::fill_rect_bgra(fb, cl.x, cl.y, cl.w, cl.h, 0x22, 0x22, 0x2c);
    let pad = 8u32;
    shell::fill_rect_bgra(fb, cl.x + pad, cl.y + pad, cl.w - 2 * pad, cl.h - 2 * pad, 0xfa, 0xfa, 0xfc);
    let max_c = ((cl.w.saturating_sub(2 * pad + 16)) / 9).max(4) as usize;
    let mut y = cl.y + pad + 8;
    let line_h = 10u32;
    let mut i = 0usize;
    let n = ui.notepad.len.min(ui.notepad.buf.len());
    let mut line_beg = 0usize;
    while i <= n && y < cl.y + cl.h - pad - line_h {
        let at_end = i == n;
        let nl = !at_end && ui.notepad.buf[i] == b'\n';
        let wrap = !at_end && !nl && i > line_beg && (i - line_beg) >= max_c;
        if at_end || nl || wrap {
            let end = if wrap {
                line_beg + max_c
            } else {
                i
            };
            if end > line_beg {
                shell::draw_ascii_line_clipped(
                    fb,
                    cl.x + pad + 8,
                    y,
                    cl.w - 2 * pad - 16,
                    &ui.notepad.buf[line_beg..end],
                    0x10,
                    0x10,
                    0x18,
                    1,
                );
            }
            y = y.saturating_add(line_h);
            if nl {
                line_beg = i + 1;
                i += 1;
            } else if wrap {
                line_beg = end;
            } else {
                break;
            }
        } else {
            i += 1;
        }
    }
}

fn paint_calc_client(fb: &FramebufferInfo, el: &ExplorerLayout, ui: &HostUiState) {
    let cl = el.client;
    shell::fill_rect_bgra(fb, cl.x, cl.y, cl.w, cl.h, 0x28, 0x28, 0x32);
    let disp = ui.calculator.entry;
    let mut tmp = [0u8; 24];
    let mut p = 0usize;
    let neg = disp < 0;
    let mut v = disp.unsigned_abs();
    if v == 0 {
        tmp[p] = b'0';
        p += 1;
    } else {
        let mut buf = [0u8; 20];
        let mut k = 20usize;
        while v > 0 && k > 0 {
            k -= 1;
            buf[k] = b'0' + (v % 10) as u8;
            v /= 10;
        }
        if neg {
            tmp[p] = b'-';
            p += 1;
        }
        for b in buf[k..].iter() {
            tmp[p] = *b;
            p += 1;
        }
    }
    shell::fill_rect_bgra(fb, cl.x + 12, cl.y + 12, cl.w - 24, 36, 0x10, 0x10, 0x18);
    shell::draw_ascii_line_clipped(fb, cl.x + 20, cl.y + 24, cl.w - 40, &tmp[..p], 0x00, 0xff, 0x90, 1);
    shell::draw_ascii_line_clipped(
        fb,
        cl.x + 12,
        cl.y + 56,
        cl.w - 24,
        b"Keys: 0-9 + - * / = Enter  C clear  Esc close",
        0x99,
        0x99,
        0xa2,
        1,
    );
}

fn paint_about_client(fb: &FramebufferInfo, el: &ExplorerLayout) {
    let cl = el.client;
    shell::fill_rect_bgra(fb, cl.x, cl.y, cl.w, cl.h, 0x26, 0x26, 0x30);
    let lines: [&[u8]; 5] = [
        b"ZirconOS NT10 bring-up shell",
        b"Fonts: OFL (Noto Sans UI raster, LXGW WenKai optional)",
        b"Icons: Zircon Fluent pack",
        b"Win32 docs: conceptual reference only",
        b"See docs/cn/Loader-Win32k-Desktop.md",
    ];
    let mut y = cl.y + 16;
    for line in lines {
        shell::draw_ascii_line_clipped(fb, cl.x + 16, y, cl.w - 32, line, 0xde, 0xde, 0xe6, 1);
        y += 18;
    }
}

fn paint_properties_client(fb: &FramebufferInfo, el: &ExplorerLayout, ui: &HostUiState) {
    let cl = el.client;
    shell::fill_rect_bgra(fb, cl.x, cl.y, cl.w, cl.h, 0x26, 0x26, 0x30);
    shell::draw_ascii_line_clipped(
        fb,
        cl.x + 12,
        cl.y + 16,
        cl.w - 24,
        b"Item properties (stub)",
        0xd8,
        0xd8,
        0xe0,
        1,
    );
    let mut line = [0u8; 64];
    let head = b"Target row index: ";
    line[..head.len()].copy_from_slice(head);
    let mut p = head.len();
    let mut v = ui.properties.target_line as u32;
    if v == 0 {
        line[p] = b'0';
        p += 1;
    } else {
        let mut t = [0u8; 8];
        let mut k = 8usize;
        while v > 0 && k > 0 {
            k -= 1;
            t[k] = b'0' + (v % 10) as u8;
            v /= 10;
        }
        for b in t[k..].iter() {
            line[p] = *b;
            p += 1;
        }
    }
    shell::draw_ascii_line_clipped(fb, cl.x + 12, cl.y + 40, cl.w - 24, &line[..p], 0xaa, 0xaa, 0xb4, 1);
}

/// Hit-test topmost window at `(px,py)`. Returns stack index and app id.
#[must_use]
pub fn hit_window_at(
    px: u32,
    py: u32,
    layout: &TaskbarLayout,
    stack: &WindowStack,
) -> Option<(usize, AppId)> {
    let n = stack.len as usize;
    for j in 0..n {
        let i = n - 1 - j;
        let el = explorer_view::layout_for_stack_depth(layout, i);
        if el.window.contains(px, py) {
            return Some((i, stack.ids[i]));
        }
    }
    None
}

#[must_use]
pub fn hit_close(el: &ExplorerLayout, px: u32, py: u32) -> bool {
    el.close_btn.contains(px, py)
}

#[must_use]
pub fn hit_files_row(el: &ExplorerLayout, px: u32, py: u32, row_count: usize) -> Option<usize> {
    explorer_view::row_hit(el.client, px, py, row_count)
}

#[must_use]
pub fn hit_taskmgr_tab(el: &ExplorerLayout, px: u32, py: u32) -> Option<u8> {
    let cl = el.client;
    if px < cl.x || py < cl.y || py >= cl.y + 28 {
        return None;
    }
    if px >= cl.x + 6 && px < cl.x + 106 {
        return Some(0);
    }
    if px >= cl.x + 112 && px < cl.x + 222 {
        return Some(1);
    }
    None
}

#[must_use]
pub fn hit_taskmgr_row(el: &ExplorerLayout, px: u32, py: u32, n_threads: usize) -> Option<usize> {
    let cl = el.client;
    let body_y = cl.y + 28 + 6;
    if px < cl.x + 4 || px >= cl.x + cl.w - 4 || py < body_y + 18 {
        return None;
    }
    let rel = py - (body_y + 18);
    let row = 22u32;
    let idx = (rel / row) as usize;
    if idx < n_threads.min(8) {
        Some(idx)
    } else {
        None
    }
}

#[must_use]
pub fn hit_settings_cat(el: &ExplorerLayout, px: u32, py: u32) -> Option<usize> {
    let cl = el.client;
    let split = 140u32.min(cl.w / 3);
    if px < cl.x + 4 || px >= cl.x + split - 4 || py < cl.y + 8 {
        return None;
    }
    let rel = py - (cl.y + 8);
    let row = 32u32;
    let i = (rel / row) as usize;
    if i < 6 {
        Some(i)
    } else {
        None
    }
}

#[must_use]
pub fn hit_settings_toggle(el: &ExplorerLayout, px: u32, py: u32) -> Option<usize> {
    let cl = el.client;
    let split = 140u32.min(cl.w / 3);
    let rx = cl.x + split + 8;
    let rw = cl.w.saturating_sub(split + 16);
    if px < rx || px >= rx + rw {
        return None;
    }
    for i in 0..4usize {
        let ry = cl.y + 40 + (i as u32) * 36;
        if py >= ry && py < ry + 30 {
            return Some(i);
        }
    }
    None
}

#[must_use]
pub fn hit_controlpanel_tile(el: &ExplorerLayout, px: u32, py: u32) -> Option<usize> {
    let cl = el.client;
    let gw = (cl.w - 40) / 2;
    let gh = (cl.h - 40) / 2;
    for i in 0..4usize {
        let col = (i % 2) as u32;
        let row = (i / 2) as u32;
        let gx = cl.x + 12 + col * (gw + 8);
        let gy = cl.y + 12 + row * (gh + 8);
        let r = HitRect {
            x: gx,
            y: gy,
            w: gw,
            h: gh,
        };
        if r.contains(px, py) {
            return Some(i);
        }
    }
    None
}

#[must_use]
pub fn hit_run_ok_cancel(el: &ExplorerLayout, px: u32, py: u32) -> Option<bool> {
    let (ok, cancel) = run_button_rects(el);
    if ok.contains(px, py) {
        return Some(true);
    }
    if cancel.contains(px, py) {
        return Some(false);
    }
    None
}

/// Taskbar slot index → `AppId` (front-most windows left to right).
#[must_use]
pub fn task_slot_app(stack: &WindowStack, visual_slot: usize) -> Option<AppId> {
    use super::taskbar::TASK_SLOT_COUNT;
    let n = stack.len as usize;
    if n == 0 || visual_slot >= TASK_SLOT_COUNT {
        return None;
    }
    let from_top = visual_slot;
    if from_top >= n {
        return None;
    }
    let idx = n - 1 - from_top;
    Some(stack.ids[idx])
}
