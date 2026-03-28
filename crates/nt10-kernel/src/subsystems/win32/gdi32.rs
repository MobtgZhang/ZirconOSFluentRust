//! gdi32 — device context stubs for GOP-era software rendering.

#[derive(Clone, Copy, Debug)]
pub struct GdiDcStub {
    pub fb_base: u64,
    pub width: u32,
    pub height: u32,
    pub stride_px: u32,
}

impl GdiDcStub {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            fb_base: 0,
            width: 0,
            height: 0,
            stride_px: 0,
        }
    }

    pub fn attach_framebuffer(&mut self, base: u64, w: u32, h: u32, stride: u32) {
        self.fb_base = base;
        self.width = w;
        self.height = h;
        self.stride_px = stride;
    }

    /// Fill rectangle in BGRA bytes (`buf` is a mapped linear FB slice).
    pub fn fill_rect_bgra(
        &self,
        buf: &mut [u8],
        x0: u32,
        y0: u32,
        w: u32,
        h: u32,
        bgra: [u8; 4],
    ) -> Result<(), ()> {
        let stride = self.stride_px as usize;
        if w == 0 || h == 0 || self.width == 0 || self.height == 0 {
            return Ok(());
        }
        for row in y0..y0.saturating_add(h).min(self.height) {
            let rbase = row as usize * stride * 4;
            for col in x0..x0.saturating_add(w).min(self.width) {
                let i = rbase + col as usize * 4;
                if i + 4 > buf.len() {
                    return Err(());
                }
                buf[i..i + 4].copy_from_slice(&bgra);
            }
        }
        Ok(())
    }

    /// Copy a tight BGRA rectangle from `src` (`src_w`×`src_h` pixels) into this framebuffer view.
    pub fn bit_blt_bgra(
        &self,
        dst_buf: &mut [u8],
        dx: u32,
        dy: u32,
        src: &[u8],
        src_w: u32,
        src_h: u32,
        sx: u32,
        sy: u32,
        w: u32,
        h: u32,
    ) -> Result<(), ()> {
        let stride = self.stride_px as usize;
        for row in 0..h {
            let sy_ = sy + row;
            let dy_ = dy + row;
            if sy_ >= src_h || dy_ >= self.height {
                break;
            }
            for col in 0..w {
                let sx_ = sx + col;
                let dx_ = dx + col;
                if sx_ >= src_w || dx_ >= self.width {
                    continue;
                }
                let si = ((sy_ * src_w + sx_) * 4) as usize;
                let di = (dy_ as usize * stride + dx_ as usize) * 4;
                if si + 4 > src.len() || di + 4 > dst_buf.len() {
                    return Err(());
                }
                dst_buf[di..di + 4].copy_from_slice(&src[si..si + 4]);
            }
        }
        Ok(())
    }

    /// Outline rectangle (1px) in BGRA.
    pub fn frame_rect_bgra(
        &self,
        buf: &mut [u8],
        x0: u32,
        y0: u32,
        w: u32,
        h: u32,
        bgra: [u8; 4],
    ) -> Result<(), ()> {
        if w == 0 || h == 0 {
            return Ok(());
        }
        self.fill_rect_bgra(buf, x0, y0, w, 1, bgra)?;
        self.fill_rect_bgra(buf, x0, y0.saturating_add(h.saturating_sub(1)), w, 1, bgra)?;
        self.fill_rect_bgra(buf, x0, y0, 1, h, bgra)?;
        self.fill_rect_bgra(buf, x0.saturating_add(w.saturating_sub(1)), y0, 1, h, bgra)?;
        Ok(())
    }
}

/// Double-buffer helper: memory DC backed by caller-owned BGRA (`width`×`height` tight).
#[derive(Clone, Copy, Debug)]
pub struct GdiMemDc {
    pub width: u32,
    pub height: u32,
}

impl GdiMemDc {
    #[must_use]
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    #[must_use]
    pub fn byte_len(self) -> usize {
        self.width as usize * self.height as usize * 4
    }

    /// Blit full mem buffer to screen DC region `(dx,dy)`.
    pub fn flush_to_screen_dc(
        self,
        screen: &GdiDcStub,
        mem: &[u8],
        dst_buf: &mut [u8],
        dx: u32,
        dy: u32,
    ) -> Result<(), ()> {
        let need = self.byte_len();
        if mem.len() < need {
            return Err(());
        }
        screen.bit_blt_bgra(
            dst_buf,
            dx,
            dy,
            mem,
            self.width,
            self.height,
            0,
            0,
            self.width,
            self.height,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_rect_writes_corner() {
        let mut dc = GdiDcStub::new();
        dc.attach_framebuffer(0, 2, 2, 2);
        let mut px = [0u8; 16];
        assert!(dc
            .fill_rect_bgra(&mut px, 0, 0, 1, 1, [1, 2, 3, 4])
            .is_ok());
        assert_eq!(px[0..4], [1, 2, 3, 4]);
    }

    #[test]
    fn bit_blt_copies_pixel() {
        let mut screen = GdiDcStub::new();
        screen.attach_framebuffer(0, 2, 2, 2);
        let mut fb = [0u8; 16];
        let src = [10u8, 20, 30, 40, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert!(screen
            .bit_blt_bgra(&mut fb, 1, 0, &src, 2, 2, 0, 0, 1, 1)
            .is_ok());
        assert_eq!(fb[4..8], [10, 20, 30, 40]);
    }

    #[test]
    fn frame_rect_outlines() {
        let mut dc = GdiDcStub::new();
        dc.attach_framebuffer(0, 3, 3, 3);
        let mut px = [0u8; 36];
        assert!(dc
            .frame_rect_bgra(&mut px, 0, 0, 3, 3, [9, 9, 9, 9])
            .is_ok());
        assert_eq!(px[0], 9);
        assert_eq!(px[(2 * 3 + 2) * 4], 9);
    }
}
