//! Address Space Layout Randomization — deterministic LCG for bring-up (replace with CSPRNG later).

/// 64-bit mixing step (original constants; not a cryptographic PRNG).
#[inline]
pub fn mix64(state: u64) -> u64 {
    state.wrapping_mul(0x9E37_79B9_7F4A_7C15).rotate_left(27) ^ 0xC6BC_2796_92B5_C323
}

/// Pick an image base slide within `[min_align, max_align)` page-aligned window.
#[must_use]
pub fn image_slide(seed: u64, min_pages: u64, max_pages: u64) -> u64 {
    if max_pages <= min_pages {
        return min_pages * 4096;
    }
    let span = max_pages - min_pages;
    let r = mix64(seed) % span;
    (min_pages + r) * 4096
}
