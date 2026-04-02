//! Win10-inspired full-screen GOP UI (ZirconOS branding only — no Microsoft assets).

use r_efi::efi::protocols::graphics_output;

use crate::boot_font;

/// #0078D7 style background (BGR).
pub const COL_BG: graphics_output::BltPixel = graphics_output::BltPixel {
    blue: 215,
    green: 120,
    red: 0,
    reserved: 0,
};
pub const COL_TILE_BG: graphics_output::BltPixel = graphics_output::BltPixel {
    blue: 180,
    green: 90,
    red: 0,
    reserved: 0,
};
pub const COL_ICON_AREA: graphics_output::BltPixel = graphics_output::BltPixel {
    blue: 235,
    green: 150,
    red: 40,
    reserved: 0,
};
pub const COL_WHITE: graphics_output::BltPixel = graphics_output::BltPixel {
    blue: 255,
    green: 255,
    red: 255,
    reserved: 0,
};
pub const COL_MUTED: graphics_output::BltPixel = graphics_output::BltPixel {
    blue: 200,
    green: 200,
    red: 200,
    reserved: 0,
};

pub const TEXT_SCALE: usize = 2;
pub const ENTRY_COUNT: usize = 5;

#[derive(Clone, Copy)]
pub struct Rect {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

impl Rect {
    pub fn contains(&self, px: i32, py: i32) -> bool {
        px >= self.x as i32
            && py >= self.y as i32
            && px < (self.x + self.w) as i32
            && py < (self.y + self.h) as i32
    }
}

pub struct TileLayout {
    pub tiles: [Rect; ENTRY_COUNT],
    pub footer: Rect,
    pub title_x: usize,
    pub title_y: usize,
    pub auto_text_x: usize,
    pub auto_text_y: usize,
}

pub unsafe fn fill_screen(gop: *mut graphics_output::Protocol, px: graphics_output::BltPixel) {
    if gop.is_null() {
        return;
    }
    let mode_ptr = (*gop).mode;
    if mode_ptr.is_null() {
        return;
    }
    let h = (*mode_ptr).info;
    if h.is_null() {
        return;
    }
    let w = (*h).horizontal_resolution as usize;
    let vh = (*h).vertical_resolution as usize;
    let mut p = px;
    let _ = ((*gop).blt)(
        gop,
        &mut p,
        graphics_output::BLT_VIDEO_FILL,
        0,
        0,
        0,
        0,
        w,
        vh,
        0,
    );
}

unsafe fn blt_pixel(
    gop: *mut graphics_output::Protocol,
    x: usize,
    y: usize,
    px: graphics_output::BltPixel,
) {
    if gop.is_null() {
        return;
    }
    let mut p = px;
    let _ = ((*gop).blt)(
        gop,
        &mut p,
        graphics_output::BLT_VIDEO_FILL,
        0,
        0,
        x,
        y,
        1,
        1,
        0,
    );
}

unsafe fn fill_rect_px(
    gop: *mut graphics_output::Protocol,
    r: Rect,
    px: graphics_output::BltPixel,
) {
    if r.w == 0 || r.h == 0 {
        return;
    }
    let mut p = px;
    let _ = ((*gop).blt)(
        gop,
        &mut p,
        graphics_output::BLT_VIDEO_FILL,
        0,
        0,
        r.x,
        r.y,
        r.w,
        r.h,
        0,
    );
}

unsafe fn stroke_rect(gop: *mut graphics_output::Protocol, r: Rect, px: graphics_output::BltPixel) {
    let t = 2usize;
    if r.w <= t * 2 || r.h <= t * 2 {
        return;
    }
    fill_rect_px(
        gop,
        Rect {
            x: r.x,
            y: r.y,
            w: r.w,
            h: t,
        },
        px,
    );
    fill_rect_px(
        gop,
        Rect {
            x: r.x,
            y: r.y + r.h - t,
            w: r.w,
            h: t,
        },
        px,
    );
    fill_rect_px(
        gop,
        Rect {
            x: r.x,
            y: r.y,
            w: t,
            h: r.h,
        },
        px,
    );
    fill_rect_px(
        gop,
        Rect {
            x: r.x + r.w - t,
            y: r.y,
            w: t,
            h: r.h,
        },
        px,
    );
}

unsafe fn draw_glyph_scaled(
    gop: *mut graphics_output::Protocol,
    x: usize,
    y: usize,
    c: u8,
    scale: usize,
    fg: graphics_output::BltPixel,
) {
    let rows = boot_font::rows(c);
    for row in 0..boot_font::GLYPH_H {
        let bits = rows[row];
        let mut col = 0usize;
        while col < boot_font::GLYPH_W {
            if (bits << col) & 0x80 == 0 {
                col += 1;
                continue;
            }
            let start = col;
            while col < boot_font::GLYPH_W && (bits << col) & 0x80 != 0 {
                col += 1;
            }
            let run = col - start;
            fill_rect_px(
                gop,
                Rect {
                    x: x + start * scale,
                    y: y + row * scale,
                    w: run * scale,
                    h: scale,
                },
                fg,
            );
        }
    }
}

pub unsafe fn draw_ascii(
    gop: *mut graphics_output::Protocol,
    x: usize,
    y: usize,
    s: &[u8],
    scale: usize,
    fg: graphics_output::BltPixel,
) {
    let mut cx = x;
    for &b in s {
        if b == b'\n' {
            continue;
        }
        draw_glyph_scaled(gop, cx, y, b, scale, fg);
        cx += (boot_font::GLYPH_W * scale) + scale;
    }
}

pub unsafe fn layout(fw: usize, fh: usize) -> TileLayout {
    let mx = (fw / 24).max(32);
    let gap = 16usize;
    let cols = 2usize;
    let rows = (ENTRY_COUNT + cols - 1) / cols;
    let tile_h = (fh / 8).clamp(56, 96);
    let tile_w = (fw.saturating_sub(mx * 2).saturating_sub(gap * (cols - 1))) / cols;
    let title_y = (fh / 14).max(40);
    let title_x = mx;
    let grid_top = title_y + 48;
    let mut tiles = [Rect { x: 0, y: 0, w: 0, h: 0 }; ENTRY_COUNT];
    for i in 0..ENTRY_COUNT {
        let row = i / cols;
        let col = i % cols;
        tiles[i] = Rect {
            x: mx + col * (tile_w + gap),
            y: grid_top + row * (tile_h + gap),
            w: tile_w,
            h: tile_h,
        };
    }
    let footer_h = 40usize;
    let footer = Rect {
        x: mx,
        y: fh.saturating_sub(footer_h + mx),
        w: fw.saturating_sub(mx * 2),
        h: footer_h,
    };
    let auto_y = grid_top + rows * (tile_h + gap) + 8;
    TileLayout {
        tiles,
        footer,
        title_x,
        title_y,
        auto_text_x: mx,
        auto_text_y: auto_y.min(fh.saturating_sub(80)),
    }
}

unsafe fn draw_icon_zircon(gop: *mut graphics_output::Protocol, r: Rect) {
    let pad = 8usize;
    if r.w < pad * 4 || r.h < pad * 4 {
        return;
    }
    let ix = r.x + pad;
    let iy = r.y + pad;
    let iw = r.w - pad * 2;
    let ih = r.h - pad * 2;
    fill_rect_px(
        gop,
        Rect {
            x: ix,
            y: iy,
            w: iw,
            h: ih,
        },
        COL_ICON_AREA,
    );
    let cx = ix + iw / 2;
    let cy = iy + ih / 2;
    let s = (iw.min(ih) / 4).max(4);
    for i in 0..s {
        blt_pixel(gop, cx - s + i, cy - s + i * 2, COL_WHITE);
    }
    for i in 0..s {
        blt_pixel(gop, cx + i, cy - s + i * 2, COL_WHITE);
    }
}

unsafe fn draw_icon_reserved(gop: *mut graphics_output::Protocol, r: Rect) {
    let pad = 8usize;
    let ix = r.x + pad;
    let iy = r.y + pad;
    let iw = r.w - pad * 2;
    let ih = r.h - pad * 2;
    fill_rect_px(
        gop,
        Rect {
            x: ix,
            y: iy,
            w: iw,
            h: ih,
        },
        COL_ICON_AREA,
    );
    stroke_rect(
        gop,
        Rect {
            x: ix + iw / 4,
            y: iy + ih / 4,
            w: iw / 2,
            h: ih / 2,
        },
        COL_WHITE,
    );
}

unsafe fn draw_icon_chain(gop: *mut graphics_output::Protocol, r: Rect) {
    let pad = 8usize;
    let ix = r.x + pad;
    let iy = r.y + pad;
    let iw = r.w - pad * 2;
    let ih = r.h - pad * 2;
    fill_rect_px(
        gop,
        Rect {
            x: ix,
            y: iy,
            w: iw,
            h: ih,
        },
        COL_ICON_AREA,
    );
    let c = 4usize;
    fill_rect_px(
        gop,
        Rect {
            x: ix + iw / 4,
            y: iy + ih / 2 - c,
            w: c * 2,
            h: c * 2,
        },
        COL_WHITE,
    );
    fill_rect_px(
        gop,
        Rect {
            x: ix + iw / 2,
            y: iy + ih / 2 - c,
            w: c * 2,
            h: c * 2,
        },
        COL_WHITE,
    );
}

unsafe fn draw_icon_reboot(gop: *mut graphics_output::Protocol, r: Rect) {
    let pad = 8usize;
    let ix = r.x + pad;
    let iy = r.y + pad;
    let iw = r.w - pad * 2;
    let ih = r.h - pad * 2;
    fill_rect_px(
        gop,
        Rect {
            x: ix,
            y: iy,
            w: iw,
            h: ih,
        },
        COL_ICON_AREA,
    );
    let cx = ix + iw / 2;
    let cy = iy + ih / 2;
    let pts: [(isize, isize); 16] = [
        (12, 2),
        (14, 2),
        (16, 3),
        (17, 5),
        (17, 8),
        (15, 11),
        (12, 12),
        (9, 11),
        (7, 8),
        (7, 5),
        (8, 3),
        (10, 2),
        (14, 6),
        (11, 6),
        (8, 8),
        (15, 8),
    ];
    for &(px, py) in &pts {
        let x = (cx as isize + px - 12).max(0) as usize;
        let y = (cy as isize + py - 7).max(0) as usize;
        fill_rect_px(gop, Rect { x, y, w: 2, h: 2 }, COL_WHITE);
    }
}

unsafe fn draw_icon_power(gop: *mut graphics_output::Protocol, r: Rect) {
    let pad = 8usize;
    let ix = r.x + pad;
    let iy = r.y + pad;
    let iw = r.w - pad * 2;
    let ih = r.h - pad * 2;
    fill_rect_px(
        gop,
        Rect {
            x: ix,
            y: iy,
            w: iw,
            h: ih,
        },
        COL_ICON_AREA,
    );
    let cx = ix + iw / 2;
    fill_rect_px(
        gop,
        Rect {
            x: cx.saturating_sub(2),
            y: iy + ih / 4,
            w: 4,
            h: ih / 3,
        },
        COL_WHITE,
    );
    for dy in 0..ih / 3 {
        let w = 8usize.saturating_sub(dy / 2);
        fill_rect_px(
            gop,
            Rect {
                x: cx.saturating_sub(w / 2),
                y: iy + ih / 2 + dy,
                w,
                h: 1,
            },
            COL_WHITE,
        );
    }
}

unsafe fn draw_entry_icon(gop: *mut graphics_output::Protocol, tile: Rect, index: usize) {
    let icon_w = (tile.h - 16).min(88);
    let icon_box = Rect {
        x: tile.x + 12,
        y: tile.y + 8,
        w: icon_w,
        h: tile.h - 16,
    };
    match index {
        0 => draw_icon_zircon(gop, icon_box),
        1 => draw_icon_reserved(gop, icon_box),
        2 => draw_icon_chain(gop, icon_box),
        3 => draw_icon_reboot(gop, icon_box),
        _ => draw_icon_power(gop, icon_box),
    }
}

pub unsafe fn draw_tile_entry(
    gop: *mut graphics_output::Protocol,
    tile: Rect,
    index: usize,
    label: &[u8],
    selected: bool,
    muted: bool,
) {
    fill_rect_px(gop, tile, COL_TILE_BG);
    draw_entry_icon(gop, tile, index);
    if selected {
        stroke_rect(gop, tile, COL_WHITE);
    }
    let icon_w = (tile.h - 16).min(88);
    let text_x = tile.x + 12 + icon_w + 16;
    let text_y = tile.y + (tile.h / 2).saturating_sub((boot_font::GLYPH_H * TEXT_SCALE) / 2);
    let fg = if muted { COL_MUTED } else { COL_WHITE };
    draw_ascii(gop, text_x, text_y, label, TEXT_SCALE, fg);
}

pub unsafe fn draw_footer_link(gop: *mut graphics_output::Protocol, footer: Rect, selected: bool) {
    let fg = if selected { COL_WHITE } else { COL_MUTED };
    draw_ascii(
        gop,
        footer.x,
        footer.y + 8,
        b"Change defaults (or press C)",
        TEXT_SCALE.saturating_sub(1).max(1),
        fg,
    );
}

pub unsafe fn draw_cursor(gop: *mut graphics_output::Protocol, cx: i32, cy: i32) {
    let x = cx.max(0) as usize;
    let y = cy.max(0) as usize;
    for i in 0..14usize {
        blt_pixel(gop, x + i, y + i, COL_WHITE);
    }
    for i in 0..8usize {
        blt_pixel(gop, x + i, y, COL_WHITE);
        blt_pixel(gop, x, y + i, COL_WHITE);
    }
}

/// Saved region side (covers `draw_cursor` tip + arms with margin).
pub const CURSOR_PATCH: usize = 24;

const ZERO_PX: graphics_output::BltPixel = graphics_output::BltPixel {
    blue: 0,
    green: 0,
    red: 0,
    reserved: 0,
};

unsafe fn blt_video_to_buffer(
    gop: *mut graphics_output::Protocol,
    src_x: usize,
    src_y: usize,
    w: usize,
    h: usize,
    dst: *mut graphics_output::BltPixel,
) {
    if gop.is_null() || w == 0 || h == 0 || dst.is_null() {
        return;
    }
    let _ = ((*gop).blt)(
        gop,
        dst,
        graphics_output::BLT_VIDEO_TO_BLT_BUFFER,
        src_x,
        src_y,
        0,
        0,
        w,
        h,
        0,
    );
}

unsafe fn blt_buffer_to_video(
    gop: *mut graphics_output::Protocol,
    dst_x: usize,
    dst_y: usize,
    w: usize,
    h: usize,
    src: *mut graphics_output::BltPixel,
) {
    if gop.is_null() || w == 0 || h == 0 || src.is_null() {
        return;
    }
    let _ = ((*gop).blt)(
        gop,
        src,
        graphics_output::BLT_BUFFER_TO_VIDEO,
        0,
        0,
        dst_x,
        dst_y,
        w,
        h,
        0,
    );
}

/// Software cursor: save/restore framebuffer under a patch to avoid full-screen redraw each poll.
pub struct CursorOverlay {
    saved: [graphics_output::BltPixel; CURSOR_PATCH * CURSOR_PATCH],
    sx: usize,
    sy: usize,
    pw: usize,
    ph: usize,
    active: bool,
    last_tip_x: i32,
    last_tip_y: i32,
}

impl CursorOverlay {
    pub const fn new() -> Self {
        Self {
            saved: [ZERO_PX; CURSOR_PATCH * CURSOR_PATCH],
            sx: 0,
            sy: 0,
            pw: 0,
            ph: 0,
            active: false,
            last_tip_x: i32::MIN,
            last_tip_y: i32::MIN,
        }
    }

