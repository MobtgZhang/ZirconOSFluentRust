# ZirconOSFluent architecture overview (NT 10.0)

**中文**：[../cn/Architecture.md](../cn/Architecture.md)

> **Disclaimer**: ZirconOSFluent is not affiliated with Microsoft. “Windows” and “Windows 10” are trademarks of Microsoft Corporation; this text describes compatibility goals only.

## 1. Project positioning

**ZirconOSFluent** is the public project name; the kernel effort is **NT10**, using **Windows NT 10.0.19045** (Windows 10 21H2—the repo’s fixed documentation / syscall baseline) as the design reference, implemented primarily in **Rust** with a small amount of **assembly** (`global_asm!` or standalone `.S`). **This repository targets NT 10.0-era capabilities only; there is no Windows NT 6.x compatibility or migration goal.**

**Core direction**:

- **Hybrid microkernel**: scheduling, memory, interrupts, IPC in kernel mode; object manager, I/O manager, security, etc. as executive components; Win32 subsystem servers may run in user mode.
- **NT semantics first**: prefer `Nt*` / `Zw*` and the NT object model over POSIX-centric APIs.
- **Modern security**: VBS, HVCI, Secure Boot, TPM 2.0 abstractions (see [Virtualization-Security-WinRT.md](Virtualization-Security-WinRT.md)).
- **Rust engineering**: types, documented `unsafe` boundaries, `repr(C)` for layout-sensitive structures, controlled allocation (custom pools / future `allocator` traits) to reduce UB.

## 2. NT 10.0 design pillars (single generation)

Dimensions intentionally in scope when aligning with **NT 10.0** (not a diff table vs older NT):

| Area | NT 10.0-oriented target |
|------|-------------------------|
| Boot | **UEFI only**, ZBM10 |
| IPC | **ALPC** |
| Display | **WDDM 2.x** (D3D12-aware) and Fluent desktop direction |
| Security | Token, SID, ACL, plus VBS / HVCI / CET / CFG (planned) |
| Virtualization | **Hyper-V awareness** |
| Runtime | Win32 + **WinRT / UWP AppModel** (long term) |
| WOW64 | **x86/ARM32 → x64/ARM64** thunk direction |
| Paging | 4-level primary, **5-level LA57** optional |
| Syscall numbering | Project-local ABI ([Syscall-ABI-ZirconOS.md](Syscall-ABI-ZirconOS.md)), **19041** as doc baseline; **not** binary-identical to any Windows build |

## 3. Layered architecture (target)

### 3.1 ASCII summary (same as draft)

```
User (Ring 3): UWP / Win32 / CMD·PS·Terminal / services
  → ntdll / kernel32 / kernelbase / user32 / gdi32 / combase / winrt …
  → syscall
Kernel (Ring 0): KE MM OB PS SE IO FS ALPC; Win32k / WDM / Loader / RTL
  → HAL → arch (x86_64 / aarch64 / …)
Hypervisor awareness: CPUID / hypercalls / VSM …
```

### 3.2 Mermaid

```mermaid
flowchart TB
  subgraph userRing3 [UserMode_Ring3]
    Apps[Win32_UWP_CMD_Services]
    ApiLayer[ntdll_kernelbase_user32_gdi32]
  end
  subgraph kernelRing0 [KernelMode_Ring0]
    Exec[NT_Executive_KE_MM_OB_PS_SE_IO_FS_ALPC]
    Win32k[Win32k_WDM_Loader_RTL]
    HAL[HAL]
    ARCH[ARCH_x86_64_aarch64_etc]
  end
  subgraph hypervisor [HypervisorAware]
    Hyp[CPUID_Hypercalls_VSM]
  end
  Apps --> ApiLayer
  ApiLayer -->|syscall| Exec
  Exec --> HAL
  HAL --> ARCH
  Exec --> Hyp
```

## 4. Target source tree (summary)

Full layout: [ideas/ZirconOS_NT10_Architecture.md](../../ideas/ZirconOS_NT10_Architecture.md) §4. **This repo (Rust)**:

- **UEFI ZBM10 stub**: [crates/nt10-boot-uefi/](../../crates/nt10-boot-uefi/)
- **Kernel library**: [crates/nt10-kernel/src/](../../crates/nt10-kernel/src/) — `arch/`, `hal/`, `ke/`, `mm/`, `ob/`, `ps/`, `se/`, `io/`, `alpc/`, `fs/`, `loader/`, `hyperv/`, `vbs/`, `drivers/`, `libs/`, `servers/`, `subsystems/win32/`, `desktop/fluent/`, etc.
- **Linker script (stub)**: [link/x86_64.ld](../../link/x86_64.ld)

## 5. Current repo vs target

| Area | Current | Target (draft) |
|------|---------|----------------|
| Kernel | **[crates/nt10-kernel](../../crates/nt10-kernel/)**: `#![no_std]` library, module tree aligned with §4, stub implementations | Full NT10 semantics |
| Boot | **[crates/nt10-boot-uefi](../../crates/nt10-boot-uefi/)**: `efi_main`, GOP/memory map/handoff, FAT load of `NT10KRNL.BIN`, jump to kernel at 1 MiB | ZBM10, BCD, Secure Boot, full PE loader |
| Fluent | [desktop/fluent](../../crates/nt10-kernel/src/desktop/fluent/) stubs; [resources/](../../resources/) pack | Win32k/WDDM + shell |
| Build | **[Cargo.toml](../../Cargo.toml)** workspace; `cargo check -p nt10-kernel --target x86_64-unknown-none`; `cargo kcheck` alias ([.cargo/config.toml](../../.cargo/config.toml)) | ISO/QEMU scripts, feature flags (`xtask` or scripts later) |

This document describes the **target** architecture; for **what builds today**, see the root [README.md](../../README.md), [Build-Test-Coding.md](Build-Test-Coding.md), and `cargo check`.

## 6. Related docs

- [Roadmap-and-TODO.md](Roadmap-and-TODO.md)
- [References-Policy.md](References-Policy.md)
- [Syscall-ABI-ZirconOS.md](Syscall-ABI-ZirconOS.md)
- [Kernel-Executive-and-HAL.md](Kernel-Executive-and-HAL.md)
- [Loader-Win32k-Desktop.md](Loader-Win32k-Desktop.md)
