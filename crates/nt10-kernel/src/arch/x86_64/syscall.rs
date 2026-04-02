//! SYSCALL/SYSRET; syscall dispatch table (original layout, public MSR names).
//!
//! User `syscall` convention (bring-up): `%rax` = number; `%rdi`,`%rsi`,`%rdx`,`%r10`,`%r8`,`%r9` = args.
//! Return value in `%rax` (sign-extended `i32` from [`zircon_syscall_from_user`]).

use core::arch::global_asm;

use crate::ke::spinlock::SpinLock;

extern "C" {
    fn zircon_syscall_entry();
}

/// Sentinel matching [`SyscallTable::dispatch`] when the slot is empty.
pub const ZR_STATUS_NOT_IMPLEMENTED: i32 = -1_073_741_822;

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

/// Invoked from [`zircon_syscall_entry`]: `num` + six guest GPR arguments.
#[no_mangle]
extern "C" fn zircon_syscall_from_user(
    num: u64,
    a1: u64,
    a2: u64,
    a3: u64,
    a4: u64,
    a5: u64,
) -> i64 {
    let r = zr_syscall_dispatch(num as u16, a1, a2, a3, a4, a5, 0);
    if r == ZR_STATUS_NOT_IMPLEMENTED {
        crate::hal::x86_64::serial::write_line(b"nt10-kernel: user syscall smoke\r\n");
        crate::hal::x86_64::serial::write_bytes(b"nt10-kernel: syscall num ");
        crate::hal::x86_64::serial::write_hex_u64(num);
        crate::hal::x86_64::serial::write_line(b" return\r\n");
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
        mov rsi, [rsp]
        mov rdx, [rsp + 8]
        mov rcx, [rsp + 16]
        mov r8, [rsp + 24]
        mov r9, [rsp + 32]
        sub rsp, 40
        call zircon_syscall_from_user
        add rsp, 40
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
