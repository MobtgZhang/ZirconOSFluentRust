//! User virtual address layout for **ZirconOSFluent / NT10** (project-local naming).
//!
//! [`crate::mm::vad::VadTable`] tracks regions within these bounds per process. Future work replaces
//! the fixed [`USER_BRINGUP_VA`] window with per-process regions once [`crate::ps::process::EProcess::cr3_phys`] is wired.

/// Lowest user VA used once null-page guards exist (1 MiB).
pub const USER_VA_BASE: u64 = 0x0000_0000_0010_0000;

/// Single 2 MiB identity-mapped window marked user/supervisor in built-in page tables (256 MiB).
/// Stays inside the first 512 MiB bring-up map; avoids colliding with typical low kernel load regions.
pub const USER_BRINGUP_VA: u64 = 0x0000_0000_1000_0000;

/// Hint VA for a larger multi-MiB user arena once per-process page tables land (documentation).
pub const USER_LARGE_ARENA_HINT: u64 = 0x0000_0000_2000_0000;

/// Top of stack within the same 2 MiB huge page as [`USER_BRINGUP_VA`] (full descending stack).
pub const USER_BRINGUP_STACK_TOP: u64 = USER_BRINGUP_VA + 0x200_000;

/// Upper bound for canonical 47-bit user addresses (exclusive of kernel canonical half).
pub const USER_VA_LIMIT: u64 = 0x0000_7FFF_FFFF_FFFF;

/// User or syscall **pointer** canonical check (lower canonical half only).
/// Matches `#PF` path in [`crate::mm::page_fault`] and [`crate::arch::x86_64::syscall_abi::user_canonical_va_ok`].
#[must_use]
pub const fn user_pointer_canonical(va: u64) -> bool {
    va < 0x0000_8000_0000_0000
}

#[must_use]
pub const fn user_range_ok(start: u64, end: u64) -> bool {
    start >= USER_VA_BASE && end <= USER_VA_LIMIT && start < end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_pointer_canonical_matches_kernel_half_split() {
        assert!(user_pointer_canonical(0));
        assert!(user_pointer_canonical(0x7FFF_FFFF_FFFF));
        assert!(!user_pointer_canonical(0x8000_0000_0000));
    }
}
