//! Window station and desktop objects (ZirconOSFluent clean-room layout).

use core::ptr::NonNull;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use super::directory::DirectoryObject;
use super::namespace::{
    normalize_winsta_path_to_sessions, parse_session_id_and_name, split_first_path_segment,
    strip_sessions_subpath, NamespaceBuckets,
};
use super::object::{ObjectHeader, ObjectTypeIndex};
use crate::ke::spinlock::SpinLock;
use crate::libs::win32_abi::{Hwnd, LParam, LResult, WParam};

/// HWND slots per desktop (bring-up).
pub const MAX_DESKTOP_WINDOWS: usize = 16;

/// Kernel-resident window procedure (tests and bring-up only).
pub type KernelWndProc = fn(Hwnd, u32, WParam, LParam) -> LResult;

#[inline]
fn desktop_nop_wndproc(_hwnd: Hwnd, _msg: u32, _wp: WParam, _lp: LParam) -> LResult {
    0
}

/// `DesktopWindowSlot::ex_style` — bring-up mirror of documented `WS_EX_*` bits (not layout-compatible).
pub const WS_EX_TOOLWINDOW: u32 = 0x0000_0080;

/// `DesktopWindowSlot::state` bits.
pub const WIN_STATE_MINIMIZED: u32 = 0x0000_0001;

/// `DesktopWindowSlot::ex_style` — skip in [`DesktopObject::hit_test_screen_topmost`] (full-screen underlay).
pub const WIN_EX_NO_HIT_TEST: u32 = 0x8000_0000;

/// Bring-up: marks a top-level popup (desktop context menu) — not layout-compatible with Win32 `WS_EX_*`.
pub const WIN_EX_SHELL_POPUP: u32 = 0x0800_0000;

/// One window on a desktop: HWND, owning thread, Z-order link.
#[derive(Clone, Copy)]
pub struct DesktopWindowSlot {
    pub in_use: bool,
    pub hwnd: Hwnd,
    pub owner_tid: u32,
    pub wndproc: KernelWndProc,
    /// Next index in [`DesktopWinInner::slots`]; `0xFF` = end of list.
    pub z_next: u8,
    /// Client-area invalid rect (surface pixel space) for Phase 4 compositor bring-up.
    pub dirty_active: bool,
    pub dirty_x0: u32,
    pub dirty_y0: u32,
    pub dirty_w: u32,
    pub dirty_h: u32,
    /// Screen placement for [`crate::subsystems::win32::compositor`]. `place_w == 0` keeps legacy
    /// horizontal strip layout at `origin_x + col * SURF_W`.
    pub place_x: u32,
    pub place_y: u32,
    pub place_w: u32,
    pub place_h: u32,
    pub ex_style: u32,
    pub state: u32,
}

impl DesktopWindowSlot {
    pub const EMPTY: Self = Self {
        in_use: false,
        hwnd: 0,
        owner_tid: 0,
        wndproc: desktop_nop_wndproc,
        z_next: 0xFF,
        dirty_active: false,
        dirty_x0: 0,
        dirty_y0: 0,
        dirty_w: 0,
        dirty_h: 0,
        place_x: 0,
        place_y: 0,
        place_w: 0,
        place_h: 0,
        ex_style: 0,
        state: 0,
    };
}

/// HWND table + Z-order head (single lock).
pub struct DesktopWinInner {
    pub slots: [DesktopWindowSlot; MAX_DESKTOP_WINDOWS],
    pub z_head: Option<u8>,
}

impl DesktopWinInner {
    pub const fn new() -> Self {
        Self {
            slots: [DesktopWindowSlot::EMPTY; MAX_DESKTOP_WINDOWS],
            z_head: None,
        }
    }
}

/// Session-0 window station: desktops live in [`Self::desktops`]; clipboard is a bring-up placeholder.
#[repr(C)]
pub struct WindowStationObject {
    pub header: ObjectHeader,
    pub desktops: DirectoryObject,
    pub clipboard_seq: AtomicU64,
    pub logon_token: u64,
}

