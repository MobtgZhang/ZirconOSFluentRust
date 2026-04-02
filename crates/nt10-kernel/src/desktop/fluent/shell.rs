//! Taskbar / Start — consumes GOP handoff for early splash (software draw, no Win32k yet).
//!
//! **Wallpaper single source (Phase 5):** when [`crate::desktop::fluent::session_win32::Win32ShellState::desktop_ready`]
//! is true, only the Win32 wallpaper HWND paints [`super::resources::DEFAULT_WALLPAPER_BGRA`]; this module skips
//! [`paint_wallpaper_only`] in [`redraw_uefi_desktop_skip_wallpaper`]. Without Win32, [`paint_wallpaper_only`]
//! / the tight base cache remain authoritative.
//!
//! Default wallpaper id for the resource pack: [`super::resources::DEFAULT_WALLPAPER_ID`].
//!
//! Taskbar layout and notification flyouts mirror the *roles* described in
//! `references/win32/desktop-src/shell/taskbar.md`, `shell/notification-area.md`, and
//! `uxguide/winenv-taskbar.md` / `uxguide/winenv-notification.md` (no Win32 shell code).

use nt10_boot_protocol::FramebufferInfo;

use crate::drivers::video::display_mgr;

use super::app_host::WindowStack;
use super::hosted_apps::{self, HostUiState};
use super::resources::{
    context_menu_label_bgra, desktop_caption_bgra, desktop_icon_bgra, start_menu_icon_bgra,
    start_menu_label_bgra, DEFAULT_WALLPAPER_BGRA, DEFAULT_WALLPAPER_HEIGHT, DEFAULT_WALLPAPER_WIDTH,
    DESKTOP_CAPTION_MAX_H, DESKTOP_ICON_COUNT, FONT_GLYPH_MASKS, POINTER_CURSOR_BGRA, POINTER_CURSOR_H,
    POINTER_CURSOR_W, POINTER_HOTSPOT_X, POINTER_HOTSPOT_Y, START_MENU_ICON_H, START_MENU_ICON_W,
};
use super::taskbar::{HitRect, TaskbarLayout, TASK_SLOT_COUNT, TRAY_ICON_COUNT, START_MENU_ROW_COUNT};

/// Stretch BGRA8 source (`src_w`×`src_h`, tight stride) to full framebuffer (nearest neighbor).
pub fn blit_bgra_stretch(fb: &FramebufferInfo, src: &[u8], src_w: u32, src_h: u32) {
    if fb.base == 0 || src_w == 0 || src_h == 0 {
        return;
    }
    let need = (src_w as usize)
        .checked_mul(src_h as usize)
        .and_then(|n| n.checked_mul(4))
        .unwrap_or(0);
    if src.len() < need {
        return;
    }
    let dst_w = fb.horizontal_resolution;
    let dst_h = fb.vertical_resolution;
    if dst_w == 0 || dst_h == 0 {
        return;
    }
    let stride_px = fb.pixels_per_scan_line as usize;
    let ptr = fb.base as *mut u8;
    let byte_cap = display_mgr::framebuffer_linear_byte_cap(fb);
    for dy in 0..dst_h {
        for dx in 0..dst_w {
            let sx = (dx as u64 * src_w as u64 / dst_w as u64) as u32;
            let sy = (dy as u64 * src_h as u64 / dst_h as u64) as u32;
            let si = ((sy * src_w + sx) * 4) as usize;
            if si + 4 > src.len() {
                return;
            }
            let off = (dy as usize * stride_px + dx as usize) * 4;
            if off + 4 > byte_cap {
                continue;
            }
            let sb = src[si];
            let sg = src[si + 1];
            let sr = src[si + 2];
            let sa = src[si + 3];
            unsafe {
                let p = ptr.add(off);
                if sa == 0 {
                    continue;
                }
                if sa >= 255 {
                    display_mgr::fb_write_opaque_rgb8(fb, p, sr, sg, sb);
                } else {
                    let ia = sa as u32;
                    let inv = 255 - ia;
                    let (dr, dg, db) = display_mgr::fb_read_rgb8(fb, p);
                    let or = ((sr as u32 * ia + dr as u32 * inv) / 255) as u8;
                    let og = ((sg as u32 * ia + dg as u32 * inv) / 255) as u8;
                    let ob = ((sb as u32 * ia + db as u32 * inv) / 255) as u8;
                    display_mgr::fb_write_opaque_rgb8(fb, p, or, og, ob);
                }
            }
        }
    }
}

/// Clears the framebuffer to a solid color (BGRx / BGRA, 32 bpp — matches typical UEFI GOP).
pub fn clear_splash_rgb(fb: &FramebufferInfo, r: u8, g: u8, b: u8) {
    if fb.base == 0 || fb.size == 0 {
        return;
    }
    let ptr = fb.base as *mut u8;
    let stride = fb.pixels_per_scan_line as usize * 4;
    let h = fb.vertical_resolution as usize;
    let w = fb.horizontal_resolution as usize;
    let cap = display_mgr::framebuffer_linear_byte_cap(fb);
    for y in 0..h {
        let row = unsafe { ptr.add(y * stride) };
        for x in 0..w {
            let off = y * stride + x * 4;
            if off + 4 > cap {
                continue;
            }
            let p = unsafe { row.add(x * 4) };
            unsafe {
                display_mgr::fb_write_opaque_rgb8(fb, p, r, g, b);
            }
        }
    }
}

