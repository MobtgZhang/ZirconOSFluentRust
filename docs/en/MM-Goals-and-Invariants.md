# Memory manager — goals and invariants (ZirconOSFluent)

**中文**：[../cn/MM-Goals-and-Invariants.md](../cn/MM-Goals-and-Invariants.md)

This document is **project-local**. It does not describe Windows internals. Implementation follows public CPU manuals (Intel SDM / AMD APM), UEFI memory descriptors where relevant, PE/COFF public layout for images, and this repository’s tests.

## Goals

- **Per address space**: one x86_64 4-level page-table tree rooted at `CR3` (bring-up may share one user `CR3` across processes until split).
- **VAD ↔ PTE consistency**: a committed user mapping must have a matching [`VadEntry`](../../crates/nt10-kernel/src/mm/vad.rs); demand-zero and file-backed paths install PTEs only after VAD checks.
- **PFN lifetime**: frames come from the buddy pool ([`buddy.rs`](../../crates/nt10-kernel/src/mm/buddy.rs), [`phys.rs`](../../crates/nt10-kernel/src/mm/phys.rs)); reference counts in [`pfn.rs`](../../crates/nt10-kernel/src/mm/pfn.rs) back shared/COW paths.
- **TLB coherence**: after PTE changes, use [`flush_after_pte_change`](../../crates/nt10-kernel/src/arch/x86_64/tlb.rs) or `shootdown_range_all_cpus` when another CPU or a different `CR3` may cache the VA (see comments in [`pt.rs`](../../crates/nt10-kernel/src/mm/pt.rs)).

## Invariants (bring-up)

1. **Buddy**: alloc/free `order` pairs; only frames returned by `pfn_bringup_alloc` / `alloc_order` are freed with matching `free_order`.
2. **Boot handoff**: [`boot_mem`](../../crates/nt10-kernel/src/mm/boot_mem.rs) usable ranges exclude the loaded kernel; those PFNs never enter the free pool.
3. **User canonical VA**: user `#PF` and syscall pointer checks use `< 0x8000_0000_0000` (47-bit canonical lower half) via [`user_va`](../../crates/nt10-kernel/src/mm/user_va.rs).
4. **Section teardown**: drop VAD entries that reference a [`SectionObject`](../../crates/nt10-kernel/src/mm/section.rs) before `release` drives PFN teardown (see `section.rs` module docs).
5. **Anonymous cap**: [`SECTION_ANONYMOUS_PAGE_CAP`](../../crates/nt10-kernel/src/mm/section.rs) is a bring-up limit; exceeding it yields [`SectionCommitError::AnonymousCapExceeded`](../../crates/nt10-kernel/src/mm/section.rs) (not silent truncation).

## Deferred / explicit non-goals (today)

- **NUMA**: multi-node placement is not implemented; see [`numa.rs`](../../crates/nt10-kernel/src/mm/numa.rs) and [`NumaBackend`](../../crates/nt10-kernel/src/mm/numa.rs).
- **Page file**: swapping to disk is [`Unsupported`](../../crates/nt10-kernel/src/mm/pagefile.rs) unless a backend is added later.
- **2 MiB / 1 GiB user mappings**: [`large_page.rs`](../../crates/nt10-kernel/src/mm/large_page.rs) documents the migration path; default remains 4 KiB.

## Related

- [Clean-Room-Implementation.md](Clean-Room-Implementation.md)
- [Public-References.md](Public-References.md)
- [Roadmap-and-TODO.md](Roadmap-and-TODO.md)
