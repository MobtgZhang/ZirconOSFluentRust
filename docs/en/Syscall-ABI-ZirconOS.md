# ZirconOS syscall ABI (x86_64)

**中文**：[../cn/Syscall-ABI-ZirconOS.md](../cn/Syscall-ABI-ZirconOS.md)

This document describes how the **ZirconOS NT10** kernel exposes system services on x86_64. It is **project-specific**: do not treat it as Microsoft documentation.

## Goals

- **Stable hand-written ABI** between [`nt10-kernel`](../../crates/nt10-kernel/src/lib.rs) and future user-mode stubs ([`libs/ntdll`](../../crates/nt10-kernel/src/libs/ntdll.rs), etc.).
- **Behavioral compatibility** with Windows NT 10-class systems where practical, using **public facts** (register convention, syscall instruction) rather than pasting copyrighted reference text.

## Mechanism

- User mode invokes `syscall`; the CPU transfers to the address in `IA32_LSTAR` with a documented register convention (see Intel SDM / AMD APM for the instruction, not reproduced here).
- The kernel maintains a **dispatch table** ([`arch/x86_64/syscall.rs`](../../crates/nt10-kernel/src/arch/x86_64/syscall.rs)). Slots may be registered more than once: **Zircon-local** indices in [`libs/ntdll.rs`](../../crates/nt10-kernel/src/libs/ntdll.rs) `numbers` and **Windows 10 22H2 x64** `Nt*` indices in [`libs/ntdll.rs`](../../crates/nt10-kernel/src/libs/ntdll.rs) `windows10_22h2_x64` / [`arch/x86_64/nt_syscall_indices.rs`](../../crates/nt10-kernel/src/arch/x86_64/nt_syscall_indices.rs) (public tables — see [Binary-compat-NT10-scope.md](Binary-compat-NT10-scope.md)).
- **Argument unpack** is configurable: [`arch/x86_64/syscall_abi.rs`](../../crates/nt10-kernel/src/arch/x86_64/syscall_abi.rs) defaults to **LegacyZircon** (`rdi`…`r8` + `r10`, sixth fixed `0`). For NT-style user stubs, call `zr_syscall_x64_unpack_set(Nt10X64)` and pass args per the public x64 convention (`r10`, `rdx`, `r8`, `r9`, stack `+0x28/+0x30` at the `syscall` boundary). **Do not** enable `Nt10X64` until the user stack layout is valid — otherwise the kernel may read garbage.
- Bump [`SYSCALL_NUMBERING_REVISION`](../../crates/nt10-kernel/src/libs/ntdll.rs) when `numbers` or `windows10_22h2_x64` changes.
- `IA32_STAR`, `IA32_FMASK`, and GDT user segments must be programmed together before enabling user callers. Bring-up currently sets **`EFER.SCE`** only after a suitable GDT is installed.

## Policy

- **No embedding** of long excerpts from `references/win32` or MSDN in this file; link externally when a public article is useful.
- Changes that alter calling convention or table layout require a **bump** documented in release notes and, if persisted on disk, in the handoff or loader contract.
