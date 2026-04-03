//! x86_64 syscall **guest register** layout at the kernel entry (`IA32_LSTAR`).
//!
//! ZirconOS supports two unpack modes:
//! - **LegacyZircon**: args in guest `rdi, rsi, rdx, r10, r8` (6th guest `r9` ignored; matches early bring-up).
//! - **Nt10X64**: matches the de-facto Windows NT x64 convention: guest `r10, rdx, r8, r9` plus 5th/6th on the
//!   user stack at offsets `0x28` and `0x30` from the user `RSP` at the `syscall` instruction (32-byte home
//!   shadow below those slots — described in public x64 calling-convention material; verify with self-tests).
//!
//! Public syscall **indices** for Windows 10 class systems are tabulated in third-party datasets (e.g. j00ru
//! `windows-syscalls` on GitHub); we cite them in [`super::nt_syscall_indices`] without embedding their full JSON.

use core::sync::atomic::{AtomicU8, Ordering};

/// `0` = [`SyscallX64Unpack::LegacyZircon`], `1` = [`SyscallX64Unpack::Nt10X64`].
static SYSCALL_X64_UNPACK: AtomicU8 = AtomicU8::new(0);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum SyscallX64Unpack {
    LegacyZircon = 0,
    Nt10X64 = 1,
}

impl SyscallX64Unpack {
    #[must_use]
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::LegacyZircon),
            1 => Some(Self::Nt10X64),
            _ => None,
        }
    }
}

/// Select how [`super::syscall::zircon_syscall_from_user`] maps saved guest GPRs to `(a1..a6)`.
pub fn zr_syscall_x64_unpack_set(mode: SyscallX64Unpack) {
    SYSCALL_X64_UNPACK.store(mode as u8, Ordering::Release);
}

#[must_use]
pub fn zr_syscall_x64_unpack_get() -> SyscallX64Unpack {
    SyscallX64Unpack::from_u8(SYSCALL_X64_UNPACK.load(Ordering::Acquire))
        .unwrap_or(SyscallX64Unpack::LegacyZircon)
}

#[must_use]
pub const fn user_canonical_va_ok(va: u64) -> bool {
    va < 0x0000_8000_0000_0000
}

/// Saved block layout from `zircon_syscall_entry` (low indices first on stack):
/// `[rdi, rsi, rdx, r10, r8, r9, r11, rcx_return, user_rsp]`.
#[must_use]
pub fn unpack_six_legacy(saved: &[u64; 9]) -> (u64, u64, u64, u64, u64, u64) {
    (
        saved[0], saved[1], saved[2], saved[3], saved[4], 0,
    )
}

/// NT-style unpack; `user_rsp` is `saved[8]`.
#[must_use]
pub fn unpack_six_nt10_x64(saved: &[u64; 9]) -> Result<(u64, u64, u64, u64, u64, u64), i32> {
    let user_rsp = saved[8];
    if !user_canonical_va_ok(user_rsp) {
        return Err(0xC000_000D_u32 as i32); // STATUS_INVALID_PARAMETER
    }
    let base = user_rsp as *const u8;
    let a1 = saved[3];
    let a2 = saved[2];
    let a3 = saved[4];
    let a4 = saved[5];
    let a5 = unsafe { core::ptr::read_unaligned(base.add(0x28) as *const u64) };
    let a6 = unsafe { core::ptr::read_unaligned(base.add(0x30) as *const u64) };
    Ok((a1, a2, a3, a4, a5, a6))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_ignores_r9_for_sixth_slot() {
        let s = [1, 2, 3, 4, 5, 99, 0, 0, 0];
        let (a1, a2, a3, a4, a5, a6) = unpack_six_legacy(&s);
        assert_eq!((a1, a2, a3, a4, a5, a6), (1, 2, 3, 4, 5, 0));
    }

    #[test]
    fn nt10_reads_stack_slots() {
        let mut stack = [0u8; 0x40];
        let w5 = 0x1111u64;
        let w6 = 0x2222u64;
        unsafe {
            core::ptr::write_unaligned(stack.as_mut_ptr().add(0x28) as *mut u64, w5);
            core::ptr::write_unaligned(stack.as_mut_ptr().add(0x30) as *mut u64, w6);
        }
        let rsp = stack.as_ptr() as u64;
        let s = [0u64, 0, 2, 10, 3, 4, 0, 0, rsp];
        let u = unpack_six_nt10_x64(&s).unwrap();
        assert_eq!(u, (10, 2, 3, 4, 0x1111, 0x2222));
    }
}
