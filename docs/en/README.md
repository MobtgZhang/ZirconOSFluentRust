# ZirconOSFluent (NT10) Documentation (English)

**中文**：[../cn/README.md](../cn/README.md)

**Implementation**: Rust + Cargo; kernel module skeleton: [crates/nt10-kernel/src/](../../crates/nt10-kernel/src/) (mirrors architecture draft §4).

## Suggested reading order

1. [Architecture.md](Architecture.md) — Overview and “current repo vs target architecture”
2. [Boot-paths.md](Boot-paths.md) — UEFI vs `-kernel`, ring-3 smoke policy
3. [Roadmap-and-TODO.md](Roadmap-and-TODO.md) — Phase 0–15 and repository status
4. [Kernel-Executive-and-HAL.md](Kernel-Executive-and-HAL.md) — Boot, HAL, kernel executive
5. [Memory-and-Objects.md](Memory-and-Objects.md) — Memory manager, object manager
6. [Processes-Security-IO.md](Processes-Security-IO.md) — PS, SE, I/O, FS, ALPC
7. [Loader-Win32k-Desktop.md](Loader-Win32k-Desktop.md) — Loader, Win32k/WDDM, `desktop/fluent` modules
8. [Virtualization-Security-WinRT.md](Virtualization-Security-WinRT.md) — Hyper-V, CFG/CET, WinRT, multi-arch
9. [Build-Test-Coding.md](Build-Test-Coding.md) — Build, tests, coding standards (Cargo / `rustc`)
10. [PROCESS_NT10.md](PROCESS_NT10.md) — Contribution and doc sync
11. [References-Policy.md](References-Policy.md) — Using `references/win32` and `references/r-efi` legally and safely
12. [Syscall-ABI-ZirconOS.md](Syscall-ABI-ZirconOS.md) — x86_64 syscall ABI (project-specific)
13. [Ring3-bringup.md](Ring3-bringup.md) — ring-3 user-mode plan (outline)

**Cross-link convention**: At the top of each article you may add `中文: ../cn/<same filename>`.

**Source draft**: [../../ideas/ZirconOS_NT10_Architecture.md](../../ideas/ZirconOS_NT10_Architecture.md)
