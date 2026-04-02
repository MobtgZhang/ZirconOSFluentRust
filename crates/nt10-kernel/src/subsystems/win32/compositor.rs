//! Software compositor: desktop window surfaces bottom-to-top into a framebuffer slice.

use crate::ob::winsta::{DesktopObject, MAX_DESKTOP_WINDOWS};

use super::window_surface;

/// Which HWND layers to composite (Phase 5: wallpaper is Z-back; shell draws shortcuts on top).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompositeDesktopFilter {
    /// All windows bottom-to-top (default).
    All,
    /// Only the bottom-most (oldest) HWND — full-screen wallpaper under shell shortcuts.
    BottomLayerOnly,
    /// All except the bottom-most layer.
    ExcludeBottomLayer,
}

/// Composite all window slots (Z-order back to front) into `dst_buf`.
///
/// Windows with `place_w == 0` use bring-up strip layout: `origin_x + col * SURF_W`, `origin_y`.
/// Otherwise each window uses [`crate::ob::winsta::DesktopWindowSlot`] placement (stretched).
/// Minimized windows (`WIN_STATE_MINIMIZED`) are skipped.
pub fn composite_desktop_to_framebuffer(
    desktop: &DesktopObject,
    dst_buf: &mut [u8],
    dst_w: u32,
    dst_h: u32,
    stride_px: u32,
    origin_x: u32,
    origin_y: u32,
) -> Result<(), ()> {
    composite_desktop_to_framebuffer_filtered(
        desktop,
        dst_buf,
        dst_w,
        dst_h,
        stride_px,
        origin_x,
        origin_y,
        CompositeDesktopFilter::All,
    )
}

/// Same as [`composite_desktop_to_framebuffer`] with a layer filter.
pub fn composite_desktop_to_framebuffer_filtered(
    desktop: &DesktopObject,
    dst_buf: &mut [u8],
    dst_w: u32,
    dst_h: u32,
    stride_px: u32,
    origin_x: u32,
    origin_y: u32,
    filter: CompositeDesktopFilter,
) -> Result<(), ()> {
    let g = desktop.win.lock();
    let Some(mut cur) = g.z_head else {
        return Ok(());
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
    let bottom_k = n.saturating_sub(1);
    let mut col = 0u32;
    for k in (0..n).rev() {
        match filter {
            CompositeDesktopFilter::All => {}
            CompositeDesktopFilter::BottomLayerOnly if k != bottom_k => continue,
            CompositeDesktopFilter::ExcludeBottomLayer if k == bottom_k => continue,
            _ => {}
        }
        let idx = chain[k] as usize;
        let s = &g.slots[idx];
        if !s.in_use {
            continue;
        }
        if (s.state & crate::ob::winsta::WIN_STATE_MINIMIZED) != 0 {
            continue;
        }
        if s.place_w == 0 {
            let dx = origin_x.saturating_add(col * window_surface::SURF_W);
            let _ = window_surface::blit_slot_to_framebuffer(
                idx,
                dst_buf,
                dst_w,
                dst_h,
                stride_px,
                dx.min(dst_w.saturating_sub(1)),
                origin_y.min(dst_h.saturating_sub(1)),
            );
            col = col.saturating_add(1);
        } else {
            let _ = window_surface::blit_slot_stretch_to_framebuffer_src_over(
                idx,
                dst_buf,
                dst_w,
                dst_h,
                stride_px,
                s.place_x,
                s.place_y,
                s.place_w,
                s.place_h,
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ob::winsta::DesktopObject;
    use crate::subsystems::win32::text_bringup::text_out_ascii;
    use crate::subsystems::win32::window_surface;
    use crate::subsystems::win32::windowing::{create_window_ex_on_desktop, register_class_ex_bringup};

    #[test]
    fn composite_writes_nonzero_pixel() {
        let tid = 3u32;
        crate::subsystems::win32::msg_dispatch::set_current_thread_for_win32(tid);
        let mut desktop = DesktopObject::new();
        let dptr = core::ptr::NonNull::from(&mut desktop);
        crate::subsystems::win32::msg_dispatch::thread_bind_desktop(tid, dptr);
        let atom = register_class_ex_bringup(0, 0x88).expect("class");
        let hwnd = create_window_ex_on_desktop(
            unsafe { dptr.as_ref() },
            atom,
            0,
            tid,
            crate::subsystems::win32::windowing::def_window_proc_bringup,
        )
        .expect("hwnd");
        let si = desktop.hwnd_slot_index(hwnd).expect("slot") as usize;
        window_surface::fill_surface_solid(si, [40, 80, 120, 255]);
        text_out_ascii(si, 4, 4, b"Phase4", [255, 255, 255, 255]);
        let mut fb = [0u8; 256 * 64 * 4];
        composite_desktop_to_framebuffer(
            unsafe { dptr.as_ref() },
            &mut fb,
            256,
            64,
            256,
            0,
            8,
        )
        .expect("composite");
        assert!(fb.iter().any(|&b| b != 0));
    }
}
