# NT 10.0 binary compatibility — scope (ZirconOSFluent)

**中文**：[../cn/Binary-compat-NT10-scope.md](../cn/Binary-compat-NT10-scope.md)

This note defines **what “compatibility” means** in this repository and how it coexists with [Clean-Room-Implementation.md](Clean-Room-Implementation.md).

## Phases

| Phase | Goal | Retail Windows binaries |
|-------|------|-------------------------|
| **A** | Self-built PE/DLL linked against **this tree’s** stubs ([`ntdll.rs`](../../crates/nt10-kernel/src/libs/ntdll.rs)) | Not targeted |
| **B** | Same as A, plus **Windows 10 22H2 x64 syscall indices** for selected `Nt*` APIs (public index tables) and matching **x64 unpack** in the kernel | Only where we explicitly test a minimal import surface |
| **C** | Broader syscall + loader + subsystem coverage | Long-term; still clean-room |

We are implementing **B infrastructure** now: dual registration at Zircon-local `numbers::*` and at NT indices (see [`nt_syscall_indices.rs`](../../crates/nt10-kernel/src/arch/x86_64/nt_syscall_indices.rs)), with [`syscall_abi.rs`](../../crates/nt10-kernel/src/arch/x86_64/syscall_abi.rs) selecting **LegacyZircon** (default) vs **Nt10X64** unpack.

## First syscall set (B)

Handlers are stubs unless noted in the Roadmap: `NtTerminateProcess`, `NtReadFile`, `NtWriteFile`, `NtClose`, `NtAllocateVirtualMemory`, `NtFreeVirtualMemory`, `NtCreateFile`, `NtProtectVirtualMemory`, `NtQuerySystemTime` — indices from the public **j00ru** dataset ([GitHub](https://github.com/j00ru/windows-syscalls)), Windows 10 **22H2** column.

## Compliance

- **Do not** use Windows retail binaries as a primary implementation source.
- **Do** use public specs (Intel SDM for `syscall`, PE/COFF for the loader) and **curated public syscall tables** (facts, not code).
- **Do** validate behavior with **self-built** test programs in QEMU or on licensed reference systems.

## Related

- [Syscall-ABI-ZirconOS.md](Syscall-ABI-ZirconOS.md)
- [Public-References.md](Public-References.md)
