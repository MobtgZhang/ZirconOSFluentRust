//! Huge / large pages (2 MiB / 1 GiB) — **not** used for default user mappings yet.
//!
//! Bring-up uses only 4 KiB mappings via [`super::pt::map_4k`]. When promoting, coordinate PAT/NX
//! with Intel SDM; do not copy OS-private PAT programming from retail binaries.
//!
//! See [MM-Goals-and-Invariants.md](../../../../docs/en/MM-Goals-and-Invariants.md).

/// Policy for future 2 MiB / 1 GiB mappings (clean-room placeholder).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LargePagePolicy {
    /// Only 4 KiB mappings are legal today.
    FourKiBOnly,
    /// Reserved: allow 2 MiB where the PFN allocator supplies aligned runs.
    Allow2MiBWhenAligned,
}

#[must_use]
pub const fn large_page_policy_bringup() -> LargePagePolicy {
    LargePagePolicy::FourKiBOnly
}

/// `true` when the kernel may attempt a 2 MiB mapping (always `false` in bring-up).
#[must_use]
pub fn large_page_kernel_promotion_enabled() -> bool {
    false
}
