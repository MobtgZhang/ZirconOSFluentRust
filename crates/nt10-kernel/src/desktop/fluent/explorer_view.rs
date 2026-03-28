//! Kernel-hosted **Files** window layout (bring-up; not a ring-3 PE).
//! **Ring3**: swap for `explore.exe` + shell namespace once PE + user32 are wired (`PHASE_WIN32K_GRAPHICS`).
//! Virtual folders: [`super::known_folder`] + [`super::shell_namespace`].

use super::resources;
use super::shell;
use super::taskbar::{HitRect, TaskbarLayout};
use nt10_boot_protocol::FramebufferInfo;

/// Static row count matching `EXPLORER_ROW_LABEL_*` in `generated_icons.rs` (default listing).
pub const STATIC_ENTRY_COUNT: usize = 6;
pub const MAX_FILE_ROWS: usize = 32;
pub const ROW_H: u32 = 36;

/// Placeholder byte strings (for logic / serial); UI uses raster labels or runtime ASCII.
pub const STATIC_ENTRIES: [&[u8]; STATIC_ENTRY_COUNT] = [
    b"This PC",
    b"System volume (not mounted)",
    b"EFI\\Boot",
    b"Documents",
    b"Pictures",
    b"Network (stub)",
];

#[derive(Clone, Copy, Debug)]
pub struct ExplorerLayout {
    pub window: HitRect,
    pub title_bar: HitRect,
    pub close_btn: HitRect,
    pub client: HitRect,
}

#[must_use]
pub fn layout_for_surface(layout: &TaskbarLayout) -> ExplorerLayout {
    base_layout(layout, 0, 0)
}

/// Staggered placement for stacked windows (`depth` 0 = back, larger = toward front — see `hosted_apps`).
#[must_use]
pub fn layout_for_stack_depth(layout: &TaskbarLayout, depth: usize) -> ExplorerLayout {
    let off = (depth.min(7) as u32).saturating_mul(22);
    base_layout(layout, off, off)
}

fn base_layout(layout: &TaskbarLayout, dx: u32, dy: u32) -> ExplorerLayout {
    let w = layout.surface_w;
    let h = layout.surface_h;
    let win_w = (w * 7 / 10).max(320).min(w.saturating_sub(16));
    let win_h = (h * 65 / 100).max(220).min(layout.bar.y.saturating_sub(24));
    let x0 = (w.saturating_sub(win_w)) / 2;
    let y0 = (layout.bar.y.saturating_sub(win_h)) / 2;
    let x = x0.saturating_add(dx).min(w.saturating_sub(win_w));
    let y = y0.saturating_add(dy).min(layout.bar.y.saturating_sub(win_h));
    let window = HitRect {
        x,
        y,
        w: win_w,
        h: win_h,
    };
    let title_h = 36u32.min(win_h.saturating_sub(40));
    let title_bar = HitRect {
        x,
        y,
        w: win_w,
        h: title_h,
    };
    let close_btn = HitRect {
        x: x + win_w.saturating_sub(36),
        y: y + 4,
        w: 28,
        h: title_h.saturating_sub(8).max(20),
    };
    let client = HitRect {
        x: x + 4,
        y: y + title_h + 4,
        w: win_w.saturating_sub(8),
        h: win_h.saturating_sub(title_h + 8),
    };
    ExplorerLayout {
        window,
        title_bar,
        close_btn,
        client,
    }
}

#[must_use]
pub fn row_index_at_y(client: HitRect, py: u32, row_count: usize) -> Option<usize> {
    if py < client.y || py >= client.y + client.h {
        return None;
    }
    let rel = py - client.y;
    let idx = (rel / ROW_H) as usize;
    if idx < row_count {
        Some(idx)
    } else {
        None
    }
}

#[must_use]
pub fn row_hit(client: HitRect, px: u32, py: u32, row_count: usize) -> Option<usize> {
    if px < client.x || px >= client.x + client.w {
        return None;
    }
    row_index_at_y(client, py, row_count)
}

