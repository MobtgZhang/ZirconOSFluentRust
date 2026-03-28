//! Ring-3 bring-up: copy minimal user code and coordinate VAD + section bookkeeping.

use super::section::{install_bringup_section_vad, SectionObject};
use super::user_va::USER_BRINGUP_VA;
use super::vad::VadTable;

/// `syscall` then `jmp .` — x86_64 machine code.
pub const USER_SYSCALL_SMOKE_CODE: [u8; 4] = [0x0f, 0x05, 0xeb, 0xfe];

/// # Safety
/// `dest` must be the kernel virtual alias of an identity-mapped, user-accessible page.
pub unsafe fn copy_user_smoke_code(dest_va: *mut u8) {
    unsafe {
        core::ptr::copy_nonoverlapping(USER_SYSCALL_SMOKE_CODE.as_ptr(), dest_va, USER_SYSCALL_SMOKE_CODE.len());
    }
}

/// Applies built-in section + VAD for the user smoke window.
pub fn register_bringup_vad(vad: &mut VadTable) -> Result<(), ()> {
    let sec = SectionObject::bringup_readonly_user_window();
    install_bringup_section_vad(vad, &sec)
}

#[must_use]
pub fn user_code_entry_va() -> u64 {
    USER_BRINGUP_VA
}
