//! SYSCALL/SYSRET; syscall dispatch table (original layout, public MSR names).

use core::arch::global_asm;

extern "C" {
    fn zircon_syscall_entry();
}

/// Invoked from [`zircon_syscall_entry`] with the user syscall number in `%rdi`.
#[no_mangle]
extern "C" fn zircon_syscall_from_user(_num: u64) {
    crate::hal::x86_64::serial::write_line(b"nt10-kernel: user syscall smoke\r\n");
}

global_asm!(
    r#"
    .globl zircon_syscall_entry
    zircon_syscall_entry:
        push rcx
        push r11
        mov rdi, rax
        call zircon_syscall_from_user
        pop r11
        pop rcx
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

/// Upper bound for syscall numbers reserved in this bring-up table.
pub const SYSCALL_TABLE_LEN: usize = 256;

/// Syscall handler: raw args from registers (extend later with trap frame).
pub type SyscallFn = fn(u64, u64, u64, u64, u64, u64) -> i32;

/// Fixed table; index is the OS syscall number (not yet wired to MSRs).
pub struct SyscallTable {
    pub entries: [Option<SyscallFn>; SYSCALL_TABLE_LEN],
}

impl SyscallTable {
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: core::array::from_fn(|_| None),
        }
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
