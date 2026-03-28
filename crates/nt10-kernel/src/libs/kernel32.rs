//! kernel32 — Win32-facing façade over `kernelbase` / `ntdll`.

#[inline]
pub fn get_last_error() -> u32 {
    super::win32_last_error::get_last_error()
}

#[inline]
pub fn set_last_error(code: u32) {
    super::win32_last_error::set_last_error(code);
}

/// Maps a bring-up `NTSTATUS` to a Win32 error for dual-stack callers.
#[inline]
pub fn map_ntstatus_to_last_error(status: super::ntdll::NtStatus) -> u32 {
    super::win32_last_error::ntstatus_to_win32_error(status)
}

#[inline]
pub fn create_file_w_stub() -> u64 {
    super::kernelbase::create_file_stub()
}

#[inline]
pub fn read_file_stub() -> u64 {
    super::kernelbase::read_file_stub()
}

#[inline]
pub fn exit_process(exit_code: u32) -> u64 {
    super::kernelbase::exit_process_via_nt(exit_code)
}

#[inline]
pub fn output_debug_string_a(ptr: u64, len: u64) -> u64 {
    super::kernelbase::output_debug_string(ptr, len)
}
