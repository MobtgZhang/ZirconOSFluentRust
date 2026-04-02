//! `InvalidateRect` / `ValidateRect` bring-up: mark desktop dirty rects and post [`WM_PAINT`](super::windowing::wm).

use crate::libs::win32_abi::Hwnd;
use crate::ob::winsta::DesktopObject;

use super::gdi32::BringupHdc;
use super::msg_dispatch;
use super::windowing::wm;

/// Subset of `PAINTSTRUCT` for kernel bring-up (`hdc` = window surface slot index).
#[derive(Clone, Copy, Debug)]
pub struct BringupPaintStruct {
    pub hdc: BringupHdc,
    pub hwnd: Hwnd,
}

/// Begin painting: resolve slot DC and optional dirty union (validation on [`end_paint_bringup`]).
#[must_use]
pub fn begin_paint_bringup(desktop: &DesktopObject, hwnd: Hwnd) -> Option<BringupPaintStruct> {
    let si = desktop.hwnd_slot_index(hwnd)? as u16;
    Some(BringupPaintStruct {
        hdc: si as BringupHdc,
        hwnd,
    })
}

/// End painting: clear dirty state for the window slot.
pub fn end_paint_bringup(desktop: &DesktopObject, ps: &BringupPaintStruct) {
    let _ = validate_rect_kernel(desktop, ps.hwnd);
}

/// Mark `hwnd`'s surface rectangle dirty and post a posted `WM_PAINT` to the owner thread.
pub fn invalidate_rect_kernel(
    desktop: &DesktopObject,
    hwnd: Hwnd,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
) -> Result<(), ()> {
    let Some(si) = desktop.hwnd_slot_index(hwnd) else {
        return Err(());
    };
    desktop.invalidate_slot_rect(si, x, y, w, h);
    msg_dispatch::post_message_kernel(desktop, hwnd, wm::WM_PAINT, 0, 0)
}

/// Clear dirty state without posting a message.
pub fn validate_rect_kernel(desktop: &DesktopObject, hwnd: Hwnd) -> Result<(), ()> {
    let si = desktop.hwnd_slot_index(hwnd).ok_or(())?;
    desktop.validate_slot(si);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ob::winsta::DesktopObject;
    use crate::subsystems::win32::msg_dispatch::{self, try_get_message_kernel};
    use crate::subsystems::win32::windowing::{create_window_ex_on_desktop, register_class_ex_bringup};

    #[test]
    fn begin_end_paint_validate_slot() {
        let tid = 11u32;
        msg_dispatch::set_current_thread_for_win32(tid);
        let mut desktop = DesktopObject::new();
        let dptr = core::ptr::NonNull::from(&mut desktop);
        msg_dispatch::thread_bind_desktop(tid, dptr);
        let atom = register_class_ex_bringup(0, 0x62).expect("class");
        let hwnd = create_window_ex_on_desktop(
            unsafe { dptr.as_ref() },
            atom,
            0,
            tid,
            crate::subsystems::win32::windowing::def_window_proc_bringup,
        )
        .expect("hwnd");
        invalidate_rect_kernel(unsafe { dptr.as_ref() }, hwnd, 0, 0, 4, 4).expect("inv");
        let ps = begin_paint_bringup(unsafe { dptr.as_ref() }, hwnd).expect("begin");
        end_paint_bringup(unsafe { dptr.as_ref() }, &ps);
        let si = desktop.hwnd_slot_index(hwnd).expect("slot");
        let g = desktop.win.lock();
        assert!(!g.slots[si as usize].dirty_active);
    }

    #[test]
    fn invalidate_rect_posts_wm_paint() {
        let tid = 9u32;
        msg_dispatch::set_current_thread_for_win32(tid);
        let mut desktop = DesktopObject::new();
        let dptr = core::ptr::NonNull::from(&mut desktop);
        msg_dispatch::thread_bind_desktop(tid, dptr);
        let atom = register_class_ex_bringup(0, 0x61).expect("class");
        let hwnd = create_window_ex_on_desktop(
            unsafe { dptr.as_ref() },
            atom,
            0,
            tid,
            crate::subsystems::win32::windowing::def_window_proc_bringup,
        )
        .expect("hwnd");
        invalidate_rect_kernel(unsafe { dptr.as_ref() }, hwnd, 0, 0, 10, 10).expect("inv");
        let m = try_get_message_kernel(tid).expect("pump");
        assert_eq!(m.msg, wm::WM_PAINT);
        assert_eq!(m.hwnd, hwnd);
    }
}
