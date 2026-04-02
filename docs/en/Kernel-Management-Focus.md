# Kernel-management focus (deferred scope)

**Status**: policy note — not a code deliverable.

When the project is in a **kernel-management-first** phase, the following remain **out of scope** except for maintenance fixes that block boot or CI:

- Fluent desktop / DWM-style expansion (Roadmap Phase 14 depth).
- WinRT/UWP stub expansion (Phase 15 depth).
- New Win32 shell integration features beyond what already exists for bring-up.
- Real `csrss.exe` / SMSS user-process bring-up and full user-buffer ALPC contracts.

Work should prioritize MM, KE (scheduler, IRQL/DPC), OB, PS, SMP/TLB, and SE integration paths. Revisit this list when product priorities change.

**中文**：[Kernel-Management-Focus.md](../cn/Kernel-Management-Focus.md)
