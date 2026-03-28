//! Display manager — bridges UEFI GOP handoff to Win32k / WDDM2 (`wddm2/`).
//!
//! **VSync / input timing**: UEFI bring-up polls HID each frame without vertical sync; when a real
//! flip queue exists (`wddm2/dxgkrnl`), align pointer sampling with present completion to reduce tear
//! (`extensions/phase-05-input-stack.md`).

use nt10_boot_protocol::FramebufferInfo;

use crate::hal::Hal;

/// UEFI `EFI_GRAPHICS_PIXEL_FORMAT`: R,G,B,Reserved 8 bpp per channel.
pub const GOP_PIXEL_RED_GREEN_BLUE_RESERVED_8: u32 = 0;
/// UEFI `EFI_GRAPHICS_PIXEL_FORMAT`: B,G,R,Reserved 8 bpp per channel (typical QEMU/OVMF).
pub const GOP_PIXEL_BLUE_GREEN_RED_RESERVED_8: u32 = 1;
/// UEFI `PixelBitMask` — channel order is firmware-defined; bring-up assumes 32 bpp BGRx layout like BGR8.
pub const GOP_PIXEL_BIT_MASK: u32 = 2;

#[inline]
#[must_use]
pub const fn gop_pixel_is_rgb_first(pixel_format: u32) -> bool {
    pixel_format == GOP_PIXEL_RED_GREEN_BLUE_RESERVED_8
}

/// Write one **opaque** 32-bit GOP pixel. `r`, `g`, `b` are logical sRGB-style components.
///
/// # Safety
/// `p` must point to four writable bytes within the linear frame buffer.
#[inline]
pub unsafe fn fb_write_opaque_rgb8(fb: &FramebufferInfo, p: *mut u8, r: u8, g: u8, b: u8) {
    if gop_pixel_is_rgb_first(fb.pixel_format) {
        p.write_volatile(r);
        p.add(1).write_volatile(g);
        p.add(2).write_volatile(b);
    } else {
        p.write_volatile(b);
        p.add(1).write_volatile(g);
        p.add(2).write_volatile(r);
    }
    p.add(3).write_volatile(0xff);
}

/// Read logical `(R, G, B)` from a 32-bit GOP pixel.
///
/// # Safety
/// `p` must point to four readable bytes within the linear frame buffer.
#[inline]
pub unsafe fn fb_read_rgb8(fb: &FramebufferInfo, p: *const u8) -> (u8, u8, u8) {
    if gop_pixel_is_rgb_first(fb.pixel_format) {
        (
            p.read_volatile(),
            p.add(1).read_volatile(),
            p.add(2).read_volatile(),
        )
    } else {
        (
            p.add(2).read_volatile(),
            p.add(1).read_volatile(),
            p.read_volatile(),
        )
    }
}

/// After a batch of GOP writes, drain CPU store buffers so scan-out sees pixels (WC framebuffers).
#[inline]
pub fn framebuffer_store_fence() {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("mfence", options(nostack, preserves_flags));
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        core::sync::atomic::fence(core::sync::atomic::Ordering::SeqCst);
    }
}

/// Why [`parse_framebuffer_handoff`] rejected the firmware block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FramebufferHandoffError {
    NullBase,
    ZeroSize,
    ZeroWidth,
    ZeroHeight,
    StrideTooSmall,
}

/// Normalized GOP / linear frame buffer parameters for software draw paths.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FramebufferHandoff {
    pub base_phys: u64,
    pub byte_len: usize,
    pub width_px: u32,
    pub height_px: u32,
    pub stride_px: u32,
    /// Opaque firmware value; interpret only in platform-specific code.
    pub pixel_format: u32,
}

/// Parse [`ZirconBootInfo::framebuffer`] (see `nt10-boot-protocol`) for bring-up drawing.
#[must_use]
pub fn parse_framebuffer_handoff(fb: &FramebufferInfo) -> Result<FramebufferHandoff, FramebufferHandoffError> {
    if fb.base == 0 {
        return Err(FramebufferHandoffError::NullBase);
    }
    if fb.size == 0 {
        return Err(FramebufferHandoffError::ZeroSize);
    }
    if fb.horizontal_resolution == 0 {
        return Err(FramebufferHandoffError::ZeroWidth);
    }
    if fb.vertical_resolution == 0 {
        return Err(FramebufferHandoffError::ZeroHeight);
    }
    if fb.pixels_per_scan_line < fb.horizontal_resolution {
        return Err(FramebufferHandoffError::StrideTooSmall);
    }
    Ok(FramebufferHandoff {
        base_phys: fb.base,
        byte_len: fb.size,
        width_px: fb.horizontal_resolution,
        height_px: fb.vertical_resolution,
        stride_px: fb.pixels_per_scan_line,
        pixel_format: fb.pixel_format,
    })
}

