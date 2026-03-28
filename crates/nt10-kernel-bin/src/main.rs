//! Low-address kernel image entry (`_start` → `nt10_kernel::kmain`).
//! Linked at physical 1 MiB ([`link/x86_64-uefi-load.ld`](../../../link/x86_64-uefi-load.ld)).
//!
//! Boot contract: `ZirconBootInfo *` in `%rdi` when launched from ZBM10; garbage or null is OK for
//! bring-up (handoff is validated in [`nt10_kernel::kmain::kmain_entry`]).

#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[used]
#[link_section = ".bss.bootstack"]
#[no_mangle]
static BOOT_STACK: [u8; 65536] = [0u8; 65536];

#[unsafe(naked)]
#[no_mangle]
#[link_section = ".text.boot"]
pub extern "C" fn _start() -> ! {
    // `x86_64-unknown-none` uses Intel assembly dialect by default; these templates are AT&T.
    // Preserve `%rdi`: ZBM10 passes physical pointer to `ZirconBootInfo`; QEMU `-kernel` may leave it
    // undefined (kernel rejects non-matching magic).
    core::arch::naked_asm!(
        "cli",
        "lea {lo}(%rip), %rax",
        "add ${sz}, %rax",
        "mov %rax, %rsp",
        "xor %ebp, %ebp",
        "call {k}",
        "0:",
        "hlt",
        "jmp 0b",
        lo = sym BOOT_STACK,
        sz = const 65536usize,
        k = sym kmain_trampoline,
        options(att_syntax),
    );
}

/// Handoff pointer in `%rdi` (System V x86_64) when entered from ZBM10.
#[no_mangle]
extern "C" fn kmain_trampoline(boot: *const nt10_kernel::handoff::ZirconBootInfo) -> ! {
    // SAFETY: `_start` passes null for bring-up; UEFI would pass a valid handoff pointer.
    unsafe { nt10_kernel::kmain::kmain_entry(boot) }
}
