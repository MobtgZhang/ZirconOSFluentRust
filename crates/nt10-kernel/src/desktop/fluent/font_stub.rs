//! Text rasterization bring-up — OFL / project fonts are wired in `build.rs`; this module holds
//! Fluent-facing **API stubs** until a ring3 font stack or in-kernel shaper lands.

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