    pub unsafe fn restore(&mut self, gop: *mut graphics_output::Protocol) {
        if !self.active || self.pw == 0 || self.ph == 0 {
            self.active = false;
            return;
        }
        blt_buffer_to_video(gop, self.sx, self.sy, self.pw, self.ph, self.saved.as_mut_ptr());
        self.active = false;
    }

    /// Call after a full menu paint (framebuffer was completely redrawn).
    pub unsafe fn place_after_full_paint(
        &mut self,
        gop: *mut graphics_output::Protocol,
        tip_x: i32,
        tip_y: i32,
        fw: usize,
        fh: usize,
    ) {
        self.active = false;
        self.place(gop, tip_x, tip_y, fw, fh);
    }

    /// Move cursor: restore old patch, save under new tip, draw cursor.
    pub unsafe fn update(
        &mut self,
        gop: *mut graphics_output::Protocol,
        tip_x: i32,
        tip_y: i32,
        fw: usize,
        fh: usize,
    ) {
        if self.active && tip_x == self.last_tip_x && tip_y == self.last_tip_y {
            return;
        }
        self.restore(gop);
        self.place(gop, tip_x, tip_y, fw, fh);
    }

    unsafe fn place(
        &mut self,
        gop: *mut graphics_output::Protocol,
        tip_x: i32,
        tip_y: i32,
        fw: usize,
        fh: usize,
    ) {
        if gop.is_null() || fw < 16 || fh < 16 {
            return;
        }
        let patch = CURSOR_PATCH;
        let max_x0 = fw.saturating_sub(patch);
        let max_y0 = fh.saturating_sub(patch);
        let x0 = ((tip_x as isize).saturating_sub(2))
            .clamp(0, max_x0 as isize) as usize;
        let y0 = ((tip_y as isize).saturating_sub(2))
            .clamp(0, max_y0 as isize) as usize;
        let pw = patch.min(fw.saturating_sub(x0));
        let ph = patch.min(fh.saturating_sub(y0));
        if pw == 0 || ph == 0 {
            return;
        }
        blt_video_to_buffer(gop, x0, y0, pw, ph, self.saved.as_mut_ptr());
        draw_cursor(gop, tip_x, tip_y);
        self.sx = x0;
        self.sy = y0;
        self.pw = pw;
        self.ph = ph;
        self.active = true;
        self.last_tip_x = tip_x;
        self.last_tip_y = tip_y;
    }
}

/// Full menu paint (clears screen). Does not draw the pointer overlay.
pub unsafe fn paint_main_menu_content(
    gop: *mut graphics_output::Protocol,
    layout: &TileLayout,
    focused_entry: Option<usize>,
    footer_selected: bool,
    chainload_tile_muted: bool,
    auto_left: u64,
    auto_countdown_visible: bool,
    labels: [&'static [u8]; ENTRY_COUNT],
) {
    fill_screen(gop, COL_BG);
    draw_title(gop, layout.title_x, layout.title_y);
    for i in 0..ENTRY_COUNT {
        let sel = matches!(focused_entry, Some(j) if j == i);
        let muted = i == 2 && chainload_tile_muted;
        draw_tile_entry(
            gop,
            layout.tiles[i],
            i,
            labels[i],
            sel,
            muted,
        );
    }
    draw_footer_link(gop, layout.footer, footer_selected);
    draw_auto_text(
        gop,
        layout.auto_text_x,
        layout.auto_text_y,
        auto_left,
        auto_countdown_visible,
    );
}

pub unsafe fn draw_title(gop: *mut graphics_output::Protocol, x: usize, y: usize) {
    draw_ascii(
        gop,
        x,
        y,
        b"Choose a startup option",
        TEXT_SCALE + 1,
        COL_WHITE,
    );
}

pub unsafe fn draw_auto_text(
    gop: *mut graphics_output::Protocol,
    x: usize,
    y: usize,
    secs: u64,
    enabled: bool,
) {
    if !enabled {
        draw_ascii(
            gop,
            x,
            y,
            b"Auto-boot: off",
            TEXT_SCALE,
            COL_MUTED,
        );
        return;
    }
    let mut buf = [0u8; 48];
    let prefix = b"Auto-boot in ";
    let mut n = 0usize;
    for &b in prefix {
        buf[n] = b;
        n += 1;
    }
    let s = secs.min(99);
    if s >= 10 {
        buf[n] = b'0' + (s / 10) as u8;
        n += 1;
    }
    buf[n] = b'0' + (s % 10) as u8;
    n += 1;
    for &b in b" s (any key cancels)" {
        buf[n] = b;
        n += 1;
    }
    draw_ascii(gop, x, y, &buf[..n], TEXT_SCALE, COL_WHITE);
}
