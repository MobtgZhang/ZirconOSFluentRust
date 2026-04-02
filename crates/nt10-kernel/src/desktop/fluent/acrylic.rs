//! Acrylic / Mica — theme parameters (no pixel-accurate Win11 clone).

/// Tunable Fluent surface parameters stored for compositor / logging only.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AcrylicMicaTheme {
    pub acrylic_blur_radius_px: u8,
    pub mica_altitude: u8,
    /// Premultiplied-style BGRA tint for diagnostics.
    pub tint_bgra: u32,
}

impl Default for AcrylicMicaTheme {
    fn default() -> Self {
        Self {
            acrylic_blur_radius_px: 0,
            mica_altitude: 0,
            tint_bgra: 0xFF_F0_F0_F0,
        }
    }
}

impl AcrylicMicaTheme {
    #[must_use]
    pub const fn fluent_default() -> Self {
        Self {
            acrylic_blur_radius_px: 12,
            mica_altitude: 64,
            tint_bgra: 0xFF_E8_E8_E8,
        }
    }
}

/// Lightweight `dst = lerp(dst, tint, strength)` per channel (bring-up blend, not real blur).
pub fn blend_bgra_pixel_under_tint(dst: &mut [u8; 4], tint_bgra: u32, strength: u8) {
    if strength == 0 {
        return;
    }
    let t = strength as u32;
    let tb = (tint_bgra & 0xFF) as u32;
    let tg = ((tint_bgra >> 8) & 0xFF) as u32;
    let tr = ((tint_bgra >> 16) & 0xFF) as u32;
    let ta = ((tint_bgra >> 24) & 0xFF) as u32;
    let s = [
        tb,
        tg,
        tr,
        (ta * t / 255).min(255),
    ];
    for i in 0..4 {
        let d = dst[i] as u32;
        let k = s[i];
        dst[i] = ((d * (255 - t) + k * t) / 255).min(255) as u8;
    }
}
