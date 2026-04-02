//! Ring-3 bring-up: copy minimal user code and coordinate VAD + section bookkeeping.

use super::section::{install_bringup_section_vad, SectionObject};
use super::user_va::USER_BRINGUP_VA;
use super::vad::{PageProtect, VadEntry, VadKind, VadTable};

/// `syscall` then `sub rsp,8` then short jump back (demand-faults stack top once).
pub const USER_RING3_BRINGUP_CODE: [u8; 8] = [
    0x0f, 0x05, // syscall
    0x48, 0x83, 0xec, 0x08, // sub rsp, 8
    0xeb, 0xf8, // jmp -8 → syscall
];

/// Legacy 4-byte sequence (`syscall` + spin) for paths that pre-map the whole 2 MiB window.
pub const USER_SYSCALL_SMOKE_CODE: [u8; 4] = [0x0f, 0x05, 0xeb, 0xfe];

/// Ring-3 demo: `mov rax, 0x102` ([`crate::subsystems::win32::msg_dispatch::ZR_SYSCALL_GET_MESSAGE`]) then `syscall` then tight spin.
/// **ABI** (ZirconOS bring-up, not Windows): `%rax` = syscall number; `%rdi`..`%r9` = args per [`crate::arch::x86_64::syscall`] entry.
pub const USER_RING3_GETMESSAGE_SYSCALL_DEMO: [u8; 14] = [
    0x48, 0xB8, 0x02, 0x01, 0, 0, 0, 0, 0, 0, // mov rax, 0x102
    0x0F, 0x05, // syscall
    0xEB, 0xFE, // jmp short $ (spin)
];

/// # Safety
/// `dest` must be the kernel virtual alias of an identity-mapped, user-accessible page.
pub unsafe fn copy_user_smoke_code(dest_va: *mut u8) {
    unsafe {
        core::ptr::copy_nonoverlapping(
            USER_SYSCALL_SMOKE_CODE.as_ptr(),
            dest_va,
            USER_SYSCALL_SMOKE_CODE.len(),
        );
    }
}

/// Full UEFI-style bring-up blob (executable + stack demand-zero).
///
/// # Safety
/// Same as [`copy_user_smoke_code`].
pub unsafe fn copy_user_ring3_bringup_code(dest_va: *mut u8) {
    unsafe {
        core::ptr::copy_nonoverlapping(
            USER_RING3_BRINGUP_CODE.as_ptr(),
            dest_va,
            USER_RING3_BRINGUP_CODE.len(),
        );
    }
}

/// QEMU `-kernel` / built-in page tables: single section VAD (no per-page demand).
pub fn register_bringup_vad(vad: &mut VadTable) -> Result<(), ()> {
    let sec = SectionObject::bringup_readonly_user_window();
    install_bringup_section_vad(vad, &sec)
}

/// UEFI path: code page (RX) + remainder demand-zero stack/commit region (RW).
pub fn install_uefi_user_bringup_vads(vad: &mut VadTable) -> Result<(), ()> {
    use super::user_va::USER_BRINGUP_STACK_TOP;
    vad.insert(VadEntry::new_range(
        USER_BRINGUP_VA,
        USER_BRINGUP_VA.saturating_add(0x1000),
        VadKind::Private,
        PageProtect::ExecuteRead,
        true,
    ))?;
    vad.insert(VadEntry::new_range(
        USER_BRINGUP_VA.saturating_add(0x1000),
        USER_BRINGUP_STACK_TOP,
        VadKind::Private,
        PageProtect::ReadWrite,
        true,
    ))?;
    Ok(())
}

#[must_use]
pub fn user_code_entry_va() -> u64 {
    USER_BRINGUP_VA
}

#[must_use]
pub fn user_ring3_bringup_code_len() -> usize {
    USER_RING3_BRINGUP_CODE.len()
}