/// Filled axis-aligned rectangle in surface space (`x`, `y` top-left).
pub fn fill_rect_bgra(fb: &FramebufferInfo, x: u32, y: u32, w: u32, h: u32, r: u8, g: u8, b: u8) {
    if fb.base == 0 || w == 0 || h == 0 {
        return;
    }
    let surf_w = fb.horizontal_resolution;
    let surf_h = fb.vertical_resolution;
    let stride_px = fb.pixels_per_scan_line as usize;
    let ptr = fb.base as *mut u8;
    let byte_cap = display_mgr::framebuffer_linear_byte_cap(fb);
    for dy in 0..h {
        let yy = y.saturating_add(dy);
        if yy >= surf_h {
            break;
        }
        for dx in 0..w {
            let xx = x.saturating_add(dx);
            if xx >= surf_w {
                break;
            }
            let off = (yy as usize * stride_px + xx as usize) * 4;
            if off + 4 > byte_cap {
                continue;
            }
            unsafe {
                let p = ptr.add(off);
                display_mgr::fb_write_opaque_rgb8(fb, p, r, g, b);
            }
        }
    }
}

pub fn paint_wallpaper_only(fb: &FramebufferInfo) {
    blit_bgra_stretch(
        fb,
        DEFAULT_WALLPAPER_BGRA,
        DEFAULT_WALLPAPER_WIDTH,
        DEFAULT_WALLPAPER_HEIGHT,
    );
}

/// Left edge of the desktop shortcut column (above the taskbar).
const DESKTOP_SHORTCUT_MARGIN_X: u32 = 16;
const DESKTOP_SHORTCUT_MARGIN_Y: u32 = 16;
const DESKTOP_SHORTCUT_ICON_LABEL_GAP: u32 = 6;
const DESKTOP_SHORTCUT_ROW_GAP: u32 = 10;

/// Column width for horizontal centering of 32×32 icon + raster caption (`build.rs` Source Han Serif).
pub const DESKTOP_COLUMN_W: u32 = 152;

#[inline]
fn desktop_shortcut_slot_h() -> u32 {
    START_MENU_ICON_H + DESKTOP_SHORTCUT_ICON_LABEL_GAP + DESKTOP_CAPTION_MAX_H
}

/// Wallpaper layer: shell_desktop icons + English captions (embedded BGRA: Libertinus at build time).
///
/// Slot order matches [`DESKTOP_ICON_COUNT`] / `build.rs` manifest ids (computer, documents, recycle, network).
pub fn paint_desktop_shortcuts(fb: &FramebufferInfo, layout: &TaskbarLayout) {
    if fb.base == 0 || layout.surface_h == 0 {
        return;
    }
    let bar_top = layout.bar.y;
    let max_y = bar_top.saturating_sub(12);
    let slot_h = desktop_shortcut_slot_h();
    let mut y = DESKTOP_SHORTCUT_MARGIN_Y;
    for i in 0..DESKTOP_ICON_COUNT {
        if y.saturating_add(slot_h) > max_y {
            break;
        }
        let col_x = DESKTOP_SHORTCUT_MARGIN_X;
        let icon_x = col_x + DESKTOP_COLUMN_W.saturating_sub(START_MENU_ICON_W) / 2;
        if let Some(icon) = desktop_icon_bgra(i) {
            blit_bgra_straight_alpha(fb, icon_x, y, icon, START_MENU_ICON_W, START_MENU_ICON_H);
        }
        if let Some((cap, cw, ch)) = desktop_caption_bgra(i) {
            let cap_x = col_x + DESKTOP_COLUMN_W.saturating_sub(cw) / 2;
            let cap_y = y + START_MENU_ICON_H + DESKTOP_SHORTCUT_ICON_LABEL_GAP;
            blit_bgra_straight_alpha(fb, cap_x, cap_y, cap, cw, ch);
        }
        y = y.saturating_add(slot_h + DESKTOP_SHORTCUT_ROW_GAP);
    }
}

/// Hit-test desktop shortcut column (`paint_desktop_shortcuts` geometry).
#[must_use]
pub fn desktop_shortcut_hit(px: u32, py: u32, layout: &TaskbarLayout) -> Option<usize> {
    let bar_top = layout.bar.y;
    let max_y = bar_top.saturating_sub(12);
    let slot_h = desktop_shortcut_slot_h();
    let mut y = DESKTOP_SHORTCUT_MARGIN_Y;
    for i in 0..DESKTOP_ICON_COUNT {
        if y.saturating_add(slot_h) > max_y {
            break;
        }
        let col_x = DESKTOP_SHORTCUT_MARGIN_X;
        let r = HitRect {
            x: col_x,
            y,
            w: DESKTOP_COLUMN_W,
            h: slot_h,
        };
        if r.contains(px, py) {
            return Some(i);
        }
        y = y.saturating_add(slot_h + DESKTOP_SHORTCUT_ROW_GAP);
    }
    None
}

/// Software cursor bounding box (`build.rs` 2× upsampled arrow, high contrast).
pub const POINTER_CURSOR_SIZE: u32 = POINTER_CURSOR_W;

/// Top-left of the pointer sprite for logical hotspot `(hotspot_x, hotspot_y)` in framebuffer pixels.
///
/// Aligns with Win32 cursor hotspot vs bitmap origin (`references/win32/desktop-src/LearnWin32/mouse-movement.md`).
#[inline]
#[must_use]
pub fn pointer_sprite_top_left(hotspot_x: u32, hotspot_y: u32) -> (u32, u32) {
    (
        hotspot_x.saturating_sub(POINTER_HOTSPOT_X),
        hotspot_y.saturating_sub(POINTER_HOTSPOT_Y),
    )
}

/// Pointer sprite from [`super::resources`] / `generated_cursor.rs`; `x`,`y` are **sprite top-left**.
///
/// Uses **opaque writes only** for non-zero alpha (no `fb_read_rgb8` blend). Some GOP/framebuffer
/// implementations do not support reliable readback; blending would make the cursor invisible.
///
/// Length of embedded `pointer_cursor.bgra` (diagnostics).
#[inline]
#[must_use]
pub fn pointer_cursor_asset_len() -> usize {
    POINTER_CURSOR_BGRA.len()
}

