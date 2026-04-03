//! Roadmap phase markers from [docs/en/Roadmap-and-TODO.md](../../../docs/en/Roadmap-and-TODO.md).
//! These constants are for documentation, logging, and future feature gates — not a runtime state machine.

/// Kernel infrastructure hardening (page fault reasons, I/O façade, SMSS chain tests, namespace DAC hooks).
pub const PHASE_KERNEL_INFRA: u8 = 16;
/// VirtIO-MMIO block polling + VFS/IRP dispatch (no PCI virtio yet).
pub const PHASE_VIRTIO_MMIO_BLK: u8 = 17;
/// SMP-aware TLB flush after PTE edits ([`crate::arch::x86_64::tlb::flush_after_pte_change`]).
pub const PHASE_SMP_TLB_FLUSH: u8 = 18;
/// NT 10–style x64 syscall indices (public tables) + dual registration with Zircon-local [`crate::libs::ntdll::numbers`].
pub const PHASE_NT_SYSCALL_DUAL_ABI: u8 = 19;
/// MM goals / invariants doc + bring-up hooks (`pfn_pool_starved_flag`, `SectionCommitError`, `VadTable::clear`, …).
pub const PHASE_MM_GOALS_AND_HOOKS: u8 = 20;

/// Phase 10 — Win32k / user-mode graphics path.
pub const PHASE_WIN32K_GRAPHICS: u8 = 10;
/// Phase 11 — WOW64-style thunking.
pub const PHASE_WOW64: u8 = 11;
/// Phase 12 — CFG, CET, DEP, MIC hardening.
pub const PHASE_MODERN_SECURITY: u8 = 12;
/// Phase 13 — Hyper-V awareness.
pub const PHASE_HYPERV: u8 = 13;
/// Phase 14 — Fluent shell / DWM integration.
pub const PHASE_FLUENT_DESKTOP: u8 = 14;
/// Phase 15 — WinRT / UWP protocol stubs.
pub const PHASE_WINRT: u8 = 15;
