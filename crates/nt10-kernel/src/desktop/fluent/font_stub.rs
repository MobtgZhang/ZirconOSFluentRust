//! Text rasterization bring-up — OFL / project fonts are wired in `build.rs`; this module holds
//! Fluent-facing **API stubs** until a ring3 font stack or in-kernel shaper lands.
//!
//! Kernel Win32 `TextOut` bring-up uses bitmap glyphs in [`crate::subsystems::win32::text_bringup`]
//! (no runtime TrueType parse in `no_std` without heap).
//!
//! ## Phase 4 / 5 strategy (fontdue vs bitmap)
//!
//! 1. **Default (current)**: embed additional glyphs in `text_bringup` and/or ship **pre-rasterized**
//!    codepoints from build scripts into read-only sections (no runtime allocator).
//! 2. **Optional kernel heap**: gate `fontdue` (or similar) behind an `alloc` feature with a **small,
//!    capped** arena — only when product policy allows kernel `no_std`+alloc.
//! 3. **Ring3**: full shaping stays in user-mode `user32`/DirectWrite-class stack when csrss/ALPC
//!    path replaces kernel-drawn UEFI chrome.

/// Placeholder metrics for a Fluent text run (future: link to `fontdue` output in `build.rs`).
#[derive(Clone, Copy, Debug)]
pub struct FluentTextRunStub {
    pub px_height: u16,
}

impl FluentTextRunStub {
    #[must_use]
    pub const fn new(px_height: u16) -> Self {
        Self { px_height }
    }
}