pub fn paint_pointer_cursor(fb: &FramebufferInfo, x: u32, y: u32) {
    blit_bgra_sprite_opaque_nonzero(fb, x, y, POINTER_CURSOR_BGRA, POINTER_CURSOR_W, POINTER_CURSOR_H);
    let hx = x.saturating_add(POINTER_HOTSPOT_X);
    let hy = y.saturating_add(POINTER_HOTSPOT_Y);
    let surf_w = fb.horizontal_resolution;
    let surf_h = fb.vertical_resolution;
    if hx < surf_w && hy < surf_h {
        let dot = 2u32.min(surf_w.saturating_sub(hx)).min(surf_h.saturating_sub(hy));
        if dot > 0 {
            fill_rect_bgra(fb, hx, hy, dot, dot, 0xff, 0xff, 0xff);
        }
    }
    display_mgr::framebuffer_store_fence();
}

/// BGRA sprite: any pixel with `alpha > 0` is written as fully opaque sRGB `(R,G,B)` from the source.
/// Per-pixel offset is clamped with [`display_mgr::framebuffer_linear_byte_cap`].
fn blit_bgra_sprite_opaque_nonzero(
    fb: &FramebufferInfo,
    dst_x: u32,
    dst_y: u32,
    src: &[u8],
    src_w: u32,
    src_h: u32,
) {
    if fb.base == 0 || src_w == 0 || src_h == 0 {
        return;
    }
    let need = (src_w as usize)
        .checked_mul(src_h as usize)
        .and_then(|n| n.checked_mul(4))
        .unwrap_or(0);
    if src.len() < need {
        return;
    }
    let surf_w = fb.horizontal_resolution;
    let surf_h = fb.vertical_resolution;
    let stride_px = fb.pixels_per_scan_line as usize;
    let ptr = fb.base as *mut u8;
    let write_cap = display_mgr::framebuffer_linear_byte_cap(fb);
    for sy in 0..src_h {
        let dy = dst_y.saturating_add(sy);
        if dy >= surf_h {
            break;
        }
        for sx in 0..src_w {
            let dx = dst_x.saturating_add(sx);
            if dx >= surf_w {
                break;
            }
            let si = ((sy * src_w + sx) * 4) as usize;
            if si + 4 > src.len() {
                return;
            }
            let sa = src[si + 3];
            if sa == 0 {
                continue;
            }
            let sb = src[si];
            let sg = src[si + 1];
            let sr = src[si + 2];
            let off = (dy as usize * stride_px + dx as usize) * 4;
            if off + 4 > write_cap {
                continue;
            }
            unsafe {
                let p = ptr.add(off);
                display_mgr::fb_write_opaque_rgb8(fb, p, sr, sg, sb);
            }
        }
    }
}

/// Monospace 8×8 bitmask glyph (`FONT_GLYPH_MASKS`), scaled.
pub fn blit_glyph_mono8(
    fb: &FramebufferInfo,
    x: u32,
    y: u32,
    ch: u8,
    r: u8,
    g: u8,
    b: u8,
    scale: u32,
) {
    if ch < 32 || ch > 126 {
        return;
    }
    let idx = (ch - 32) as usize;
    if idx >= FONT_GLYPH_MASKS.len() {
        return;
    }
    let rows = &FONT_GLYPH_MASKS[idx];
    let sc = scale.max(1);
    for row in 0..8u32 {
        let bits = rows[row as usize];
        for col in 0..8u32 {
            if bits & (1 << (7 - col)) == 0 {
                continue;
            }
            let px = x.saturating_add(col * sc);
            let py = y.saturating_add(row * sc);
            fill_rect_bgra(fb, px, py, sc, sc, r, g, b);
        }
    }
}

/// Draw ASCII bytes clipped to `max_width` from `x`.
pub fn draw_ascii_line_clipped(
    fb: &FramebufferInfo,
    mut x: u32,
    y: u32,
    max_width: u32,
    text: &[u8],
    r: u8,
    g: u8,
    b: u8,
    scale: u32,
) {
    let x0 = x;
    let sc = scale.max(1);
    let step = 8 * sc + sc;
    for &ch in text {
        if ch < 32 || ch > 126 {
            continue;
        }
        let idx = (ch - 32) as usize;
        if idx >= FONT_GLYPH_MASKS.len() {
            continue;
        }
        if x.saturating_sub(x0).saturating_add(8 * sc) > max_width {
            break;
        }
        blit_glyph_mono8(fb, x, y, ch, r, g, b, sc);
        x = x.saturating_add(step);
    }
}

/// Aggregated shell paint flags for UEFI desktop session (taskbar, flyouts, context menus).
#[derive(Clone, Copy, Debug)]
pub struct DesktopChromeState {
    pub menu_open: bool,
    pub menu_sel: usize,
    pub ctx_open: bool,
    pub ctx_sel: usize,
    pub ctx_x: u32,
    pub ctx_y: u32,
    pub win_stack_len: u8,
    pub win_stack: [u8; 8],
    pub power_confirm_open: bool,
    /// ASCII clock `HH:MM:SS` (taskbar notification area).
    pub clock_time: [u8; 16],
    pub clock_time_n: u8,
    /// ASCII date `YYYY/MM/DD`.
    pub clock_date: [u8; 20],
    pub clock_date_n: u8,
    /// `0` = none; see [`FLYOUT_KIND_CALENDAR_STUB`], [`FLYOUT_KIND_VOLUME_STUB`].
    pub flyout: u8,
    pub flyout_x: u32,
    pub flyout_y: u32,
    pub tb_ctx_open: bool,
    pub tb_ctx_sel: usize,
    pub tb_ctx_x: u32,
    pub tb_ctx_y: u32,
}

