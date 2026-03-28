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
