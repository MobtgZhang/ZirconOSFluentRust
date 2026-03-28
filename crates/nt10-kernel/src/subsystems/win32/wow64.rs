//! WOW64 thunk — x86 user code on x86_64 kernel (skeleton; [`crate::milestones::PHASE_WOW64`]).
//!
//! Process creation must keep [`crate::ps::process::EProcess::session_id`] and WOW64 PEB/TEB layout
//! in sync when dual 32/64 images load (`loader/pe32.rs` + syscall gate).

/// 32-bit context slice saved across the syscall gate (ZirconOS bring-up layout).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Wow64SyscallFrame {
    pub eax: u32,
    pub ecx: u32,
    pub edx: u32,
    pub ebx: u32,
    pub esp: u32,
    pub ebp: u32,
    pub esi: u32,
    pub edi: u32,
    /// User EIP before the syscall gate (ring 3).
    pub eip: u32,
    pub eflags: u32,
    /// Native return slot (x86_64 kernel writes syscall service result here).
    pub ret_value_lo: u32,
    pub ret_value_hi: u32,
}

impl Wow64SyscallFrame {
    pub const fn zeroed() -> Self {
        Self {
            eax: 0,
            ecx: 0,
            edx: 0,
            ebx: 0,
            esp: 0,
            ebp: 0,
            esi: 0,
            edi: 0,
            eip: 0,
            eflags: 0,
            ret_value_lo: 0,
            ret_value_hi: 0,
        }
    }
}

/// Map a WOW64 syscall index to the native syscall table (stub: identity on low indices).
#[must_use]
pub fn map_wow64_index(idx: u16) -> u16 {
    idx
}
