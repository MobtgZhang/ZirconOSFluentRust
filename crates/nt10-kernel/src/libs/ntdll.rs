//! ntdll API surface — syscall indices are **ZirconOS-local** (do not assume Windows build numbers).
//!
//! Full ABI notes: [Syscall-ABI-ZirconOS.md](../../../../docs/en/Syscall-ABI-ZirconOS.md).
//!
//! `NTSTATUS` values stay on this layer; Win32 `GetLastError` is [`super::win32_last_error`].

/// Syscall number type (kernel ABI TBD).
pub type SyscallNumber = u32;

/// Native NT status code (`NTSTATUS`) — do not conflate with Win32 error codes.
pub type NtStatus = u32;

/// Bump when renumbering [`numbers`] so user stubs and tests can detect mismatch.
pub const SYSCALL_NUMBERING_REVISION: u32 = 2;

/// Single source of syscall numbers; bump [`SYSCALL_NUMBERING_REVISION`] when editing.
pub mod numbers {
    use super::SyscallNumber;

    pub const NT_TERMINATE_PROCESS: SyscallNumber = 0x001;
    pub const NT_READ_FILE: SyscallNumber = 0x002;
    pub const NT_WRITE_FILE: SyscallNumber = 0x003;
    pub const NT_CREATE_FILE: SyscallNumber = 0x004;
    pub const NT_ALLOCATE_VIRTUAL_MEMORY: SyscallNumber = 0x005;
    pub const NT_OUTPUT_DEBUG_STRING: SyscallNumber = 0x006;
    pub const NT_QUERY_SYSTEM_TIME: SyscallNumber = 0x007;
}

/// Placeholder: would emit `syscall` after kernel sets `LSTAR`.
#[inline]
pub fn syscall0(_n: SyscallNumber) -> u64 {
    0
}

/// Six-argument syscall stub (matches common x86_64 register packing).
#[inline]
pub fn syscall6(
    _n: SyscallNumber,
    _a1: u64,
    _a2: u64,
    _a3: u64,
    _a4: u64,
    _a5: u64,
    _a6: u64,
) -> u64 {
    0
}

/// User stub: terminate current process (kernel interprets `exit_code` in a future ABI).
#[inline]
pub fn nt_terminate_process_stub(exit_code: u64) -> u64 {
    syscall6(numbers::NT_TERMINATE_PROCESS, exit_code, 0, 0, 0, 0, 0)
}

/// User stub: output debug string (`ptr`, `len` in bytes — exact semantics TBD).
#[inline]
pub fn nt_output_debug_string_stub(ptr: u64, len: u64) -> u64 {
    syscall6(numbers::NT_OUTPUT_DEBUG_STRING, ptr, len, 0, 0, 0, 0)
}

/// User stub: reserve/commit virtual memory (opaque args for bring-up).
#[inline]
pub fn nt_allocate_virtual_memory_stub(
    base_hint: u64,
    size: u64,
    alloc_type: u64,
    protect: u64,
) -> u64 {
    syscall6(
        numbers::NT_ALLOCATE_VIRTUAL_MEMORY,
        base_hint,
        size,
        alloc_type,
        protect,
        0,
        0,
    )
}

/// User stub: query system time (returns status in high-level ABI TBD).
#[inline]
pub fn nt_query_system_time_stub() -> u64 {
    syscall6(numbers::NT_QUERY_SYSTEM_TIME, 0, 0, 0, 0, 0, 0)
}
