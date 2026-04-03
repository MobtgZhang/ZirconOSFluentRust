//! Windows **NT** syscall indices for x64 — **Windows 10 22H2** row from the public dataset
//! [j00ru/windows-syscalls](https://github.com/j00ru/windows-syscalls) (`nt-per-syscall.json`, `Windows 10` → `22H2`).
//!
//! These are **factual identifiers** for interoperability testing, not copied Microsoft code.
//! Build `19045` (NT 10.0) aligns with the project Roadmap baseline; 22H2 and 2004 share the same indices
//! for the symbols below in that dataset.

/// `NtReadFile`
pub const NT10_22H2_NT_READ_FILE: u16 = 6;
/// `NtWriteFile`
pub const NT10_22H2_NT_WRITE_FILE: u16 = 8;
/// `NtClose`
pub const NT10_22H2_NT_CLOSE: u16 = 15;
/// `NtAllocateVirtualMemory`
pub const NT10_22H2_NT_ALLOCATE_VIRTUAL_MEMORY: u16 = 24;
/// `NtFreeVirtualMemory`
pub const NT10_22H2_NT_FREE_VIRTUAL_MEMORY: u16 = 30;
/// `NtTerminateProcess`
pub const NT10_22H2_NT_TERMINATE_PROCESS: u16 = 44;
/// `NtCreateFile`
pub const NT10_22H2_NT_CREATE_FILE: u16 = 85;
/// `NtProtectVirtualMemory`
pub const NT10_22H2_NT_PROTECT_VIRTUAL_MEMORY: u16 = 80;
/// `NtQuerySystemTime`
pub const NT10_22H2_NT_QUERY_SYSTEM_TIME: u16 = 90;
