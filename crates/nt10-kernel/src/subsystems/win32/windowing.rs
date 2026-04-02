//! Minimal window class / HWND bring-up (kernel-side stubs until csrss owns real objects).

use crate::ke::spinlock::SpinLock;
use crate::libs::win32_abi::{Hwnd, LParam, LResult, WParam};
use crate::ob::winsta::{DesktopObject, KernelWndProc};
use core::sync::atomic::{AtomicU64, Ordering};

/// Standard window messages (subset for DefWindowProc and pump tests).
pub mod wm {
    pub const WM_NULL: u32 = 0x0000;
    pub const WM_CREATE: u32 = 0x0001;
    pub const WM_DESTROY: u32 = 0x0002;
    pub const WM_MOVE: u32 = 0x0003;
    pub const WM_SIZE: u32 = 0x0005;
    pub const WM_SETFOCUS: u32 = 0x0007;
    pub const WM_KILLFOCUS: u32 = 0x0008;
    pub const WM_PAINT: u32 = 0x000F;
    pub const WM_CLOSE: u32 = 0x0010;
    pub const WM_QUIT: u32 = 0x0012;
    pub const WM_ERASEBKGND: u32 = 0x0014;
    pub const WM_SHOWWINDOW: u32 = 0x0018;
    pub const WM_TIMER: u32 = 0x0113;
    pub const WM_NCCREATE: u32 = 0x0081;
    pub const WM_NCDESTROY: u32 = 0x0082;
    pub const WM_NCHITTEST: u32 = 0x0084;
    pub const WM_MOUSEMOVE: u32 = 0x0200;
    pub const WM_LBUTTONDOWN: u32 = 0x0201;
    pub const WM_LBUTTONUP: u32 = 0x0202;
    pub const WM_RBUTTONUP: u32 = 0x0205;
    pub const WM_CAPTURECHANGED: u32 = 0x0215;
    pub const WM_USER: u32 = 0x0400;
}

/// [`wm::WM_NCHITTEST`] return values (documented names; Zircon uses `i64` bring-up).
pub mod ht {
    use crate::libs::win32_abi::LResult;
    pub const HTNOWHERE: LResult = 0;
    pub const HTCLIENT: LResult = 1;
    pub const HTCAPTION: LResult = 2;
    pub const HTBORDER: LResult = 18;
}

static NEXT_HWND: AtomicU64 = AtomicU64::new(0x1_0000);

const MAX_ATOMS: usize = 32;

#[derive(Clone, Copy, Debug)]
struct ClassSlot {
    in_use: bool,
    atom: u16,
    style: u32,
}

impl ClassSlot {
    const fn empty() -> Self {
        Self {
            in_use: false,
            atom: 0,
            style: 0,
        }
    }
}

static CLASSES: SpinLock<[ClassSlot; MAX_ATOMS]> = SpinLock::new([ClassSlot::empty(); MAX_ATOMS]);

/// Register a window class (bring-up: no name string table; `class_hint` seeds atom).
pub fn register_class_ex_bringup(style: u32, class_hint: u32) -> Result<u16, ()> {
    let mut guard = CLASSES.lock();
    for slot in guard.iter_mut() {
        if !slot.in_use {
            let atom = ((class_hint as u16) ^ 0xC000) | 0x8000;
            slot.in_use = true;
            slot.atom = atom;
            slot.style = style;
            return Ok(atom);
        }
    }
    Err(())
}

/// Allocate a new HWND value (opaque handle id).
#[must_use]
pub fn alloc_hwnd() -> Hwnd {
    NEXT_HWND.fetch_add(4, Ordering::Relaxed) as Hwnd
}

/// Create-window stub: returns a fresh HWND linked to `class_atom` for tracing.
pub fn create_window_ex_bringup(class_atom: u16, _parent: Hwnd) -> Result<Hwnd, ()> {
    let guard = CLASSES.lock();
    let found = guard.iter().any(|s| s.in_use && s.atom == class_atom);
    if !found {
        return Err(());
    }
    drop(guard);
    Ok(alloc_hwnd())
}

/// Create-window on a desktop: validates class, allocates HWND, registers HWND table + Z-order.
pub fn create_window_ex_on_desktop(
    desktop: &DesktopObject,
    class_atom: u16,
    _parent: Hwnd,
    owner_tid: u32,
    wndproc: KernelWndProc,
) -> Result<Hwnd, ()> {
    let guard = CLASSES.lock();
    let found = guard.iter().any(|s| s.in_use && s.atom == class_atom);
    if !found {
        return Err(());
    }
    drop(guard);
    let hwnd = alloc_hwnd();
    desktop.register_window(hwnd, owner_tid, wndproc)?;
    if let Some(si) = desktop.hwnd_slot_index(hwnd) {
        super::window_surface::clear_surface(si as usize);
    }
    Ok(hwnd)
}

/// Default window procedure — minimal NT10 bring-up behavior.
///
/// Non-client hit-testing returns [`ht::HTCLIENT`] here; framed windows use a custom `WndProc` for
/// [`ht::HTCAPTION`] / [`ht::HTBORDER`] (Phase 5 test HWND).
#[must_use]
pub fn def_window_proc_bringup(_hwnd: Hwnd, msg: u32, _wparam: WParam, _lparam: LParam) -> LResult {
    match msg {
        wm::WM_CLOSE => 0,
        wm::WM_DESTROY => 0,
        wm::WM_PAINT => 0,
        wm::WM_ERASEBKGND => 1,
        wm::WM_NCHITTEST => ht::HTCLIENT,
        wm::WM_MOUSEMOVE | wm::WM_LBUTTONDOWN | wm::WM_LBUTTONUP | wm::WM_RBUTTONUP => 0,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_create_hwnd() {
        let a = register_class_ex_bringup(0, 1).expect("atom");
        let h = create_window_ex_bringup(a, 0).expect("hwnd");
        assert!(h >= 0x1_0000);
    }

    #[test]
    fn create_on_desktop_registers_lookup() {
        use crate::ob::winsta::DesktopObject;
        let a = register_class_ex_bringup(0, 2).expect("atom");
        let d = DesktopObject::new();
        let h = create_window_ex_on_desktop(&d, a, 0, 1, def_window_proc_bringup).expect("hwnd");
        let got = d.lookup_hwnd(h).expect("lookup");
        assert_eq!(got.0, 1u32);
    }
}