impl Default for DesktopChromeState {
    fn default() -> Self {
        Self {
            menu_open: false,
            menu_sel: 0,
            ctx_open: false,
            ctx_sel: 0,
            ctx_x: 0,
            ctx_y: 0,
            win_stack_len: 0,
            win_stack: [0; 8],
            power_confirm_open: false,
            clock_time: [0; 16],
            clock_time_n: 0,
            clock_date: [0; 20],
            clock_date_n: 0,
            flyout: FLYOUT_KIND_NONE,
            flyout_x: 0,
            flyout_y: 0,
            tb_ctx_open: false,
            tb_ctx_sel: 0,
            tb_ctx_x: 0,
            tb_ctx_y: 0,
        }
    }
}

pub const FLYOUT_KIND_NONE: u8 = 0;
pub const FLYOUT_KIND_CALENDAR_STUB: u8 = 1;
pub const FLYOUT_KIND_VOLUME_STUB: u8 = 2;

/// Max tight BGRA bytes for cached wallpaper + desktop shortcuts (`1920×1080×4`).
pub const DESKTOP_BASE_LAYER_CAP_BYTES: usize = 1920 * 1080 * 4;

#[must_use]
pub fn desktop_base_layer_byte_len(w: u32, h: u32) -> Option<usize> {
    (w as usize).checked_mul(h as usize)?.checked_mul(4)
}

/// Rasterize wallpaper + desktop shortcuts into a tight BGRA buffer (`stride == width`).
#[must_use]
pub fn rebuild_desktop_base_layer(
    tight_buf: &mut [u8],
    w: u32,
    h: u32,
    pixel_format: u32,
    layout: &TaskbarLayout,
) -> bool {
    let need = match desktop_base_layer_byte_len(w, h) {
        Some(n) => n,
        None => return false,
    };
    if tight_buf.len() < need {
        return false;
    }
    let sub = FramebufferInfo {
        base: tight_buf.as_mut_ptr() as u64,
        size: need,
        horizontal_resolution: w,
        vertical_resolution: h,
        pixels_per_scan_line: w,
        pixel_format,
    };
    paint_wallpaper_only(&sub);
    paint_desktop_shortcuts(&sub, layout);
    true
}

/// Copy tight BGRA (`w`×`h`, stride `w`) into a GOP framebuffer.
pub fn blit_tight_bgra_to_framebuffer(src: &[u8], w: u32, h: u32, dst: &FramebufferInfo) {
    let need = match desktop_base_layer_byte_len(w, h) {
        Some(n) => n,
        None => return,
    };
    if src.len() < need || dst.base == 0 {
        return;
    }
    let stride_px = dst.pixels_per_scan_line as usize;
    let cap = display_mgr::framebuffer_linear_byte_cap(dst);
    let ptr = dst.base as *mut u8;
    let row_b = w as usize * 4;
    for row in 0..h as usize {
        let src_row = row * row_b;
        let dst_row = row * stride_px * 4;
        if src_row + row_b > need || dst_row + row_b > cap {
            return;
        }
        unsafe {
            core::ptr::copy_nonoverlapping(src.as_ptr().add(src_row), ptr.add(dst_row), row_b);
        }
    }
}

pub const CONTEXT_MENU_ROW_H: u32 = 36;
pub const CONTEXT_MENU_ROWS: usize = 3;
pub const CONTEXT_MENU_W: u32 = 200;

/// Right-click menu (Zircon copy; not Microsoft UI assets).
pub fn paint_context_menu(
    fb: &FramebufferInfo,
    anchor_x: u32,
    anchor_y: u32,
    selected: usize,
) {
    let surf_w = fb.horizontal_resolution;
    let surf_h = fb.vertical_resolution;
    if fb.base == 0 || surf_w == 0 || surf_h == 0 {
        return;
    }
    let w = CONTEXT_MENU_W.min(surf_w.saturating_sub(4));
    let h = CONTEXT_MENU_ROW_H * CONTEXT_MENU_ROWS as u32;
    let x = anchor_x.min(surf_w.saturating_sub(w));
    let y = anchor_y.min(surf_h.saturating_sub(h));
    fill_rect_bgra(fb, x, y, w, h, 0x2a, 0x2a, 0x34);
    fill_rect_bgra(fb, x, y, w, 2, 0x00, 0x78, 0xd7);
    for i in 0..CONTEXT_MENU_ROWS {
        let ry = y + 2 + (i as u32) * CONTEXT_MENU_ROW_H;
        let sel = i == selected.min(CONTEXT_MENU_ROWS - 1);
        let (r, g, b) = if sel {
            (0x00, 0x78, 0xd7)
        } else {
            (0x36, 0x36, 0x40)
        };
        fill_rect_bgra(
            fb,
            x + 2,
            ry,
            w.saturating_sub(4),
            CONTEXT_MENU_ROW_H.saturating_sub(2),
            r,
            g,
            b,
        );
        let pad_x = 8u32;
        if let Some((label, lw, lh)) = context_menu_label_bgra(i) {
            let max_w = w.saturating_sub(4).saturating_sub(pad_x * 2);
            if lw <= max_w {
                let lx = x + pad_x;
                let row_inner_h = CONTEXT_MENU_ROW_H.saturating_sub(2);
                let ly = ry.saturating_add(row_inner_h.saturating_sub(lh) / 2);
                blit_bgra_straight_alpha(fb, lx, ly, label, lw, lh);
            }
        }
    }
}

