# .NET and user-mode policy (ZirconOSFluent / NT10-aligned)

**中文**：[../cn/DotNet-UserMode.md](../cn/DotNet-UserMode.md)

## 1. Ecosystem context

On Windows 10 (including 10.0.19045), many **desktop apps, tools, and management components** depend on **.NET Framework** or **.NET (Core)**. For ZirconOSFluent as a long-term NT10-behavior-aligned platform, this user-mode dependency must be acknowledged rather than assuming all workloads are native PE-only.

## 2. Relationship to this repository

- **Kernel vs user mode**: The kernel (`nt10-kernel`) is `no_std` Rust; **the CLR, JIT, and BCL are not implemented in the kernel**.
- **Compliance**: Any future managed runtime and libraries must follow [CONTRIBUTING.md](../../CONTRIBUTING.md) — clean-room implementation or standards-based compatibility, **no copying** of proprietary Windows / .NET source.
- **Full compatibility (future)**: A ring-3 **CLR host** and **BCL subset** would interact with the kernel via existing **NT-style syscalls** (virtual memory, sections, threads, …); scope is a future roadmap item.

## 3. Why PowerShell is out of scope

**Windows PowerShell** is built on **.NET**. Without a .NET runtime in this project, a faithful PowerShell implementation is not realistic; **this repository does not implement PowerShell**, and kernel-side PowerShell placeholders have been removed.

Shell bring-up focuses on **CMD-style stubs**; a **PowerShell-like scripting experience** should be planned as a **future .NET user-mode host**, not as kernel-duplicated PowerShell semantics.

## 4. Tie-in to the memory manager

Managed processes use `VirtualAlloc` / `NtAllocateVirtualMemory` patterns with heavy **reserve/commit**; JIT code requires **executable mappings with NX/DEP**. That aligns with VAD, demand paging, and page protections described in [Memory-and-Objects.md](Memory-and-Objects.md). The kernel exposes **documented MM semantics**; it does not need to identify “.NET” specifically.

## 5. See also

- [Memory-and-Objects.md](Memory-and-Objects.md) — MM layout, VAD, sections
- [Ring3-bringup.md](Ring3-bringup.md) — Ring-3 plan
- [CONTRIBUTING.md](../../CONTRIBUTING.md) — reference policy and copyright
