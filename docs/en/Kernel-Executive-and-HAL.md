# Boot, HAL, and Kernel Executive (KE)

**中文**: [../cn/Kernel-Executive-and-HAL.md](../cn/Kernel-Executive-and-HAL.md)

This document covers the source draft §5–§7: boot subsystem, HAL, scheduling / IRQL / DPC / APC. **Implementation status**: design target; boot code is planned in **[crates/nt10-boot-uefi](../../crates/nt10-boot-uefi/)**; HAL/KE stubs live under **[crates/nt10-kernel/src/hal/](../../crates/nt10-kernel/src/hal/)** and **[ke/](../../crates/nt10-kernel/src/ke/)** (see [Architecture.md](Architecture.md)).

## 1. Boot subsystem (ZBM10)

**ZBM10** (ZirconOS Boot Manager 10) is planned as **UEFI-only** (no legacy BIOS/MBR path).

### 1.1 Boot flow

```
UEFI firmware
  → ZBM10 (planned in nt10-boot-uefi: main / BCD / menu / Secure Boot)
  → Secure Boot verification
  → BCD parsing
  → Boot menu (Fluent text UI)
  → Load kernel EFI image
  → GetMemoryMap / GOP
  → ExitBootServices
  → Jump to kernel entry (future nt10-kernel binary or static lib + linker script)
```

### 1.2 ZirconBootInfo (planned)

Example handoff structure; in Rust use `#[repr(C)]`:

```rust
#[repr(C)]
pub struct ZirconBootInfo {
    pub magic: u64, // 0x5A49524E4F4E3130 "ZIRNON10"
    pub mem_map: *mut MemoryDescriptor,
    pub mem_map_count: usize,
    pub framebuffer: FramebufferInfo,
    pub acpi_rsdp: u64,
    pub tpm_base: u64,
    pub cmdline: [u8; 256],
    pub kernel_base: u64,
    pub initrd_base: u64,
    pub initrd_size: usize,
}
```

Placeholder constant: `ZIRNON10_MAGIC` in [crates/nt10-boot-uefi/src/lib.rs](../../crates/nt10-boot-uefi/src/lib.rs).

### 1.3 Secure Boot

Verify PE signatures before loading the kernel (e.g. via `EFI_SECURITY2_ARCHITECTURAL_PROTOCOL`) to anchor later HVCI-style trust chains.

## 2. Hardware abstraction layer (HAL)

The HAL wraps platform operations; the executive talks to hardware only through it. Express polymorphism with **traits**, **generics**, or **`cfg(target_arch)`** instead of the draft’s comptime wording:

```rust
pub trait Hal {
    fn mask_irq(&self, vector: u8);
    fn unmask_irq(&self, vector: u8);
    fn send_eoi(&self);
    fn current_time_ns(&self) -> u64;
    fn set_timer(&self, ns: u64, cb: TimerCallback);
    fn flush_tlb(&self, addr: u64);
    fn flush_tlb_all(&self);
    fn debug_write(&self, s: &[u8]);
}
```

**x86_64**: prefer APIC + HPET + invariant TSC; fall back to PIC + PIT. Under Hyper-V, use enlightenment time sources where applicable ([Virtualization-Security-WinRT.md](Virtualization-Security-WinRT.md)).

**Module path**: [crates/nt10-kernel/src/hal/](../../crates/nt10-kernel/src/hal/).

## 3. Kernel executive core (KE)

### 3.1 Scheduler

- Multi-level feedback queue, **32 priorities** (0–31), NT `KPRIORITY`-style.
- 0–15: dynamic (typical user work); 16–31: real-time class.
- Priority inheritance, CPU affinity (`KAFFINITY`), NUMA awareness.

### 3.2 IRQL model (summary)

| IRQL | Name | Role |
|------|------|------|
| 0 | PASSIVE_LEVEL | Normal kernel/user code |
| 1 | APC_LEVEL | APCs |
| 2 | DISPATCH_LEVEL | DPCs, scheduler |
| 3–26 | DIRQL | Device interrupts |
| 27 | PROFILE_LEVEL | Profiling |
| 28 | CLOCK_LEVEL | Clock |
| 29 | IPI_LEVEL | IPI |
| 30 | POWER_LEVEL | Power |
| 31 | HIGH_LEVEL | NMI, etc. |

### 3.3 DPC / APC

- **DPC**: runs at `DISPATCH_LEVEL`, driver bottom halves; **per-CPU queues**.
- **KernelAPC**: `APC_LEVEL`, e.g. thread state, memory completion.
- **UserAPC**: asynchronous user-mode execution (`NtQueueApcThread`).

### 3.4 Modules in this repo

[ke/sched.rs](../../crates/nt10-kernel/src/ke/sched.rs), `timer.rs`, `dpc.rs`, `apc.rs`, `irq.rs`, `spinlock.rs`, `mutex.rs`, `event.rs`, `semaphore.rs`, `waitobj.rs`, `trap.rs`, etc.

## 4. Related docs

- [Memory-and-Objects.md](Memory-and-Objects.md)
- [Build-Test-Coding.md](Build-Test-Coding.md)