#[must_use]
pub fn context_menu_panel_contains(
    px: u32,
    py: u32,
    anchor_x: u32,
    anchor_y: u32,
    surf_w: u32,
    surf_h: u32,
) -> bool {
    let w = CONTEXT_MENU_W.min(surf_w.saturating_sub(4));
    let h = CONTEXT_MENU_ROW_H * CONTEXT_MENU_ROWS as u32;
    let x = anchor_x.min(surf_w.saturating_sub(w));
    let y = anchor_y.min(surf_h.saturating_sub(h));
    px >= x && py >= y && px < x + w && py < y + h
}

#[must_use]
pub fn context_menu_hit_row(
    px: u32,
    py: u32,
    anchor_x: u32,
    anchor_y: u32,
    surf_w: u32,
    surf_h: u32,
) -> Option<usize> {
    let w = CONTEXT_MENU_W.min(surf_w.saturating_sub(4));
    let h = CONTEXT_MENU_ROW_H * CONTEXT_MENU_ROWS as u32;
    let x = anchor_x.min(surf_w.saturating_sub(w));
    let y = anchor_y.min(surf_h.saturating_sub(h));
    if px < x + 2 || px >= x + w.saturating_sub(2) || py < y + 2 || py >= y + h {
        return None;
    }
    let rel = py - y - 2;
    let row = (rel / CONTEXT_MENU_ROW_H) as usize;
    if row < CONTEXT_MENU_ROWS {
        Some(row)
    } else {
        None
    }
}

pub const TASKBAR_CTX_MENU_ROW_H: u32 = 40;
pub const TASKBAR_CTX_MENU_ROWS: usize = 4;
pub const TASKBAR_CTX_MENU_W: u32 = 240;

const TB_CTX_LABELS: [&[u8]; TASKBAR_CTX_MENU_ROWS] = [
    b"Task Manager",
    b"Cascade windows (stub)",
    b"Show desktop",
    b"Taskbar settings (stub)",
];

/// Right-click menu on the taskbar (`references/win32/desktop-src/shell/taskbar.md`).
pub fn paint_taskbar_context_menu(
    fb: &FramebufferInfo,
    anchor_x: u32,
    anchor_y: u32,
    selected: usize,
) {
    let surf_w = fb.horizontal_resolution;
    let surf_h = fb.vertical_resolution;
    if fb.base == 0 || surf_w == 0 || surf_h == 0 {
        return;
    }
    let w = TASKBAR_CTX_MENU_W.min(surf_w.saturating_sub(4));
    let h = TASKBAR_CTX_MENU_ROW_H * TASKBAR_CTX_MENU_ROWS as u32;
    let x = anchor_x.min(surf_w.saturating_sub(w));
    let y = anchor_y.min(surf_h.saturating_sub(h));
    fill_rect_bgra(fb, x, y, w, h, 0x2a, 0x2a, 0x34);
    fill_rect_bgra(fb, x, y, w, 2, 0x00, 0x78, 0xd7);
    for i in 0..TASKBAR_CTX_MENU_ROWS {
        let ry = y + 2 + (i as u32) * TASKBAR_CTX_MENU_ROW_H;
        let sel = i == selected.min(TASKBAR_CTX_MENU_ROWS - 1);
        let (r, g, b) = if sel {
            (0x00, 0x78, 0xd7)
        } else {
            (0x36, 0x36, 0x40)
        };
        fill_rect_bgra(
            fb,
            x + 2,
            ry,
            w.saturating_sub(4),
            TASKBAR_CTX_MENU_ROW_H.saturating_sub(2),
            r,
            g,
            b,
        );
        let pad_x = 10u32;
        let row_inner_h = TASKBAR_CTX_MENU_ROW_H.saturating_sub(2);
        let glyph_h = 8u32 * 2;
        let ly = ry.saturating_add(row_inner_h.saturating_sub(glyph_h) / 2);
        draw_ascii_line_clipped(
            fb,
            x + pad_x,
            ly,
            w.saturating_sub(4).saturating_sub(pad_x * 2),
            TB_CTX_LABELS[i],
            0xf8,
            0xf8,
            0xff,
            2,
        );
    }
}

#[must_use]
pub fn taskbar_ctx_panel_contains(
    px: u32,
    py: u32,
    anchor_x: u32,
    anchor_y: u32,
    surf_w: u32,
    surf_h: u32,
) -> bool {
    let w = TASKBAR_CTX_MENU_W.min(surf_w.saturating_sub(4));
    let h = TASKBAR_CTX_MENU_ROW_H * TASKBAR_CTX_MENU_ROWS as u32;
    let x = anchor_x.min(surf_w.saturating_sub(w));
    let y = anchor_y.min(surf_h.saturating_sub(h));
    px >= x && py >= y && px < x + w && py < y + h
}

#[must_use]
pub fn taskbar_ctx_hit_row(
    px: u32,
    py: u32,
    anchor_x: u32,
    anchor_y: u32,
    surf_w: u32,
    surf_h: u32,
) -> Option<usize> {
    let w = TASKBAR_CTX_MENU_W.min(surf_w.saturating_sub(4));
    let h = TASKBAR_CTX_MENU_ROW_H * TASKBAR_CTX_MENU_ROWS as u32;
    let x = anchor_x.min(surf_w.saturating_sub(w));
    let y = anchor_y.min(surf_h.saturating_sub(h));
    if px < x + 2 || px >= x + w.saturating_sub(2) || py < y + 2 || py >= y + h {
        return None;
    }
    let rel = py - y - 2;
    let row = (rel / TASKBAR_CTX_MENU_ROW_H) as usize;
    if row < TASKBAR_CTX_MENU_ROWS {
        Some(row)
    } else {
        None
    }
}

const FLYOUT_W: u32 = 280;
const FLYOUT_H: u32 = 118;