impl WindowStationObject {
    #[must_use]
    pub fn new() -> Self {
        Self {
            header: ObjectHeader::new(ObjectTypeIndex::WINDOW_STATION),
            desktops: DirectoryObject::new(),
            clipboard_seq: AtomicU64::new(0),
            logon_token: 0,
        }
    }

    #[must_use]
    pub fn as_header_ptr(&mut self) -> NonNull<()> {
        NonNull::from(&mut self.header).cast()
    }
}

/// Per-desktop message pump / Z-order bring-up state.
#[repr(C)]
pub struct DesktopObject {
    pub header: ObjectHeader,
    pub posted_message_count: AtomicU32,
    /// Legacy counter; kept in sync with Z-order length for diagnostics.
    pub z_order_top_count: AtomicU32,
    pub win: SpinLock<DesktopWinInner>,
}

impl DesktopObject {
    #[must_use]
    pub fn new() -> Self {
        Self {
            header: ObjectHeader::new(ObjectTypeIndex::DESKTOP),
            posted_message_count: AtomicU32::new(0),
            z_order_top_count: AtomicU32::new(0),
            win: SpinLock::new(DesktopWinInner::new()),
        }
    }

    #[must_use]
    pub fn as_header_ptr(&mut self) -> NonNull<()> {
        NonNull::from(&mut self.header).cast()
    }

    /// Insert HWND with owner thread and procedure; newest at Z-order front.
    pub fn register_window(
        &self,
        hwnd: Hwnd,
        owner_tid: u32,
        wndproc: KernelWndProc,
    ) -> Result<(), ()> {
        let mut g = self.win.lock();
        for i in 0..MAX_DESKTOP_WINDOWS {
            if !g.slots[i].in_use {
                let old_head = g.z_head;
                g.slots[i] = DesktopWindowSlot {
                    in_use: true,
                    hwnd,
                    owner_tid,
                    wndproc,
                    z_next: old_head.unwrap_or(0xFF),
                    dirty_active: false,
                    dirty_x0: 0,
                    dirty_y0: 0,
                    dirty_w: 0,
                    dirty_h: 0,
                    place_x: 0,
                    place_y: 0,
                    place_w: 0,
                    place_h: 0,
                    ex_style: 0,
                    state: 0,
                };
                g.z_head = Some(i as u8);
                let n = self.z_order_top_count.load(Ordering::Relaxed) + 1;
                self.z_order_top_count.store(n, Ordering::Relaxed);
                return Ok(());
            }
        }
        Err(())
    }

    /// Resolve HWND to owning thread id and window procedure.
    #[must_use]
    pub fn lookup_hwnd(&self, hwnd: Hwnd) -> Option<(u32, KernelWndProc)> {
        let g = self.win.lock();
        for s in g.slots.iter() {
            if s.in_use && s.hwnd == hwnd {
                return Some((s.owner_tid, s.wndproc));
            }
        }
        None
    }

    /// Slot index `0..MAX_DESKTOP_WINDOWS` for `hwnd`, if registered.
    #[must_use]
    pub fn hwnd_slot_index(&self, hwnd: Hwnd) -> Option<u8> {
        let g = self.win.lock();
        for i in 0..MAX_DESKTOP_WINDOWS {
            if g.slots[i].in_use && g.slots[i].hwnd == hwnd {
                return Some(i as u8);
            }
        }
        None
    }

