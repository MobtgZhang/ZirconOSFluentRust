//! Text rasterization bring-up — OFL / project fonts are wired in **`nt10-kernel/build.rs`** (e.g. Noto
//! Sans for UI Latin, Libertinus for desktop captions); this module holds Fluent-facing **API stubs**
//! until a Ring-3 font stack or in-kernel shaper lands.
//!
//! ## Current default (no global `alloc` on bare metal)
//!
//! - **Win32 `TextOut` / taskbar / captions**: bitmap glyphs and metrics from
//!   [`crate::subsystems::win32::text_bringup`] (codepoints baked at build time).
//! - **Host-only `build.rs`**: uses the `fontdue` **crate** to rasterize selected strings into
//!   `include_bytes!` blobs — **not** linked into the `no_std` kernel image.
//!
//! ## Optional paths (documented only)
//!
//! 1. **More codepoints**: extend `text_bringup` tables or add `build.rs` raster passes (still no
//!    runtime TTF parse in the kernel).
//! 2. **Kernel `fontdue`**: would require a **global allocator** + policy gate (`Cargo` feature); not
//!    enabled in this workspace today.
//! 3. **Ring-3**: full shaping in user-mode when csrss + ALPC replace kernel-drawn UEFI chrome.

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