fn flyout_panel_xy(anchor_x: u32, anchor_y: u32, surf_w: u32, surf_h: u32) -> (u32, u32) {
    let w = FLYOUT_W.min(surf_w.saturating_sub(4));
    let h = FLYOUT_H.min(surf_h.saturating_sub(4));
    let x = anchor_x.saturating_sub(w / 2).min(surf_w.saturating_sub(w));
    let y = anchor_y.saturating_sub(h.saturating_add(6)).min(surf_h.saturating_sub(h));
    (x, y)
}

/// Flyout above the notification area (`references/win32/desktop-src/shell/notification-area.md`).
pub fn paint_notification_flyout(fb: &FramebufferInfo, kind: u8, anchor_x: u32, anchor_y: u32) {
    if kind == FLYOUT_KIND_NONE || fb.base == 0 {
        return;
    }
    let surf_w = fb.horizontal_resolution;
    let surf_h = fb.vertical_resolution;
    let (x, y) = flyout_panel_xy(anchor_x, anchor_y, surf_w, surf_h);
    let w = FLYOUT_W.min(surf_w.saturating_sub(4));
    let h = FLYOUT_H.min(surf_h.saturating_sub(4));
    fill_rect_bgra(fb, x, y, w, h, 0x2e, 0x2e, 0x38);
    fill_rect_bgra(fb, x, y, w, 2, 0x00, 0x78, 0xd7);
    let body: &[u8] = match kind {
        FLYOUT_KIND_CALENDAR_STUB => b"Calendar (stub)",
        FLYOUT_KIND_VOLUME_STUB => b"Volume (stub)",
        _ => &[],
    };
    draw_ascii_line_clipped(fb, x + 12, y + 14, w - 24, body, 0xf8, 0xf8, 0xff, 2);
    draw_ascii_line_clipped(
        fb,
        x + 12,
        y + 14 + 8 * 2 + 10,
        w - 24,
        b"Tray / clock flyout",
        0xc8,
        0xc8,
        0xd4,
        2,
    );
}

#[must_use]
pub fn flyout_panel_contains(
    kind: u8,
    anchor_x: u32,
    anchor_y: u32,
    px: u32,
    py: u32,
    surf_w: u32,
    surf_h: u32,
) -> bool {
    if kind == FLYOUT_KIND_NONE {
        return false;
    }
    let (x, y) = flyout_panel_xy(anchor_x, anchor_y, surf_w, surf_h);
    let w = FLYOUT_W.min(surf_w.saturating_sub(4));
    let h = FLYOUT_H.min(surf_h.saturating_sub(4));
    px >= x && py >= y && px < x + w && py < y + h
}

/// Taskbar: Start, task slots, notification tray placeholders, clock, show-desktop sliver.
/// Layout aligns with `references/win32/desktop-src/shell/taskbar.md` / `uxguide/winenv-taskbar.md`.
pub fn paint_desktop_chrome(
    fb: &FramebufferInfo,
    layout: &TaskbarLayout,
    st: &DesktopChromeState,
    stack: &WindowStack,
) {
    let w = fb.horizontal_resolution;
    let h = fb.vertical_resolution;
    if w == 0 || h == 0 {
        return;
    }

    let bar = layout.bar;
    fill_rect_bgra(fb, bar.x, bar.y, bar.w, bar.h, 0x20, 0x20, 0x28);

    let sb = layout.start_button;
    let (br, bg, bb) = if st.menu_open {
        (0x50, 0xc8, 0xff)
    } else {
        (0x00, 0x99, 0xff)
    };
    fill_rect_bgra(fb, sb.x, sb.y, sb.w, sb.h, br, bg, bb);
    let inset = 8u32;
    if sb.w > inset * 2 && sb.h > inset * 2 {
        fill_rect_bgra(
            fb,
            sb.x + inset,
            sb.y + inset,
            sb.w - inset * 2,
            sb.h - inset * 2,
            0xff,
            0xff,
            0xff,
        );
    }

    let slots = layout.task_slots();
    let top_u8 = if stack.len > 0 {
        Some(stack.ids[(stack.len - 1) as usize].to_u8())
    } else {
        None
    };
    for s in 0..TASK_SLOT_COUNT {
        let r = slots[s];
        if let Some(aid) = hosted_apps::task_slot_app(stack, s) {
            let active = top_u8 == Some(aid.to_u8());
            let (cr, cg, cb) = if active {
                (0x00u8, 0x78u8, 0xd7u8)
            } else {
                (0x2cu8, 0x2cu8, 0x36u8)
            };
            fill_rect_bgra(fb, r.x, r.y, r.w, r.h, cr, cg, cb);
            let letter = match aid {
                super::app_host::AppId::Files => b'F',
                super::app_host::AppId::TaskMgr => b'T',
                super::app_host::AppId::Settings => b'S',
                super::app_host::AppId::ControlPanel => b'C',
                super::app_host::AppId::Run => b'R',
                super::app_host::AppId::Notepad => b'N',
                super::app_host::AppId::Calculator => b'K',
                super::app_host::AppId::About => b'A',
                super::app_host::AppId::Properties => b'P',
            };
            let cx = r.x + r.w / 2 - 4;
            let cy = r.y + r.h / 2 - 4;
            blit_glyph_mono8(fb, cx, cy, letter, 0xf8, 0xf8, 0xfc, 1);
        } else {
            fill_rect_bgra(fb, r.x, r.y, r.w, r.h, 0x1c, 0x1c, 0x26);
        }
    }

    let ck = layout.clock_area();
    fill_rect_bgra(fb, ck.x, ck.y, ck.w, ck.h, 0x14, 0x14, 0x1c);

    for i in 0..TRAY_ICON_COUNT {
        if let Some(r) = layout.tray_icon_rect(i) {
            let (cr, cg, cb) = match i {
                0 => (0x4c, 0xb4, 0xff),
                1 => (0x6c, 0xd9, 0x7a),
                _ => (0xff, 0xd4, 0x6c),
            };
            fill_rect_bgra(fb, r.x, r.y, r.w, r.h, 0x28, 0x28, 0x32);
            let inset = 3u32;
            if r.w > inset * 2 && r.h > inset * 2 {
                fill_rect_bgra(
                    fb,
                    r.x + inset,
                    r.y + inset,
                    r.w - inset * 2,
                    r.h - inset * 2,
                    cr,
                    cg,
                    cb,
                );
            }
        }
    }

    let cd = layout.clock_display_area();
    let tn = st.clock_time_n as usize;
    let dn = st.clock_date_n as usize;
    if tn > 0 && tn <= st.clock_time.len() {
        draw_ascii_line_clipped(
            fb,
            cd.x + 2,
            cd.y + 2,
            cd.w.saturating_sub(4),
            &st.clock_time[..tn],
            0xfc,
            0xfc,
            0xff,
            2,
        );
    }
    if dn > 0 && dn <= st.clock_date.len() {
        let date_y = cd.y.saturating_add(2 + 8 * 2 + 2);
        draw_ascii_line_clipped(
            fb,
            cd.x + 2,
            date_y,
            cd.w.saturating_sub(4),
            &st.clock_date[..dn],
            0xc8,
            0xc8,
            0xd4,
            1,
        );
    }

    let sd = layout.show_desktop_corner();
    fill_rect_bgra(fb, sd.x, sd.y, sd.w, sd.h, 0x3a, 0x3a, 0x44);
}

