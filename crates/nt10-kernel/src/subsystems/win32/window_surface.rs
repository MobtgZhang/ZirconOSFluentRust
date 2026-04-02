//! Per-window-slot BGRA offscreen buffers (Phase 4 bring-up; fixed size, no pool allocator).

use crate::ke::spinlock::SpinLock;
use crate::ob::winsta::MAX_DESKTOP_WINDOWS;

/// Surface width/height for each desktop window slot (bring-up).
pub const SURF_W: u32 = 128;
pub const SURF_H: u32 = 32;
/// Tight BGRA bytes per slot.
pub const SURF_BYTES: usize = (SURF_W * SURF_H * 4) as usize;

static POOL: SpinLock<[[u8; SURF_BYTES]; MAX_DESKTOP_WINDOWS]> =
    SpinLock::new([[0u8; SURF_BYTES]; MAX_DESKTOP_WINDOWS]);

/// Downsample tight BGRA (`src_w`×`src_h`) into slot via nearest neighbor (wallpaper → 128×32 bring-up).
pub fn downsample_bgra_nearest_to_slot(
    slot: usize,
    src: &[u8],
    src_w: u32,
    src_h: u32,
) -> Result<(), ()> {
    if slot >= MAX_DESKTOP_WINDOWS || src_w == 0 || src_h == 0 {
        return Err(());
    }
    let need = (src_w as usize)
        .checked_mul(src_h as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or(())?;
    if src.len() < need {
        return Err(());
    }
    let mut g = POOL.lock();
    let buf = &mut g[slot];
    let dw = SURF_W;
    let dh = SURF_H;
    let sw = src_w as u64;
    let sh = src_h as u64;
    for dy in 0..dh {
        let sy = ((dy as u64 * sh) / dh as u64).min(sh.saturating_sub(1));
        for dx in 0..dw {
            let sx = ((dx as u64 * sw) / dw as u64).min(sw.saturating_sub(1));
            let si = ((sy * src_w as u64 + sx) * 4) as usize;
            let di = ((dy * SURF_W + dx) * 4) as usize;
            if si + 4 > src.len() || di + 4 > buf.len() {
                return Err(());
            }
            buf[di..di + 4].copy_from_slice(&src[si..si + 4]);
            buf[di + 3] = 0xff;
        }
    }
    Ok(())
}

/// Clear slot pixels to transparent black.
pub fn clear_surface(slot: usize) {
    if slot >= MAX_DESKTOP_WINDOWS {
        return;
    }
    POOL.lock()[slot].fill(0);
}

/// Fill entire slot with solid BGRA.
pub fn fill_surface_solid(slot: usize, bgra: [u8; 4]) {
    if slot >= MAX_DESKTOP_WINDOWS {
        return;
    }
    let mut g = POOL.lock();
    let b = &mut g[slot];
    let mut i = 0usize;
    while i + 4 <= b.len() {
        b[i..i + 4].copy_from_slice(&bgra);
        i += 4;
    }
}

/// Fill axis-aligned rectangle in slot (surface pixel space).
pub fn fill_rect_surface(slot: usize, x0: u32, y0: u32, w: u32, h: u32, bgra: [u8; 4]) {
    if slot >= MAX_DESKTOP_WINDOWS || w == 0 || h == 0 {
        return;
    }
    let stride = SURF_W as usize;
    let mut g = POOL.lock();
    let buf = &mut g[slot];
    for row in y0..y0.saturating_add(h).min(SURF_H) {
        let rbase = row as usize * stride * 4;
        for col in x0..x0.saturating_add(w).min(SURF_W) {
            let i = rbase + col as usize * 4;
            if i + 4 <= buf.len() {
                buf[i..i + 4].copy_from_slice(&bgra);
            }
        }
    }
}

/// Straight-alpha src-over onto a BGRX-style destination (alpha byte forced opaque after blend).
#[inline]
pub fn blend_src_over_bgra(dst_px: &mut [u8], src: [u8; 4]) {
    if dst_px.len() < 4 {
        return;
    }
    let sa = src[3] as u32;
    if sa == 0 {
        return;
    }
    if sa >= 255 {
        dst_px[..4].copy_from_slice(&src);
        dst_px[3] = 0xff;
        return;
    }
    let inv = 255u32 - sa;
    for i in 0..3 {
        dst_px[i] = ((src[i] as u32 * sa + dst_px[i] as u32 * inv + 127) / 255) as u8;
    }
    dst_px[3] = 0xff;
}

/// Blit one slot to a linear BGRA framebuffer (same layout as [`super::gdi32::GdiDcStub::bit_blt_bgra`]).
pub fn blit_slot_to_framebuffer(
    slot: usize,
    dst_buf: &mut [u8],
    dst_w: u32,
    dst_h: u32,
    stride_px: u32,
    dx: u32,
    dy: u32,
) -> Result<(), ()> {
    if slot >= MAX_DESKTOP_WINDOWS {
        return Err(());
    }
    let src = POOL.lock();
    let mem = &src[slot];
    let stride = stride_px as usize;
    let src_w = SURF_W;
    let src_h = SURF_H;
    for row in 0..src_h {
        let dy_ = dy + row;
        if dy_ >= dst_h {
            break;
        }
        for col in 0..src_w {
            let dx_ = dx + col;
            if dx_ >= dst_w {
                continue;
            }
            let si = ((row * src_w + col) * 4) as usize;
            let di = (dy_ as usize * stride + dx_ as usize) * 4;
            if si + 4 > mem.len() || di + 4 > dst_buf.len() {
                return Err(());
            }
            let px = [mem[si], mem[si + 1], mem[si + 2], mem[si + 3]];
            blend_src_over_bgra(&mut dst_buf[di..di + 4], px);
        }
    }
    Ok(())
}

/// Nearest-neighbor stretch of slot surface into a screen rectangle with src-over blending.
pub fn blit_slot_stretch_to_framebuffer_src_over(
    slot: usize,
    dst_buf: &mut [u8],
    dst_w: u32,
    dst_h: u32,
    stride_px: u32,
    dx: u32,
    dy: u32,
    dw: u32,
    dh: u32,
) -> Result<(), ()> {
    if slot >= MAX_DESKTOP_WINDOWS || dw == 0 || dh == 0 {
        return Err(());
    }
    let src = POOL.lock();
    let mem = &src[slot];
    let stride = stride_px as usize;
    let sw = SURF_W as u64;
    let sh = SURF_H as u64;
    for iy in 0..dh {
        let py = dy.saturating_add(iy);
        if py >= dst_h {
            break;
        }
        let sy = ((iy as u64 * sh) / dh as u64).min(sh.saturating_sub(1));
        for ix in 0..dw {
            let px = dx.saturating_add(ix);
            if px >= dst_w {
                continue;
            }
            let sx = ((ix as u64 * sw) / dw as u64).min(sw.saturating_sub(1));
            let si = ((sy * SURF_W as u64 + sx) * 4) as usize;
            let di = (py as usize * stride + px as usize) * 4;
            if si + 4 > mem.len() || di + 4 > dst_buf.len() {
                return Err(());
            }
            let bgra = [mem[si], mem[si + 1], mem[si + 2], mem[si + 3]];
            blend_src_over_bgra(&mut dst_buf[di..di + 4], bgra);
        }
    }
    Ok(())
}

/// Read-only access for tests.
#[cfg(test)]
pub fn surface_pixel(slot: usize, x: u32, y: u32) -> Option<[u8; 4]> {
    if slot >= MAX_DESKTOP_WINDOWS || x >= SURF_W || y >= SURF_H {
        return None;
    }
    let g = POOL.lock();
    let i = (y as usize * SURF_W as usize + x as usize) * 4;
    let b = &g[slot];
    if i + 4 > b.len() {
        return None;
    }
    Some([b[i], b[i + 1], b[i + 2], b[i + 3]])
}