/// Linear frame buffer bytes addressable with `pixels_per_scan_line` × `vertical_resolution` × 4.
///
/// UEFI sometimes sets `FrameBufferSize` to `width×height×4` while `PixelsPerScanLine` is larger
/// (row padding). Code that stops at `fb.size` then **aborts** mid-blit; the pointer path used
/// `max(size, stride×h×4)` and could write a **different** region than wallpaper. Use this cap everywhere.
#[inline]
#[must_use]
pub fn framebuffer_linear_byte_cap(fb: &FramebufferInfo) -> usize {
    let layout = (fb.pixels_per_scan_line as usize)
        .saturating_mul(fb.vertical_resolution as usize)
        .saturating_mul(4);
    fb.size.max(layout)
}

/// Registers handoff dimensions for early drawing until a real KMD stack exists.
#[must_use]
pub fn register_uefi_framebuffer_stub(fb: &FramebufferInfo) -> bool {
    parse_framebuffer_handoff(fb).is_ok()
}

fn write_hex_u32<H: Hal + ?Sized>(hal: &H, mut v: u32) {
    hal.debug_write(b"0x");
    let mut buf = [0u8; 8];
    let mut i = buf.len();
    if v == 0 {
        i -= 1;
        buf[i] = b'0';
    } else {
        while v > 0 && i > 0 {
            i -= 1;
            let d = (v & 0xF) as u8;
            buf[i] = if d < 10 { b'0' + d } else { b'a' + (d - 10) };
            v >>= 4;
        }
    }
    hal.debug_write(&buf[i..]);
}

fn write_hex_u64<H: Hal + ?Sized>(hal: &H, mut v: u64) {
    hal.debug_write(b"0x");
    const DIG: &[u8; 16] = b"0123456789abcdef";
    let mut buf = [0u8; 16];
    let mut n = 0usize;
    if v == 0 {
        hal.debug_write(b"0");
        return;
    }
    while v > 0 && n < buf.len() {
        buf[n] = DIG[(v & 0xF) as usize];
        v >>= 4;
        n += 1;
    }
    for i in (0..n).rev() {
        hal.debug_write(core::slice::from_ref(&buf[i]));
    }
}

/// Serial diagnostics for UEFI GOP handoff (pixel format, stride). Non-BGR layout may look wrong with current BGRA blits.
pub fn log_uefi_framebuffer_diag<H: Hal + ?Sized>(hal: &H, fb: &FramebufferInfo) {
    hal.debug_write(b"nt10-kernel: GOP base_phys=");
    write_hex_u64(hal, fb.base);
    hal.debug_write(b" size=");
    let mut n = fb.size;
    let mut tmp = [0u8; 16];
    let mut j = tmp.len();
    if n == 0 {
        j -= 1;
        tmp[j] = b'0';
    } else {
        while n > 0 && j > 0 {
            j -= 1;
            tmp[j] = b'0' + (n % 10) as u8;
            n /= 10;
        }
    }
    hal.debug_write(&tmp[j..]);
    hal.debug_write(b" res=");
    log_u32_decimal(hal, fb.horizontal_resolution);
    hal.debug_write(b"x");
    log_u32_decimal(hal, fb.vertical_resolution);
    hal.debug_write(b" ppsl=");
    log_u32_decimal(hal, fb.pixels_per_scan_line);
    hal.debug_write(b" pixel_format=");
    write_hex_u32(hal, fb.pixel_format);
    hal.debug_write(b"\r\n");
    if fb.pixel_format != GOP_PIXEL_BLUE_GREEN_RED_RESERVED_8
        && fb.pixel_format != GOP_PIXEL_RED_GREEN_BLUE_RESERVED_8
        && fb.pixel_format != GOP_PIXEL_BIT_MASK
    {
        hal.debug_write(
            b"nt10-kernel: GOP pixel_format is not RGB8(0), BGR8(1), or BitMask(2); blit may be wrong\r\n",
        );
    } else if fb.pixel_format == GOP_PIXEL_BIT_MASK {
        hal.debug_write(
            b"nt10-kernel: GOP PixelBitMask(2) - using BGR8-style byte order for software blit\r\n",
        );
    } else if fb.pixel_format == GOP_PIXEL_RED_GREEN_BLUE_RESERVED_8 {
        hal.debug_write(b"nt10-kernel: GOP pixel_format RGB8 (R-first blit path)\r\n");
    }
}

/// Write/read/write the first pixel to verify linear FB is mapped at `base` (UEFI bring-up).
pub fn uefi_framebuffer_touch_selftest<H: Hal + ?Sized>(hal: &H, fb: &FramebufferInfo) {
    if fb.base == 0 || fb.size < 4 || fb.horizontal_resolution == 0 || fb.vertical_resolution == 0 {
        hal.debug_write(b"nt10-kernel: GOP touch self-test skipped\r\n");
        return;
    }
    unsafe {
        let p = fb.base as *mut u8;
        let (or, og, ob) = fb_read_rgb8(fb, p);
        fb_write_opaque_rgb8(fb, p, 0x5a, 0xa5, 0x3c);
        let (r, g, b) = fb_read_rgb8(fb, p);
        fb_write_opaque_rgb8(fb, p, or, og, ob);
        if r == 0x5a && g == 0xa5 && b == 0x3c {
            hal.debug_write(b"nt10-kernel: GOP corner R/W self-test OK\r\n");
        } else {
            hal.debug_write(
                b"nt10-kernel: GOP corner R/W self-test FAIL (unmap or nonstandard layout)\r\n",
            );
        }
    }
}