/// Blend BGRA8 **straight-alpha** sprite onto the framebuffer (`dst` treated as opaque BGRx).
pub fn blit_bgra_straight_alpha(
    fb: &FramebufferInfo,
    dst_x: u32,
    dst_y: u32,
    src: &[u8],
    src_w: u32,
    src_h: u32,
) {
    if fb.base == 0 || src_w == 0 || src_h == 0 {
        return;
    }
    let need = (src_w as usize)
        .checked_mul(src_h as usize)
        .and_then(|n| n.checked_mul(4))
        .unwrap_or(0);
    if src.len() < need {
        return;
    }
    let surf_w = fb.horizontal_resolution;
    let surf_h = fb.vertical_resolution;
    let stride_px = fb.pixels_per_scan_line as usize;
    let ptr = fb.base as *mut u8;
    let byte_cap = display_mgr::framebuffer_linear_byte_cap(fb);
    for sy in 0..src_h {
        let dy = dst_y.saturating_add(sy);
        if dy >= surf_h {
            break;
        }
        for sx in 0..src_w {
            let dx = dst_x.saturating_add(sx);
            if dx >= surf_w {
                break;
            }
            let si = ((sy * src_w + sx) * 4) as usize;
            if si + 4 > src.len() {
                return;
            }
            let sb = src[si] as u32;
            let sg = src[si + 1] as u32;
            let sr = src[si + 2] as u32;
            let sa = src[si + 3] as u32;
            if sa == 0 {
                continue;
            }
            let off = (dy as usize * stride_px + dx as usize) * 4;
            if off + 4 > byte_cap {
                continue;
            }
            unsafe {
                let p = ptr.add(off);
                if sa >= 255 {
                    display_mgr::fb_write_opaque_rgb8(fb, p, sr as u8, sg as u8, sb as u8);
                } else {
                    let inv = 255 - sa;
                    let (dr, dg, db) = display_mgr::fb_read_rgb8(fb, p);
                    let or = (sr * sa + dr as u32 * inv) / 255;
                    let og = (sg * sa + dg as u32 * inv) / 255;
                    let ob = (sb * sa + db as u32 * inv) / 255;
                    display_mgr::fb_write_opaque_rgb8(fb, p, or as u8, og as u8, ob as u8);
                }
            }
        }
    }
}

/// Start menu overlay (solid rows + Zircon Fluent icons; no font renderer).
pub fn paint_start_menu_overlay(
    fb: &FramebufferInfo,
    layout: &TaskbarLayout,
    open: bool,
    selected: usize,
) {
    if !open {
        return;
    }
    let sm = layout.start_menu();
    let p = sm.panel;
    fill_rect_bgra(fb, p.x, p.y, p.w, p.h, 0x22, 0x22, 0x2c);
    // border
    fill_rect_bgra(fb, p.x, p.y, p.w, 2, 0x00, 0x78, 0xd7);
    let max_sel = START_MENU_ROW_COUNT.saturating_sub(1);
    for i in 0..START_MENU_ROW_COUNT {
        let r = sm.items[i];
        let sel = i == selected.min(max_sel);
        let (cr, cg, cb) = if sel {
            (0x00, 0x78, 0xd7)
        } else {
            (0x32, 0x32, 0x3c)
        };
        fill_rect_bgra(fb, r.x, r.y, r.w, r.h, cr, cg, cb);
        if let Some(icon) = start_menu_icon_bgra(i) {
            let iy = r.y + r.h.saturating_sub(START_MENU_ICON_H) / 2;
            let ix = r.x.saturating_add(8);
            blit_bgra_straight_alpha(fb, ix, iy, icon, START_MENU_ICON_W, START_MENU_ICON_H);
        }
        let lx = r.x + START_MENU_ICON_W + 16;
        let max_lw = r.w.saturating_sub(START_MENU_ICON_W + 24);
        if let Some((label, lw, lh)) = start_menu_label_bgra(i) {
            if lw <= max_lw {
                let ly = r.y.saturating_add(r.h.saturating_sub(lh) / 2);
                blit_bgra_straight_alpha(fb, lx, ly, label, lw, lh);
            }
        }
    }
}

