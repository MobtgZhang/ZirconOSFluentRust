//! Desktop Window Manager — compositor configuration for Fluent (Acrylic/Mica) bring-up
//! ([`crate::milestones::PHASE_FLUENT_DESKTOP`]).
//!
//! Aligns conceptually with DWM: ordered surfaces, per-layer dirty rects, optional per-`hwnd`
//! offscreen targets before full Win32k (`extensions/phase-06-dwm-composition.md`).
//!
//! ## Win32 DWM API ↔ Fluent (bring-up)
//! - `DwmExtendFrameIntoClientArea` → grow `CompositorSurface` client bleed + acrylic on that node.
//! - Blur behind / acrylic materials → [`super::acrylic`] radii on the same surface slot.
//! - Mica → [`super::mica`] altitude fields on `CompositorSurface`.

/// One fullscreen or window-sized surface in the compositor tree.
#[derive(Clone, Copy, Debug)]
pub struct CompositorSurface {
    pub width: u32,
    pub height: u32,
    pub z_order: i32,
    pub acrylic_blur_radius: u8,
    pub mica_altitude: u8,
    /// Opaque window handle when this surface backs a Win32 HWND (0 = unnamed / desktop root).
    pub owner_hwnd: u64,
}

impl CompositorSurface {
    #[must_use]
    pub const fn fullscreen(w: u32, h: u32) -> Self {
        Self {
            width: w,
            height: h,
            z_order: 0,
            acrylic_blur_radius: 0,
            mica_altitude: 0,
            owner_hwnd: 0,
        }
    }

    #[must_use]
    pub const fn for_hwnd(w: u32, h: u32, hwnd: u64, z: i32) -> Self {
        Self {
            width: w,
            height: h,
            z_order: z,
            acrylic_blur_radius: 0,
            mica_altitude: 0,
            owner_hwnd: hwnd,
        }
    }
}

/// Inclusive-exclusive dirty box in pixel space.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DirtyRect {
    pub x0: u32,
    pub y0: u32,
    pub x1: u32,
    pub y1: u32,
}

impl DirtyRect {
    #[must_use]
    pub fn union(a: Self, b: Self) -> Self {
        Self {
            x0: a.x0.min(b.x0),
            y0: a.y0.min(b.y0),
            x1: a.x1.max(b.x1),
            y1: a.y1.max(b.y1),
        }
    }
}

/// Root compositor state (single display path until VidPN exists).
#[derive(Clone, Copy, Debug)]
pub struct DwmCompositor {
    pub root: Option<CompositorSurface>,
    pub pending_dirty: Option<DirtyRect>,
    /// Per-window offscreen layers (milestone toward HWND-keyed composition).
    pub hwnd_layers: [Option<CompositorSurface>; 4],
    /// When set, compositor may paint FPS / dirty-outline overlay (kernel `cfg` gate in session).
    pub debug_composition_overlay: bool,
}

impl DwmCompositor {
    pub const fn new() -> Self {
        Self {
            root: None,
            pending_dirty: None,
            hwnd_layers: [None; 4],
            debug_composition_overlay: false,
        }
    }

    pub fn attach_framebuffer(&mut self, w: u32, h: u32) {
        self.root = Some(CompositorSurface::fullscreen(w, h));
    }

    /// Attach or replace the first free slot with a HWND-keyed surface.
    pub fn attach_hwnd_layer(&mut self, surf: CompositorSurface) -> Result<(), ()> {
        for slot in &mut self.hwnd_layers {
            if slot.is_none() {
                *slot = Some(surf);
                return Ok(());
            }
        }
        Err(())
    }

    pub fn mark_hwnd_dirty(&mut self, hwnd: u64, r: DirtyRect) {
        let hit = self
            .hwnd_layers
            .iter()
            .flatten()
            .any(|c| c.owner_hwnd == hwnd);
        if hit {
            self.mark_dirty(r);
        }
    }

    pub fn mark_dirty(&mut self, r: DirtyRect) {
        self.pending_dirty = Some(match self.pending_dirty {
            Some(p) => DirtyRect::union(p, r),
            None => r,
        });
    }

    /// Take merged dirty rect for this frame (single-layer compositor).
    pub fn take_merged_dirty(&mut self) -> Option<DirtyRect> {
        self.pending_dirty.take()
    }

    /// Flat single-layer fill of dirty region into a BGRA linear buffer (tests / software FB).
    pub fn commit_dirty_flat_bgra(
        &mut self,
        buf: &mut [u8],
        stride_px: u32,
        fill_bgra: [u8; 4],
    ) -> Result<(), ()> {
        let Some(root) = self.root else {
            return Ok(());
        };
        let Some(d) = self.take_merged_dirty() else {
            return Ok(());
        };
        let w = root.width;
        let _h = root.height;
        let stride = stride_px as usize;
        let x0 = d.x0.min(w);
        let x1 = d.x1.min(w).max(x0);
        let y0 = d.y0.min(root.height);
        let y1 = d.y1.min(root.height).max(y0);
        for row in y0..y1 {
            let base = row as usize * stride * 4;
            for col in x0..x1 {
                let i = base + col as usize * 4;
                if i + 4 > buf.len() {
                    return Err(());
                }
                buf[i..i + 4].copy_from_slice(&fill_bgra);
            }
        }
        Ok(())
    }

