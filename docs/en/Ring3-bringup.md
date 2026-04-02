# Ring-3 user-mode bring-up plan (ZirconOSFluent)

**See also**: [Boot-paths.md](Boot-paths.md). Today, syscall smoke runs only on `-kernel` with built-in CR3.

## Intended order

1. **Link/load**: ship a static PIE or bare ELF with a fixed layout; map from VFS into user VA (grow beyond [`USER_LARGE_ARENA_HINT`](../../crates/nt10-kernel/src/mm/user_va.rs)).
2. **User ntdll**: thin `syscall` wrappers matching `numbers` in [`libs/ntdll.rs`](../../crates/nt10-kernel/src/libs/ntdll.rs).
3. **Minimal shell**: replace kernel-only [`shell_bringup`](../../crates/nt10-kernel/src/subsystems/win32/shell_bringup.rs) with a line reader + builtins.

## Compliance

Implement from public PE/syscall docs and [Syscall-ABI-ZirconOS.md](Syscall-ABI-ZirconOS.md) only; do not copy Windows user-mode sources.