    /// Union an invalid rectangle into the slot (surface coordinates).
    pub fn invalidate_slot_rect(&self, slot: u8, x: u32, y: u32, w: u32, h: u32) {
        if w == 0 || h == 0 {
            return;
        }
        let i = slot as usize;
        if i >= MAX_DESKTOP_WINDOWS {
            return;
        }
        let mut g = self.win.lock();
        if !g.slots[i].in_use {
            return;
        }
        let s = &mut g.slots[i];
        if !s.dirty_active {
            s.dirty_active = true;
            s.dirty_x0 = x;
            s.dirty_y0 = y;
            s.dirty_w = w;
            s.dirty_h = h;
            return;
        }
        let x1 = s.dirty_x0.saturating_add(s.dirty_w);
        let y1 = s.dirty_y0.saturating_add(s.dirty_h);
        let nx0 = s.dirty_x0.min(x);
        let ny0 = s.dirty_y0.min(y);
        let nx1 = x1.max(x.saturating_add(w));
        let ny1 = y1.max(y.saturating_add(h));
        s.dirty_x0 = nx0;
        s.dirty_y0 = ny0;
        s.dirty_w = nx1.saturating_sub(nx0);
        s.dirty_h = ny1.saturating_sub(ny0);
    }

    /// Clear dirty flag after painting (optional compositor use).
    pub fn validate_slot(&self, slot: u8) {
        let i = slot as usize;
        if i >= MAX_DESKTOP_WINDOWS {
            return;
        }
        let mut g = self.win.lock();
        if g.slots[i].in_use {
            g.slots[i].dirty_active = false;
        }
    }

    /// Screen rectangle for compositor (UEFI / Phase5 shell).
    pub fn set_window_placement(
        &self,
        hwnd: Hwnd,
        x: u32,
        y: u32,
        w: u32,
        h: u32,
    ) -> Result<(), ()> {
        if w == 0 || h == 0 {
            return Err(());
        }
        let mut g = self.win.lock();
        for s in g.slots.iter_mut() {
            if s.in_use && s.hwnd == hwnd {
                s.place_x = x;
                s.place_y = y;
                s.place_w = w;
                s.place_h = h;
                return Ok(());
            }
        }
        Err(())
    }

    pub fn set_window_ex_style(&self, hwnd: Hwnd, ex_style: u32) -> Result<(), ()> {
        let mut g = self.win.lock();
        for s in g.slots.iter_mut() {
            if s.in_use && s.hwnd == hwnd {
                s.ex_style = ex_style;
                return Ok(());
            }
        }
        Err(())
    }

    pub fn set_window_minimized(&self, hwnd: Hwnd, minimized: bool) -> Result<(), ()> {
        let mut g = self.win.lock();
        for s in g.slots.iter_mut() {
            if s.in_use && s.hwnd == hwnd {
                if minimized {
                    s.state |= WIN_STATE_MINIMIZED;
                } else {
                    s.state &= !WIN_STATE_MINIMIZED;
                }
                return Ok(());
            }
        }
        Err(())
    }

    #[must_use]
    pub fn is_window_minimized(&self, hwnd: Hwnd) -> bool {
        let g = self.win.lock();
        g.slots
            .iter()
            .find(|s| s.in_use && s.hwnd == hwnd)
            .is_some_and(|s| (s.state & WIN_STATE_MINIMIZED) != 0)
    }

    /// Move `hwnd` to top of Z-order (newest / front-most).
    pub fn bring_hwnd_to_top(&self, hwnd: Hwnd) -> Result<(), ()> {
        let mut g = self.win.lock();
        let mut idx: Option<u8> = None;
        for i in 0..MAX_DESKTOP_WINDOWS {
            if g.slots[i].in_use && g.slots[i].hwnd == hwnd {
                idx = Some(i as u8);
                break;
            }
        }
        let idx = idx.ok_or(())?;
        let head = g.z_head.ok_or(())?;
        if head == idx {
            return Ok(());
        }
        let idx_us = idx as usize;
        let mut cur = head;
        loop {
            let nx = g.slots[cur as usize].z_next;
            if nx == 0xFF {
                return Err(());
            }
            if nx == idx {
                g.slots[cur as usize].z_next = g.slots[idx_us].z_next;
                break;
            }
            cur = nx;
        }
        let old_head = g.z_head;
        g.slots[idx_us].z_next = old_head.unwrap_or(0xFF);
        g.z_head = Some(idx);
        Ok(())
    }

