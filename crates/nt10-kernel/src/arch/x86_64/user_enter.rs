//! `iretq` path into ring 3 for bring-up (no TSS IST yet — user stack passed explicitly).

use core::arch::global_asm;

// Single dialect for `cargo test` (host) and `x86_64-unknown-none` (same LLVM Intel default).
global_asm!(
    r#"
    .globl zircon_enter_ring3
    zircon_enter_ring3:
        cli
        push 0x1B
        push rsi
        push 0x202
        push 0x23
        push rdi
        iretq
    "#,
);

extern "C" {
    /// User `RIP` in `%rdi`, user `RSP` in `%rsi`. GDT: `0x23` = ring-3 code, `0x1b` = ring-3 data.
    pub fn zircon_enter_ring3(user_rip: u64, user_rsp: u64) -> !;
}
