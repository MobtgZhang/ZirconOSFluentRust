//! User virtual address layout for ZirconOS NT10 (independent naming; not copied from external SDKs).
//!
//! [`crate::mm::vad::VadTable`] tracks regions within these bounds per process.

/// Lowest user VA used once null-page guards exist (1 MiB).
pub const USER_VA_BASE: u64 = 0x0000_0000_0010_0000;

/// Single 2 MiB identity-mapped window marked user/supervisor in built-in page tables (256 MiB).
/// Stays inside the first 512 MiB bring-up map; avoids colliding with typical low kernel load regions.
pub const USER_BRINGUP_VA: u64 = 0x0000_0000_1000_0000;

/// Top of stack within the same 2 MiB huge page as [`USER_BRINGUP_VA`] (full descending stack).
pub const USER_BRINGUP_STACK_TOP: u64 = USER_BRINGUP_VA + 0x200_000;

/// Upper bound for canonical 47-bit user addresses (exclusive of kernel canonical half).
pub const USER_VA_LIMIT: u64 = 0x0000_7FFF_FFFF_FFFF;

#[must_use]
pub const fn user_range_ok(start: u64, end: u64) -> bool {
    start >= USER_VA_BASE && end <= USER_VA_LIMIT && start < end
}
