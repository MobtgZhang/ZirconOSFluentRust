//! SYSCALL/SYSRET; syscall dispatch table (original layout, public MSR names).
//!
//! User `syscall` convention is selected by [`super::syscall_abi::zr_syscall_x64_unpack_get`]:
//! - **Legacy (default)**: `%rax` = number; `%rdi`..`%r8` + `%r10` as five args (guest `%r9` not passed — see [`super::syscall_abi`]).
//! - **Nt10X64**: NT-style unpack (`r10`, `rdx`, `r8`, `r9`, stack `+0x28/+0x30`).
//!
//! Return value in `%rax` (sign-extended `i32` from [`zircon_syscall_from_user`]).

use core::arch::global_asm;

pub use super::syscall_abi::{zr_syscall_x64_unpack_get, zr_syscall_x64_unpack_set, SyscallX64Unpack};
use super::syscall_abi;
use crate::ke::spinlock::SpinLock;
use crate::rtl::log::{log_line_serial, SUB_SYSC, PREFIX};

extern "C" {
    fn zircon_syscall_entry();
}

/// Sentinel matching [`SyscallTable::dispatch`] when the slot is empty.
pub const ZR_STATUS_NOT_IMPLEMENTED: i32 = -1_073_741_822;

/// Magic syscall number for [`crate::mm::bringup_user::USER_RING3_UEFI_PROBE_SYSCALL`] (serial one-liner).
pub const ZR_UEFI_R3_PROBE_SYSCALL: u64 = 0x5ED;

/// User pointer canonicality probe: returns `0` if `a1` is a user canonical VA, else error.
pub const ZR_USER_VA_PROBE_SYSCALL: u64 = 0x401;

static ZR_SYSCALL_STATE: SpinLock<SyscallTable> = SpinLock::new(SyscallTable::empty());

pub fn zr_syscall_register(index: u16, f: SyscallFn) -> Result<(), ()> {
    ZR_SYSCALL_STATE.lock().register(index, f)
}

#[must_use]
pub fn zr_syscall_dispatch(
    index: u16,
    a1: u64,
    a2: u64,
    a3: u64,
    a4: u64,
    a5: u64,
    a6: u64,
) -> i32 {
    ZR_SYSCALL_STATE.lock().dispatch(index, a1, a2, a3, a4, a5, a6)
}

/// Invoked from [`zircon_syscall_entry`] with `num` (`rax`) and `saved` pointing at nine `u64` words:
/// guest `[rdi, rsi, rdx, r10, r8, r9, r11, rcx_return, user_rsp]`.
#[no_mangle]
extern "C" fn zircon_syscall_from_user(num: u64, saved: *const u64) -> i64 {
    if num > u64::from(u16::MAX) {
        return i64::from(ZR_STATUS_NOT_IMPLEMENTED);
    }
    let s = unsafe { core::slice::from_raw_parts(saved, 9) };
    let mut arr = [0u64; 9];
    arr.copy_from_slice(s);

    if num == ZR_UEFI_R3_PROBE_SYSCALL {
        log_line_serial(SUB_SYSC, b"ZR_UEFI_R3_SYSCALL_PROBE_OK");
        return 0;
    }
    if num == ZR_USER_VA_PROBE_SYSCALL {
        let probe_va = arr[0];
        if !syscall_abi::user_canonical_va_ok(probe_va) {
            return i64::from(ZR_STATUS_NOT_IMPLEMENTED);
        }
        log_line_serial(SUB_SYSC, b"ZR_USER_VA_PROBE_OK");
        return 0;
    }

    let (a1, a2, a3, a4, a5, a6) = match zr_syscall_x64_unpack_get() {
        SyscallX64Unpack::LegacyZircon => syscall_abi::unpack_six_legacy(&arr),
        SyscallX64Unpack::Nt10X64 => match syscall_abi::unpack_six_nt10_x64(&arr) {
            Ok(t) => t,
            Err(e) => return i64::from(e),
        },
    };

    let r = zr_syscall_dispatch(num as u16, a1, a2, a3, a4, a5, a6);
    if r == ZR_STATUS_NOT_IMPLEMENTED {
        log_line_serial(SUB_SYSC, b"user syscall smoke");
        crate::hal::x86_64::serial::write_bytes(PREFIX);
        crate::hal::x86_64::serial::write_bytes(SUB_SYSC);
        crate::hal::x86_64::serial::write_byte(b' ');
        crate::hal::x86_64::serial::write_bytes(b"syscall num ");
        crate::hal::x86_64::serial::write_hex_u64(num);
        crate::hal::x86_64::serial::write_line(b" return");
    }
    r as i64
}

