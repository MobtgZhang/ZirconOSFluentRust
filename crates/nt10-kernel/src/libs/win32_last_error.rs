//! Bring-up `GetLastError` / `SetLastError` — single-threaded kernel stub uses one atomic slot.
//!
//! Win32 layer maps many failures to Win32 error codes; `NTSTATUS` stays on the native path
//! (`ntdll::NtStatus`) and is converted at the syscall façade when needed.

use core::sync::atomic::{AtomicU32, Ordering};

static LAST_ERROR: AtomicU32 = AtomicU32::new(0);

/// `ERROR_SUCCESS`
pub const ERROR_SUCCESS: u32 = 0;
/// `ERROR_INVALID_PARAMETER`
pub const ERROR_INVALID_PARAMETER: u32 = 87;
/// `ERROR_NOT_ENOUGH_MEMORY`
pub const ERROR_NOT_ENOUGH_MEMORY: u32 = 8;
/// `ERROR_GEN_FAILURE`
pub const ERROR_GEN_FAILURE: u32 = 31;

#[inline]
pub fn set_last_error(code: u32) {
    LAST_ERROR.store(code, Ordering::Relaxed);
}

#[inline]
pub fn get_last_error() -> u32 {
    LAST_ERROR.load(Ordering::Relaxed)
}

/// Stub: map a subset of `NTSTATUS` values to Win32 errors (expand with real table later).
#[inline]
pub fn ntstatus_to_win32_error(status: u32) -> u32 {
    if status == 0 {
        return ERROR_SUCCESS;
    }
    ERROR_GEN_FAILURE
}
