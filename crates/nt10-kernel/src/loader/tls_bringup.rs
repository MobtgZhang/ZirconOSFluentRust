//! TLS directory — parsed in [`super::pe_image::Pe64Headers`] but not executed.
//!
//! Full TLS: allocate per-thread slot array, run TLS callbacks on thread start, and wire
//! `NtCurrentTeb()->ThreadLocalStoragePointer`. Thread pools share the same gap until loader grows.

use super::pe_image::Pe64Headers;

/// Records presence of a TLS directory for diagnostics (no allocation yet).
#[inline]
pub fn record_tls_directory_present(headers: &Pe64Headers) {
    let _ = (headers.tls_rva, headers.tls_size);
}
