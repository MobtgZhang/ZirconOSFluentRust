//! Taskbar / Start — layout, clock zone, Start menu flyout hit rects.

use super::resources;

/// Re-export from `generated_icons.rs` (must match `START_MENU_ICON_IDS.len()` in `build.rs`).
pub use resources::START_MENU_ROW_COUNT;

/// Pinned taskbar buttons (left → right = front-most windows first).
pub const TASK_SLOT_COUNT: usize = 5;

/// Placeholder notification-area icons (network / volume / input) per
/// `references/win32/desktop-src/shell/notification-area.md`.
pub const TRAY_ICON_COUNT: usize = 3;

/// Axis-aligned hit region in surface space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HitRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

impl HitRect {
    #[must_use]
    pub const fn contains(self, px: u32, py: u32) -> bool {
        px >= self.x
            && py >= self.y
            && px < self.x.saturating_add(self.w)
            && py < self.y.saturating_add(self.h)
    }
}

/// Start menu panel + row hit boxes.
#[derive(Clone, Copy, Debug)]
pub struct StartMenuLayout {
    pub panel: HitRect,
    pub items: [HitRect; START_MENU_ROW_COUNT],
}

/// Minimal shell chrome for bring-up dumps.
#[derive(Clone, Copy, Debug)]
pub struct TaskbarLayout {
    pub bar: HitRect,
    pub start_button: HitRect,
    pub surface_w: u32,
    pub surface_h: u32,
}

impl TaskbarLayout {
    #[must_use]
    pub fn for_surface(w: u32, h: u32) -> Self {
        let bar_h = 48u32.min(h);
        Self {
            bar: HitRect {
                x: 0,
                y: h.saturating_sub(bar_h),
                w,
                h: bar_h,
            },
            start_button: HitRect {
                x: 4,
                y: h.saturating_sub(bar_h) + 4,
                w: 44,
                h: bar_h.saturating_sub(8),
            },
            surface_w: w,
            surface_h: h,
        }
    }

    /// Right-side notification cluster (tray + clock text), inside the taskbar.
    /// See `references/win32/desktop-src/shell/taskbar.md` (notification area, clock).
    #[must_use]
    pub fn clock_area(self) -> HitRect {
        let cw = 128u32.min(self.surface_w.saturating_sub(64));
        HitRect {
            x: self.surface_w.saturating_sub(cw + 8),
            y: self.bar.y + 6,
            w: cw,
            h: self.bar.h.saturating_sub(12),
        }
    }

    /// Left part of [`Self::clock_area`] reserved for tray-style status icons.
    #[must_use]
    pub fn tray_area(self) -> HitRect {
        let c = self.clock_area();
        let tray_w = 52u32.min(c.w);
        HitRect {
            x: c.x,
            y: c.y,
            w: tray_w,
            h: c.h,
        }
    }

    /// Time + date text region (right side of the notification cluster).
    #[must_use]
    pub fn clock_display_area(self) -> HitRect {
        let c = self.clock_area();
        let tray_w = 52u32.min(c.w);
        HitRect {
            x: c.x + tray_w,
            y: c.y,
            w: c.w.saturating_sub(tray_w),
            h: c.h,
        }
    }

    /// Hit rect for tray icon `i` (`0..TRAY_ICON_COUNT`).
    #[must_use]
    pub fn tray_icon_rect(self, i: usize) -> Option<HitRect> {
        if i >= TRAY_ICON_COUNT {
            return None;
        }
        let t = self.tray_area();
        let slot = 16u32;
        let gap = 3u32;
        let n = TRAY_ICON_COUNT as u32;
        let total = n.saturating_mul(slot).saturating_add((n.saturating_sub(1)).saturating_mul(gap));
        let x0 = t.x + t.w.saturating_sub(total) / 2;
        let y = t.y + t.h.saturating_sub(slot) / 2;
        Some(HitRect {
            x: x0.saturating_add(i as u32 * (slot + gap)),
            y,
            w: slot,
            h: slot,
        })
    }

    /// Thin “show desktop” strip at the far right (`references/win32/desktop-src/uxguide/winenv-taskbar.md`).
    #[must_use]
    pub fn show_desktop_corner(self) -> HitRect {
        HitRect {
            x: self.surface_w.saturating_sub(5),
            y: self.bar.y,
            w: 5,
            h: self.bar.h,
        }
    }

    /// Task buttons between Start and clock.
    #[must_use]
    pub fn task_slots(self) -> [HitRect; TASK_SLOT_COUNT] {
        let ck = self.clock_area();
        let x0 = self.start_button.x + self.start_button.w + 10;
        let right = ck.x.saturating_sub(12);
        let avail = right.saturating_sub(x0);
        let gap = 4u32;
        let denom = TASK_SLOT_COUNT as u32;
        let slot_w = (avail.saturating_sub(gap.saturating_mul(denom.saturating_sub(1))) / denom)
            .max(36)
            .min(56);
        let mut out = [HitRect {
            x: 0,
            y: 0,
            w: 0,
            h: 0,
        }; TASK_SLOT_COUNT];
        let mut x = x0;
        let y = self.bar.y + 6;
        let h = self.bar.h.saturating_sub(12);
        for i in 0..TASK_SLOT_COUNT {
            out[i] = HitRect {
                x,
                y,
                w: slot_w,
                h,
            };
            x = x.saturating_add(slot_w).saturating_add(gap);
        }
        out
    }

    /// Flyout above the taskbar, left aligned with Start.
    #[must_use]
    pub fn start_menu(self) -> StartMenuLayout {
        let menu_w = 300u32.min(self.surface_w.saturating_sub(8));
        let row_min = 28u32;
        let menu_h = (20 + START_MENU_ROW_COUNT as u32 * row_min)
            .min(self.bar.y.saturating_sub(8))
            .max(160);
        let x = 4u32;
        let y = self.bar.y.saturating_sub(menu_h).saturating_sub(4);
        let panel = HitRect {
            x,
            y,
            w: menu_w,
            h: menu_h,
        };
        let inner_h = menu_h.saturating_sub(20);
        let row = inner_h / START_MENU_ROW_COUNT as u32;
        let mut items = [HitRect { x: 0, y: 0, w: 0, h: 0 }; START_MENU_ROW_COUNT];
        for i in 0..START_MENU_ROW_COUNT {
            items[i] = HitRect {
                x: x + 10,
                y: y + 10 + (i as u32) * row,
                w: menu_w.saturating_sub(20),
                h: row.saturating_sub(4).max(1),
            };
        }
        StartMenuLayout { panel, items }
    }
}
