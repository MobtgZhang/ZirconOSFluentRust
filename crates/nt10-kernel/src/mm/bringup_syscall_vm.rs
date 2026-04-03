//! Virtual-memory syscall stubs until full Nt* user-pointer parsing exists.
//!
//! See [MM-Goals-and-Invariants.md](../../../../docs/en/MM-Goals-and-Invariants.md).

/// `STATUS_NO_MEMORY` — PFN pool cannot satisfy the request.
pub const STATUS_NO_MEMORY: i32 = 0xC000_0017u32 as i32;

/// `STATUS_NOT_IMPLEMENTED`
pub const STATUS_NOT_IMPLEMENTED: i32 = -1_073_741_822;

/// Bring-up path for `NtAllocateVirtualMemory` indices: surfaces pool pressure without dereferencing user pointers.
#[must_use]
pub fn nt_allocate_virtual_memory_syscall_stub() -> i32 {
    if crate::mm::phys::pfn_pool_starved_flag() {
        return STATUS_NO_MEMORY;
    }
    crate::rtl::log::log_line_serial(crate::rtl::log::SUB_MM, b"NtAllocateVirtualMemory_syscall_stub");
    STATUS_NOT_IMPLEMENTED
}
