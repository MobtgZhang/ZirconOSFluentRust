# ZirconOS syscall ABI (x86_64)

**中文**：[../cn/Syscall-ABI-ZirconOS.md](../cn/Syscall-ABI-ZirconOS.md)

This document describes how the **ZirconOS NT10** kernel exposes system services on x86_64. It is **project-specific**: do not treat it as Microsoft documentation.

## Goals

- **Stable hand-written ABI** between [`nt10-kernel`](../../crates/nt10-kernel/src/lib.rs) and future user-mode stubs ([`libs/ntdll`](../../crates/nt10-kernel/src/libs/ntdll.rs), etc.).
- **Behavioral compatibility** with Windows NT 10-class systems where practical, using **public facts** (register convention, syscall instruction) rather than pasting copyrighted reference text.

## Mechanism

- User mode invokes `syscall`; the CPU transfers to the address in `IA32_LSTAR` with a documented register convention (see Intel SDM / AMD APM for the instruction, not reproduced here).
- The kernel maintains a **dispatch table** ([`arch/x86_64/syscall.rs`](../../crates/nt10-kernel/src/arch/x86_64/syscall.rs)); indices are **ZirconOS syscall numbers**. The **authoritative user-side number list** is the `numbers` module plus `SYSCALL_NUMBERING_REVISION` in [`libs/ntdll.rs`](../../crates/nt10-kernel/src/libs/ntdll.rs); bump the revision whenever those constants change.
- `IA32_STAR`, `IA32_FMASK`, and GDT user segments must be programmed together before enabling user callers. Bring-up currently sets **`EFER.SCE`** only after a suitable GDT is installed.

## Policy

- **No embedding** of long excerpts from `references/win32` or MSDN in this file; link externally when a public article is useful.
- Changes that alter calling convention or table layout require a **bump** documented in release notes and, if persisted on disk, in the handoff or loader contract.
