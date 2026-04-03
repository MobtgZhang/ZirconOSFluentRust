//! Minimal `Nt*` syscall handlers and **duplicate registration** at ZirconOS-local indices from
//! [`crate::libs::ntdll::numbers`] plus Windows 10 22H2 indices from [`super::nt_syscall_indices`].
//!
//! Full semantics come later; today we return success or `STATUS_NOT_IMPLEMENTED` without touching user pointers.

#![cfg(target_arch = "x86_64")]

use super::nt_syscall_indices::*;
use super::syscall::zr_syscall_register;
use crate::libs::ntdll::numbers;
use crate::mm::bringup_syscall_vm;

const STATUS_NOT_IMPLEMENTED: i32 = -1_073_741_822;
const STATUS_SUCCESS: i32 = 0;

fn sc_not_implemented(
    _a1: u64,
    _a2: u64,
    _a3: u64,
    _a4: u64,
    _a5: u64,
    _a6: u64,
) -> i32 {
    STATUS_NOT_IMPLEMENTED
}

fn sc_success(
    _a1: u64,
    _a2: u64,
    _a3: u64,
    _a4: u64,
    _a5: u64,
    _a6: u64,
) -> i32 {
    STATUS_SUCCESS
}

fn sc_nt_allocate_virtual_memory_bringup(
    _a1: u64,
    _a2: u64,
    _a3: u64,
    _a4: u64,
    _a5: u64,
    _a6: u64,
) -> i32 {
    bringup_syscall_vm::nt_allocate_virtual_memory_syscall_stub()
}

fn sc_nt_terminate_process_bringup(
    _a1: u64,
    _a2: u64,
    _a3: u64,
    _a4: u64,
    _a5: u64,
    _a6: u64,
) -> i32 {
    crate::rtl::log::log_line_serial(crate::rtl::log::SUB_SYSC, b"NtTerminateProcess_bringup_stub");
    STATUS_SUCCESS
}

/// Register both Zircon-local `numbers::*` slots and NT 10 22H2 aliases (idempotent best-effort).
pub fn register_nt_syscall_stubs_bringup() {
    let ni = sc_not_implemented;
    let ok = sc_success;
    let term = sc_nt_terminate_process_bringup;
    let alloc = sc_nt_allocate_virtual_memory_bringup;

    let _ = zr_syscall_register(numbers::NT_TERMINATE_PROCESS as u16, term);
    let _ = zr_syscall_register(numbers::NT_READ_FILE as u16, ni);
    let _ = zr_syscall_register(numbers::NT_WRITE_FILE as u16, ni);
    let _ = zr_syscall_register(numbers::NT_CREATE_FILE as u16, ni);
    let _ = zr_syscall_register(numbers::NT_ALLOCATE_VIRTUAL_MEMORY as u16, alloc);
    let _ = zr_syscall_register(numbers::NT_OUTPUT_DEBUG_STRING as u16, ni);
    let _ = zr_syscall_register(numbers::NT_QUERY_SYSTEM_TIME as u16, ok);

    let _ = zr_syscall_register(NT10_22H2_NT_TERMINATE_PROCESS, term);
    let _ = zr_syscall_register(NT10_22H2_NT_READ_FILE, ni);
    let _ = zr_syscall_register(NT10_22H2_NT_WRITE_FILE, ni);
    let _ = zr_syscall_register(NT10_22H2_NT_CLOSE, ok);
    let _ = zr_syscall_register(NT10_22H2_NT_ALLOCATE_VIRTUAL_MEMORY, alloc);
    let _ = zr_syscall_register(NT10_22H2_NT_FREE_VIRTUAL_MEMORY, ni);
    let _ = zr_syscall_register(NT10_22H2_NT_CREATE_FILE, ni);
    let _ = zr_syscall_register(NT10_22H2_NT_PROTECT_VIRTUAL_MEMORY, ni);
    let _ = zr_syscall_register(NT10_22H2_NT_QUERY_SYSTEM_TIME, ok);
}
