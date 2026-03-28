//! kernelbase — split layer over [`super::ntdll`] syscall stubs (ZirconOS numbering; see project syscall ABI doc).

/// `CreateFileW` eventually maps to `NtCreateFile` — syscall wiring is kernel-side.
#[inline]
pub fn create_file_stub() -> u64 {
    super::ntdll::syscall6(super::ntdll::numbers::NT_CREATE_FILE, 0, 0, 0, 0, 0, 0)
}

/// Read path stub.
#[inline]
pub fn read_file_stub() -> u64 {
    super::ntdll::syscall6(super::ntdll::numbers::NT_READ_FILE, 0, 0, 0, 0, 0, 0)
}

#[inline]
pub fn exit_process_via_nt(exit_code: u32) -> u64 {
    super::ntdll::nt_terminate_process_stub(u64::from(exit_code))
}

#[inline]
pub fn output_debug_string(ptr: u64, len: u64) -> u64 {
    super::ntdll::nt_output_debug_string_stub(ptr, len)
}

#[inline]
pub fn allocate_virtual_memory(
    base_hint: u64,
    size: u64,
    alloc_type: u64,
    protect: u64,
) -> u64 {
    super::ntdll::nt_allocate_virtual_memory_stub(base_hint, size, alloc_type, protect)
}