    /// Front-most window whose explicit placement contains `(px, py)`. Skips minimized windows and
    /// slots with `place_w == 0` (legacy strip layout).
    #[must_use]
    pub fn hit_test_screen_topmost(&self, px: u32, py: u32) -> Option<Hwnd> {
        let g = self.win.lock();
        let Some(mut cur) = g.z_head else {
            return None;
        };
        let mut chain = [0u8; MAX_DESKTOP_WINDOWS];
        let mut n = 0usize;
        while n < MAX_DESKTOP_WINDOWS {
            chain[n] = cur;
            n += 1;
            let nx = g.slots[cur as usize].z_next;
            if nx == 0xFF {
                break;
            }
            cur = nx;
        }
        for k in 0..n {
            let idx = chain[k] as usize;
            let s = &g.slots[idx];
            if !s.in_use || (s.state & WIN_STATE_MINIMIZED) != 0 {
                continue;
            }
            if (s.ex_style & WIN_EX_NO_HIT_TEST) != 0 {
                continue;
            }
            if s.place_w == 0 {
                continue;
            }
            let x1 = s.place_x.saturating_add(s.place_w);
            let y1 = s.place_y.saturating_add(s.place_h);
            if px >= s.place_x && px < x1 && py >= s.place_y && py < y1 {
                return Some(s.hwnd);
            }
        }
        None
    }

    /// Walk Z-order from **bottom** (oldest) to **top** (newest) for compositing.
    pub fn for_each_hwnd_bottom_to_top(&self, mut f: impl FnMut(u8, Hwnd)) {
        let g = self.win.lock();
        let Some(mut cur) = g.z_head else {
            return;
        };
        let mut chain = [0u8; MAX_DESKTOP_WINDOWS];
        let mut n = 0usize;
        while n < MAX_DESKTOP_WINDOWS {
            chain[n] = cur;
            n += 1;
            let nx = g.slots[cur as usize].z_next;
            if nx == 0xFF {
                break;
            }
            cur = nx;
        }
        for k in (0..n).rev() {
            let idx = chain[k];
            let s = &g.slots[idx as usize];
            if s.in_use {
                f(idx, s.hwnd);
            }
        }
    }

    /// Front-to-back walk: HWNDs usable for Alt+Tab-style lists (visible placement, not minimized,
    /// not [`WS_EX_TOOLWINDOW`], not [`WIN_EX_NO_HIT_TEST`]).
    pub fn collect_visible_switcher_hwnds(&self, out: &mut [Hwnd]) -> usize {
        let g = self.win.lock();
        let Some(mut cur) = g.z_head else {
            return 0;
        };
        let mut chain = [0u8; MAX_DESKTOP_WINDOWS];
        let mut n = 0usize;
        while n < MAX_DESKTOP_WINDOWS {
            chain[n] = cur;
            n += 1;
            let nx = g.slots[cur as usize].z_next;
            if nx == 0xFF {
                break;
            }
            cur = nx;
        }
        let mut w = 0usize;
        for k in 0..n {
            let idx = chain[k] as usize;
            let s = &g.slots[idx];
            if !s.in_use || (s.state & WIN_STATE_MINIMIZED) != 0 || s.place_w == 0 {
                continue;
            }
            if (s.ex_style & WS_EX_TOOLWINDOW) != 0 {
                continue;
            }
            if (s.ex_style & WIN_EX_NO_HIT_TEST) != 0 {
                continue;
            }
            if w < out.len() {
                out[w] = s.hwnd;
                w += 1;
            }
        }
        w
    }
}