/// Smoke hook: child count from [`super::shell_namespace::RootShellFolder`] (Win32 `IShellFolder` path).
#[must_use]
pub fn shell_namespace_entry_count_smoke() -> usize {
    use super::shell_namespace::{RootShellFolder, ShellFolder};
    let mut buf = [&[] as &[u8]; 4];
    RootShellFolder.enumerate_children(&mut buf)
}

/// Title bar + chrome; `title_idx` → `resources::window_title_bgra`.
pub fn paint_window_chrome(
    fb: &FramebufferInfo,
    el: &ExplorerLayout,
    title_idx: usize,
    focused: bool,
) {
    let win = el.window;
    shell::fill_rect_bgra(fb, win.x, win.y, win.w, win.h, 0x28, 0x28, 0x32);
    let acc = if focused {
        (0x00u8, 0x99u8, 0xffu8)
    } else {
        (0x00u8, 0x78u8, 0xd7u8)
    };
    shell::fill_rect_bgra(fb, win.x, win.y, win.w, 2, acc.0, acc.1, acc.2);

    let tb = el.title_bar;
    shell::fill_rect_bgra(fb, tb.x, tb.y, tb.w, tb.h, 0x1a, 0x1a, 0x22);
    if let Some((title, tw, th)) = resources::window_title_bgra(title_idx) {
        let max_w = tb.w.saturating_sub(48);
        if tw <= max_w {
            let lx = tb.x + 10;
            let ly = tb.y.saturating_add(tb.h.saturating_sub(th) / 2);
            shell::blit_bgra_straight_alpha(fb, lx, ly, title, tw, th);
        }
    }

    let cb = el.close_btn;
    shell::fill_rect_bgra(fb, cb.x, cb.y, cb.w, cb.h, 0xc4, 0x2b, 0x1e);
}

/// List rows: use bitmap labels when `lens[i]==0` and `i < EXPLORER_ROW_LABEL_COUNT`, else ASCII from `rows`.
pub fn paint_files_client(
    fb: &FramebufferInfo,
    cl: HitRect,
    selected: usize,
    row_count: usize,
    rows: &[[u8; 80]; MAX_FILE_ROWS],
    lens: &[usize; MAX_FILE_ROWS],
) {
    let rc = row_count.min(MAX_FILE_ROWS);
    for i in 0..rc {
        let ry = cl.y + (i as u32) * ROW_H;
        if ry + ROW_H > cl.y + cl.h {
            break;
        }
        let sel = i == selected.min(rc.saturating_sub(1).max(0));
        let (r, g, b) = if sel {
            (0x00, 0x78, 0xd7)
        } else {
            (0x32, 0x32, 0x3c)
        };
        shell::fill_rect_bgra(fb, cl.x, ry, cl.w, ROW_H.saturating_sub(2), r, g, b);
        let icon = resources::explorer_row_icon_bgra(i);
        let iy = ry + (ROW_H.saturating_sub(resources::START_MENU_ICON_H)) / 2;
        let ix = cl.x + 6;
        shell::blit_bgra_straight_alpha(
            fb,
            ix,
            iy,
            icon,
            resources::START_MENU_ICON_W,
            resources::START_MENU_ICON_H,
        );
        let lx = ix + resources::START_MENU_ICON_W + 8;
        let max_txt_w = cl.w.saturating_sub(resources::START_MENU_ICON_W + 20);
        if lens[i] > 0 {
            shell::draw_ascii_line_clipped(
                fb,
                lx,
                ry + ROW_H / 2 - 4,
                max_txt_w,
                &rows[i][..lens[i]],
                0xf0,
                0xf0,
                0xf8,
                1,
            );
        } else if i < resources::EXPLORER_ROW_LABEL_COUNT {
            if let Some((label, lw, lh)) = resources::explorer_row_label_bgra(i) {
                if lw <= max_txt_w {
                    let ly = ry.saturating_add(ROW_H.saturating_sub(lh) / 2);
                    shell::blit_bgra_straight_alpha(fb, lx, ly, label, lw, lh);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_namespace_smoke_lists_children() {
        assert_eq!(shell_namespace_entry_count_smoke(), 2);
    }
}