fn paint_power_confirm(fb: &FramebufferInfo, layout: &TaskbarLayout) {
    let w = layout.surface_w;
    let h = layout.bar.y;
    let dw = 320u32.min(w.saturating_sub(40));
    let dh = 120u32.min(h.saturating_sub(40));
    let x = (w.saturating_sub(dw)) / 2;
    let y = (h.saturating_sub(dh)) / 2;
    fill_rect_bgra(fb, x, y, dw, dh, 0x2e, 0x2e, 0x38);
    fill_rect_bgra(fb, x, y, dw, 2, 0x00, 0x78, 0xd7);
    draw_ascii_line_clipped(
        fb,
        x + 16,
        y + 24,
        dw - 32,
        b"Power actions use firmware.",
        0xe8,
        0xe8,
        0xf0,
        1,
    );
    draw_ascii_line_clipped(
        fb,
        x + 16,
        y + 44,
        dw - 32,
        b"(stub - no shutdown from shell)",
        0xb0,
        0xb0,
        0xb8,
        1,
    );
}

/// Full desktop background + chrome + Start / hosted windows / context menu (no cursor).
pub fn redraw_uefi_desktop(
    fb: &FramebufferInfo,
    layout: &TaskbarLayout,
    st: &DesktopChromeState,
    stack: &WindowStack,
    ui: &HostUiState,
) {
    paint_wallpaper_only(fb);
    paint_desktop_shortcuts(fb, layout);
    paint_desktop_chrome(fb, layout, st, stack);
    paint_start_menu_overlay(fb, layout, st.menu_open, st.menu_sel);
    hosted_apps::paint_window_stack(fb, layout, stack, ui);
    if st.power_confirm_open {
        paint_power_confirm(fb, layout);
    }
    if st.tb_ctx_open {
        paint_taskbar_context_menu(fb, st.tb_ctx_x, st.tb_ctx_y, st.tb_ctx_sel);
    }
    if st.ctx_open {
        paint_context_menu(fb, st.ctx_x, st.ctx_y, st.ctx_sel);
    }
    if st.flyout != FLYOUT_KIND_NONE {
        paint_notification_flyout(fb, st.flyout, st.flyout_x, st.flyout_y);
    }
    display_mgr::framebuffer_store_fence();
}

/// Full desktop redraw **without** [`paint_wallpaper_only`] — used when the Win32 wallpaper HWND
/// composites the resource wallpaper as the bottom Z layer (Phase 5 single-source path).
pub fn redraw_uefi_desktop_skip_wallpaper(
    fb: &FramebufferInfo,
    layout: &TaskbarLayout,
    st: &DesktopChromeState,
    stack: &WindowStack,
    ui: &HostUiState,
) {
    paint_desktop_shortcuts(fb, layout);
    paint_desktop_chrome(fb, layout, st, stack);
    paint_start_menu_overlay(fb, layout, st.menu_open, st.menu_sel);
    hosted_apps::paint_window_stack(fb, layout, stack, ui);
    if st.power_confirm_open {
        paint_power_confirm(fb, layout);
    }
    if st.tb_ctx_open {
        paint_taskbar_context_menu(fb, st.tb_ctx_x, st.tb_ctx_y, st.tb_ctx_sel);
    }
    if st.ctx_open {
        paint_context_menu(fb, st.ctx_x, st.ctx_y, st.ctx_sel);
    }
    if st.flyout != FLYOUT_KIND_NONE {
        paint_notification_flyout(fb, st.flyout, st.flyout_x, st.flyout_y);
    }
    display_mgr::framebuffer_store_fence();
}

/// Composite on top of a cached tight base layer (wallpaper + desktop shortcuts only).
pub fn redraw_uefi_desktop_from_base_cache(
    fb: &FramebufferInfo,
    base_tight: &[u8],
    base_w: u32,
    base_h: u32,
    layout: &TaskbarLayout,
    st: &DesktopChromeState,
    stack: &WindowStack,
    ui: &HostUiState,
) {
    blit_tight_bgra_to_framebuffer(base_tight, base_w, base_h, fb);
    paint_desktop_chrome(fb, layout, st, stack);
    paint_start_menu_overlay(fb, layout, st.menu_open, st.menu_sel);
    hosted_apps::paint_window_stack(fb, layout, stack, ui);
    if st.power_confirm_open {
        paint_power_confirm(fb, layout);
    }
    if st.tb_ctx_open {
        paint_taskbar_context_menu(fb, st.tb_ctx_x, st.tb_ctx_y, st.tb_ctx_sel);
    }
    if st.ctx_open {
        paint_context_menu(fb, st.ctx_x, st.ctx_y, st.ctx_sel);
    }
    if st.flyout != FLYOUT_KIND_NONE {
        paint_notification_flyout(fb, st.flyout, st.flyout_x, st.flyout_y);
    }
    display_mgr::framebuffer_store_fence();
}

#[must_use]
pub fn hit_test_clock_display(layout: &TaskbarLayout, px: u32, py: u32) -> bool {
    layout.clock_display_area().contains(px, py)
}

#[must_use]
pub fn hit_test_tray_volume(layout: &TaskbarLayout, px: u32, py: u32) -> bool {
    layout
        .tray_icon_rect(1)
        .is_some_and(|r| r.contains(px, py))
}

#[must_use]
pub fn hit_test_show_desktop_corner(layout: &TaskbarLayout, px: u32, py: u32) -> bool {
    layout.show_desktop_corner().contains(px, py)
}

/// One-shot Fluent-style desktop (menu closed).
pub fn paint_uefi_desktop_shell(fb: &FramebufferInfo) {
    let w = fb.horizontal_resolution;
    let h = fb.vertical_resolution;
    if w == 0 || h == 0 {
        return;
    }
    let layout = TaskbarLayout::for_surface(w, h);
    let stack = WindowStack::new();
    let ui = HostUiState::default();
    redraw_uefi_desktop(fb, &layout, &DesktopChromeState::default(), &stack, &ui);
}