fn log_u32_decimal<H: Hal + ?Sized>(hal: &H, mut v: u32) {
    let mut buf = [0u8; 12];
    let mut i = buf.len();
    if v == 0 {
        i -= 1;
        buf[i] = b'0';
    } else {
        while v > 0 && i > 0 {
            i -= 1;
            buf[i] = b'0' + (v % 10) as u8;
            v /= 10;
        }
    }
    hal.debug_write(&buf[i..]);
}

/// Fill a horizontal span with opaque black (`0xFF000000`) — assumes 32 bpp BGRX-style GOP (QEMU bring-up).
///
/// # Safety
/// `base` must be a valid linear mapping of the UEFI frame buffer; `stride_px` is pixels per scan line.
pub unsafe fn fill_rect_black_32bpp(
    base: *mut u32,
    stride_px: u32,
    x0: u32,
    y0: u32,
    width: u32,
    height: u32,
) {
    if base.is_null() || width == 0 || height == 0 {
        return;
    }
    let stride = stride_px as usize;
    for row in y0..y0.saturating_add(height) {
        let row_off = row as usize * stride;
        for col in x0..x0.saturating_add(width) {
            let i = row_off + col as usize;
            base.add(i).write_volatile(0xFF00_0000);
        }
    }
}

/// Horizontal color bars (BGRA 32 bpp) into a linear buffer — unit-test friendly.
pub fn draw_gradient_bars_bgra(
    buf: &mut [u8],
    width: u32,
    height: u32,
    stride_px: u32,
) -> Result<(), ()> {
    let stride = stride_px as usize;
    let need = stride
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or(())?;
    if buf.len() < need {
        return Err(());
    }
    let w = width as usize;
    for y in 0..height as usize {
        let row = y * stride * 4;
        for x in 0..w {
            let seg = (x * 4 / w.max(1)) as u8;
            let (b, g, r) = match seg {
                0 => (0xFFu8, 0x40, 0x40),
                1 => (0x40, 0xFF, 0x40),
                2 => (0x40, 0x40, 0xFF),
                _ => (0xC0, 0xC0, 0xC0),
            };
            let i = row + x * 4;
            buf[i] = b;
            buf[i + 1] = g;
            buf[i + 2] = r;
            buf[i + 3] = 0xFF;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nt10_boot_protocol::FramebufferInfo;

    #[test]
    fn handoff_rejects_null_base() {
        let fb = FramebufferInfo {
            base: 0,
            size: 1,
            horizontal_resolution: 1,
            vertical_resolution: 1,
            pixels_per_scan_line: 1,
            pixel_format: 0,
        };
        assert_eq!(parse_framebuffer_handoff(&fb), Err(FramebufferHandoffError::NullBase));
    }

    #[test]
    fn gradient_writes_expected_top_left_pixel() {
        let mut buf = [0u8; 16];
        assert!(draw_gradient_bars_bgra(&mut buf, 4, 1, 4).is_ok());
        assert_eq!(buf[0], 0xFF);
        assert_eq!(buf[1], 0x40);
    }

    #[test]
    fn linear_byte_cap_maxes_size_and_layout() {
        let fb = FramebufferInfo {
            base: 1,
            size: 1280 * 800 * 4,
            horizontal_resolution: 1280,
            vertical_resolution: 800,
            pixels_per_scan_line: 1408,
            pixel_format: GOP_PIXEL_BLUE_GREEN_RED_RESERVED_8,
        };
        let layout = 1408usize * 800 * 4;
        assert_eq!(framebuffer_linear_byte_cap(&fb), fb.size.max(layout));
    }

    #[test]
    fn fb_rgb8_bgr_round_trip() {
        let mut px = [0u8; 4];
        let fb_bgr = FramebufferInfo {
            base: 0,
            size: 4,
            horizontal_resolution: 1,
            vertical_resolution: 1,
            pixels_per_scan_line: 1,
            pixel_format: GOP_PIXEL_BLUE_GREEN_RED_RESERVED_8,
        };
        unsafe {
            fb_write_opaque_rgb8(&fb_bgr, px.as_mut_ptr(), 0x12, 0x34, 0x56);
            assert_eq!(px, [0x56, 0x34, 0x12, 0xff]);
            let (r, g, b) = fb_read_rgb8(&fb_bgr, px.as_ptr());
            assert_eq!((r, g, b), (0x12, 0x34, 0x56));
        }
        let fb_rgb = FramebufferInfo {
            pixel_format: GOP_PIXEL_RED_GREEN_BLUE_RESERVED_8,
            ..fb_bgr
        };
        unsafe {
            fb_write_opaque_rgb8(&fb_rgb, px.as_mut_ptr(), 0x12, 0x34, 0x56);
            assert_eq!(px, [0x12, 0x34, 0x56, 0xff]);
            let (r, g, b) = fb_read_rgb8(&fb_rgb, px.as_ptr());
            assert_eq!((r, g, b), (0x12, 0x34, 0x56));
        }
    }
}