/// Mount `WinSta0` under `\Sessions\0\` and `Default` under that station.
/// `\Sessions\<0-7>\<WinSta>\<Desktop>` or `\Sessions\<0-7>\<WinSta>` only.
#[must_use]
pub fn lookup_session_winsta_desktop_path(
    ns: &NamespaceBuckets,
    path: &[u8],
) -> Option<NonNull<()>> {
    let mut buf = [0u8; 160];
    let path = match normalize_winsta_path_to_sessions(path, &mut buf) {
        Some(n) => &buf[..n],
        None => path,
    };
    let rest = strip_sessions_subpath(path)?;
    let (sid, after_session) = parse_session_id_and_name(rest)?;
    let (winsta_name, desktop_opt) = split_first_path_segment(after_session)?;
    let winsta_ptr = ns.by_session[sid].lookup(winsta_name)?;
    if let Some(desktop_name) = desktop_opt {
        if desktop_name.is_empty() {
            return None;
        }
        let ws = unsafe { &*winsta_ptr.as_ptr().cast::<WindowStationObject>() };
        ws.desktops.lookup(desktop_name)
    } else {
        Some(winsta_ptr)
    }
}

pub fn mount_session0_winsta0_default(
    ns: &mut NamespaceBuckets,
    mut winsta: NonNull<WindowStationObject>,
    desktop: NonNull<DesktopObject>,
) -> Result<(), ()> {
    unsafe {
        winsta
            .as_mut()
            .desktops
            .insert(b"Default", desktop.cast())?;
    }
    ns.insert_session_child(0, b"WinSta0", winsta.cast())
}

pub unsafe fn delete_desktop_static(p: *mut ()) {
    #[cfg(test)]
    {
        drop(alloc::boxed::Box::from_raw(p.cast::<DesktopObject>()));
        return;
    }
    #[cfg(not(test))]
    {
        let _ = p.cast::<DesktopObject>();
    }
}

pub unsafe fn delete_window_station_static(p: *mut ()) {
    let ws = unsafe { &mut *p.cast::<WindowStationObject>() };
    for c in ws.desktops.iter_objects() {
        delete_desktop_static(c.as_ptr());
    }
    ws.desktops.clear_for_teardown();
    #[cfg(test)]
    drop(alloc::boxed::Box::from_raw(p.cast::<WindowStationObject>()));
    #[cfg(not(test))]
    let _ = p.cast::<WindowStationObject>();
}

#[cfg(test)]
extern crate alloc;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ob::namespace::SESSION0_DESKTOP_DEFAULT;
    use crate::ps::process::EProcess;
    use alloc::boxed::Box;

    #[test]
    fn create_winsta_desktop_mount_lookup_handle_close_teardown() {
        let w = Box::leak(Box::new(WindowStationObject::new()));
        let d = Box::leak(Box::new(DesktopObject::new()));
        let win = NonNull::new(w).unwrap();
        let dsk = NonNull::new(d).unwrap();
        let mut ns = NamespaceBuckets::new();
        assert!(mount_session0_winsta0_default(&mut ns, win, dsk).is_ok());
        let got = lookup_session_winsta_desktop_path(&ns, SESSION0_DESKTOP_DEFAULT).unwrap();
        assert_eq!(got, dsk.cast());

        let mut proc = EProcess::new_bootstrap();
        let hw = proc.alloc_handle(w.as_header_ptr()).unwrap();
        assert!(proc.close_handle(hw).is_none());
        assert!(ns.remove_session_child(0, b"WinSta0").is_ok());
        assert!(lookup_session_winsta_desktop_path(&ns, SESSION0_DESKTOP_DEFAULT).is_none());
    }

    #[test]
    fn lookup_accepts_windows_winstations_alias() {
        let w = Box::leak(Box::new(WindowStationObject::new()));
        let d = Box::leak(Box::new(DesktopObject::new()));
        let win = NonNull::new(w).unwrap();
        let dsk = NonNull::new(d).unwrap();
        let mut ns = NamespaceBuckets::new();
        assert!(mount_session0_winsta0_default(&mut ns, win, dsk).is_ok());
        let legacy = br"\Windows\WindowStations\WinSta0\Default";
        assert_eq!(
            lookup_session_winsta_desktop_path(&ns, legacy).unwrap(),
            dsk.cast()
        );
    }
}