global_asm!(
    r#"
    .pushsection .bss
    .p2align 12
    zr_sc_stack:
        .space 4096
    zr_sc_stack_top:
    .popsection
    .globl zircon_syscall_entry
    .align 16
    zircon_syscall_entry:
        mov r12, rax
        mov rbx, rsp
        lea rsp, [rip + zr_sc_stack_top]
        push rbx
        push rcx
        push r11
        push r9
        push r8
        push r10
        push rdx
        push rsi
        push rdi
        mov rdi, r12
        mov rsi, rsp
        sub rsp, 8
        call zircon_syscall_from_user
        add rsp, 8
        add rsp, 48
        pop r11
        pop rcx
        pop rbx
        mov rsp, rbx
        sysretq
    "#,
);

/// `IA32_EFER`
pub const MSR_IA32_EFER: u32 = 0xC000_0080;
/// `IA32_STAR`
pub const MSR_IA32_STAR: u32 = 0xC000_0081;
/// `IA32_LSTAR`
pub const MSR_IA32_LSTAR: u32 = 0xC000_0082;
/// `IA32_FMASK`
pub const MSR_IA32_FMASK: u32 = 0xC000_0084;

/// Upper bound for syscall numbers reserved in this bring-up table (Win32 bring-up uses `0x100`..).
pub const SYSCALL_TABLE_LEN: usize = 512;

/// Syscall handler: raw args from registers (extend later with trap frame).
pub type SyscallFn = fn(u64, u64, u64, u64, u64, u64) -> i32;

/// Fixed table; index is the OS syscall number (not yet wired to MSRs).
pub struct SyscallTable {
    pub entries: [Option<SyscallFn>; SYSCALL_TABLE_LEN],
}

impl SyscallTable {
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            entries: [None; SYSCALL_TABLE_LEN],
        }
    }

    #[must_use]
    pub fn new() -> Self {
        Self::empty()
    }

    pub fn register(&mut self, index: u16, f: SyscallFn) -> Result<(), ()> {
        let i = usize::from(index);
        if i >= SYSCALL_TABLE_LEN {
            return Err(());
        }
        self.entries[i] = Some(f);
        Ok(())
    }

    #[must_use]
    pub fn dispatch(
        &self,
        index: u16,
        a1: u64,
        a2: u64,
        a3: u64,
        a4: u64,
        a5: u64,
        a6: u64,
    ) -> i32 {
        let i = usize::from(index);
        if i >= SYSCALL_TABLE_LEN {
            return -1073741801; // STATUS_INVALID_PARAMETER-style sentinel
        }
        match self.entries[i] {
            Some(f) => f(a1, a2, a3, a4, a5, a6),
            None => -1073741822, // STATUS_NOT_IMPLEMENTED-style sentinel
        }
    }
}

/// Enables `EFER.SCE` so `syscall/sysret` are legal. `STAR` / `LSTAR` / `FMASK` must still be
/// programmed before user mode can invoke [`dispatch`] safely (see project syscall ABI doc).
///
/// # Safety
/// Caller must have installed a GDT that includes ring-3 segments (see [`crate::arch::x86_64::gdt`]).
#[inline]
pub unsafe fn enable_syscall_extension() {
    let efer = unsafe { crate::arch::x86_64::msr::rdmsr(MSR_IA32_EFER) };
    unsafe { crate::arch::x86_64::msr::wrmsr(MSR_IA32_EFER, efer | 1) };
}

/// Back-compat name used by older bring-up paths.
#[inline]
pub fn enable_extension_stub() {
    unsafe { enable_syscall_extension() };
}

/// Programs `STAR` / `LSTAR` / `FMASK` for 64-bit `syscall`/`sysret` with the current [`crate::arch::x86_64::gdt`]
/// layout (kernel `0x08`/`0x10`, user `0x20`/`0x18`; `STAR[47:32]=0x10` yields `SYSRET` CS=`0x20`, SS=`0x18`).
///
/// # Safety
/// Must run on BSP with valid IDT; user mode must not invoke `syscall` until this returns.
#[inline]
pub unsafe fn install_syscall_msrs_bringup() {
    let entry = zircon_syscall_entry as *const () as usize as u64;
    let star: u64 = (0x08u64 << 48) | (0x10u64 << 32);
    unsafe {
        crate::arch::x86_64::msr::wrmsr(MSR_IA32_STAR, star);
        crate::arch::x86_64::msr::wrmsr(MSR_IA32_LSTAR, entry);
        crate::arch::x86_64::msr::wrmsr(MSR_IA32_FMASK, 0x200);
    }
}
