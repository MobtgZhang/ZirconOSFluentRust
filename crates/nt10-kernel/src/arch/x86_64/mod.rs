//! x86_64: boot, paging, IDT, syscalls, …

pub mod boot;
pub mod cet;
pub mod cpuid;
pub mod gdt;
pub mod idt;
pub mod isr;
pub mod msr;
pub mod nt_syscall_indices;
#[cfg(target_arch = "x86_64")]
pub mod nt_syscall_stubs;
pub mod paging;
pub mod syscall;
pub mod syscall_abi;
pub mod tlb;
pub mod user_enter;
pub mod vmx;