    /// One frame: fill dirty rect with `base_bgra`, then apply acrylic/mica strength from `root` fields.
    pub fn commit_dirty_acrylic_mica_light(
        &mut self,
        buf: &mut [u8],
        stride_px: u32,
        base_bgra: [u8; 4],
        theme_tint_bgra: u32,
    ) -> Result<(), ()> {
        let Some(root) = self.root else {
            return Ok(());
        };
        let Some(d) = self.take_merged_dirty() else {
            return Ok(());
        };
        self.pending_dirty = Some(d);
        self.commit_dirty_flat_bgra(buf, stride_px, base_bgra)?;
        let strength = (root.acrylic_blur_radius as u32)
            .saturating_add(root.mica_altitude as u32)
            .min(255) as u8;
        if strength == 0 {
            return Ok(());
        }
        let w = root.width;
        let stride = stride_px as usize;
        let x0 = d.x0.min(w);
        let x1 = d.x1.min(w).max(x0);
        let y0 = d.y0.min(root.height);
        let y1 = d.y1.min(root.height).max(y0);
        for row in y0..y1 {
            let base = row as usize * stride * 4;
            for col in x0..x1 {
                let i = base + col as usize * 4;
                if i + 4 > buf.len() {
                    return Err(());
                }
                let mut px = [buf[i], buf[i + 1], buf[i + 2], buf[i + 3]];
                super::acrylic::blend_bgra_pixel_under_tint(&mut px, theme_tint_bgra, strength);
                buf[i..i + 4].copy_from_slice(&px);
            }
        }
        Ok(())
    }
}

/// Win32 software compositor pass plus optional DWM dirty overlay (single scheduling point for UEFI GOP).
pub fn composite_desktop_with_dwm_overlay(
    dwm: &mut DwmCompositor,
    desktop: &crate::ob::winsta::DesktopObject,
    fb: &mut [u8],
    dst_w: u32,
    dst_h: u32,
    stride_px: u32,
    filter: crate::subsystems::win32::compositor::CompositeDesktopFilter,
) -> Result<(), ()> {
    crate::subsystems::win32::compositor::composite_desktop_to_framebuffer_filtered(
        desktop,
        fb,
        dst_w,
        dst_h,
        stride_px,
        0,
        0,
        filter,
    )?;
    if dwm.debug_composition_overlay {
        if let Some(r) = dwm.root {
            if r.acrylic_blur_radius > 0 || r.mica_altitude > 0 {
                let tint = super::acrylic::AcrylicMicaTheme::fluent_default().tint_bgra;
                let _ = dwm.commit_dirty_acrylic_mica_light(
                    fb,
                    stride_px,
                    [0x30, 0x60, 0xFF, 0x14],
                    tint,
                );
            } else {
                let _ = dwm.commit_dirty_flat_bgra(fb, stride_px, [0x30, 0x60, 0xFF, 0x14]);
            }
        } else {
            let _ = dwm.take_merged_dirty();
        }
    } else {
        let _ = dwm.take_merged_dirty();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dirty_merge_and_commit() {
        let mut c = DwmCompositor::new();
        c.attach_framebuffer(4, 4);
        c.mark_dirty(DirtyRect {
            x0: 0,
            y0: 0,
            x1: 2,
            y1: 2,
        });
        c.mark_dirty(DirtyRect {
            x0: 2,
            y0: 2,
            x1: 4,
            y1: 4,
        });
        let mut px = [0u8; 64];
        assert!(c
            .commit_dirty_flat_bgra(&mut px, 4, [9, 8, 7, 6])
            .is_ok());
        assert_eq!(px[0], 9);
    }

    #[test]
    fn acrylic_mica_light_blend_changes_pixel() {
        let mut c = DwmCompositor::new();
        let mut surf = CompositorSurface::fullscreen(4, 4);
        surf.acrylic_blur_radius = 80;
        surf.mica_altitude = 40;
        c.root = Some(surf);
        c.mark_dirty(DirtyRect {
            x0: 0,
            y0: 0,
            x1: 1,
            y1: 1,
        });
        let mut px = [0u8; 64];
        let tint = crate::desktop::fluent::acrylic::AcrylicMicaTheme::fluent_default().tint_bgra;
        assert!(c
            .commit_dirty_acrylic_mica_light(&mut px, 4, [10, 20, 30, 255], tint)
            .is_ok());
        assert_ne!(px[0], 10);
    }
}
