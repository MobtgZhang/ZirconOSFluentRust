# Public references for implementation (short list)

Use these **only** as behavioral or structural hints. Implement in original code; do not paste copyrighted prose or sample code from vendors into this tree. See [Clean-Room-Implementation.md](Clean-Room-Implementation.md) and [References-Policy.md](References-Policy.md).

| Topic | Suggested public basis |
|-------|-------------------------|
| UEFI memory types / handoff | UEFI specification (memory descriptor types). |
| PE/COFF / DLL layout | Microsoft PE Format (documented), ECMA-335 where relevant. |
| x86_64 paging / MSRs | Intel SDM (paging, `SYSCALL`, `#PF` error code bits). |
| VirtIO block / MMIO | VirtIO 1.x specification (device layout, queues). |
| FAT32 | Public BPB/FAT layout descriptions; field semantics only. |
| Win32 API names (optional surface) | Microsoft Learn function signatures and documented behavior only. |
| NT x64 syscall **indices** (build rows) | Third-party **factual** tables (e.g. j00ru [windows-syscalls](https://github.com/j00ru/windows-syscalls)); cite build key (e.g. Windows 10 22H2), do not paste large excerpts into source. |
| x86_64 paging / `#PF` | Intel SDM (4-level paging, page-fault error code, `INVLPG`, NX bit); AMD APM for cross-check. |
| PE image sections / characteristics | Microsoft PE Format (documented COFF section flags, NX-compatible). |
| MM architecture (project) | [MM-Goals-and-Invariants.md](MM-Goals-and-Invariants.md) — ZirconOSFluent naming only. |

**中文**：[../cn/Public-References.md](../cn/Public-References.md)
