# MM: PFN, buddy, and pool bring-up (ZirconOSFluent)

**中文**：[MM-Pool-and-PFN-Bringup.md](../cn/MM-Pool-and-PFN-Bringup.md)（简体镜像）。

This page records **design intent and invariants** for early kernel memory management. It is a clean-room description of **this repository’s** behavior, not a copy of any vendor internals.

## Physical frames (PFN)

- **Source of truth**: UEFI memory map via [`ZirconBootInfo`](../../crates/nt10-boot-protocol/src/lib.rs), validated in [`boot_mem`](../../crates/nt10-kernel/src/mm/boot_mem.rs) and summarized in [`early_map`](../../crates/nt10-kernel/src/mm/early_map.rs).
- **Bring-up allocator**: [`phys`](../../crates/nt10-kernel/src/mm/phys.rs) / [`pfn`](../../crates/nt10-kernel/src/mm/pfn.rs) bump-style allocation for early kernel use.
- **Invariant**: Frames handed to the pool or to user mappings must not overlap the loaded kernel image or firmware-reserved regions described in the validated map.

## Buddy / coalescing (when enabled)

- [`buddy`](../../crates/nt10-kernel/src/mm/buddy.rs) complements the PFN layer for larger or reusable blocks.
- **Invariant**: Any page returned to the buddy must match the same order it was allocated with (no double-free; tag or magic checks are future hardening).

## Slab pool (`pool.rs`)

- **Shape**: Power-of-two **total** chunk sizes (header + payload), with an 8-byte header storing a **caller-supplied tag** and class index (see [`ex_allocate_pool_with_tag`](../../crates/nt10-kernel/src/mm/pool.rs)).
- **Refill**: [`refill_class`](../../crates/nt10-kernel/src/mm/pool.rs) carves multiple chunks from a single 4 KiB PFN when possible.
- **Invariants**:
  1. `ex_free_pool_with_tag` must receive the **user** pointer returned by allocate, with the **same tag**, or the free is ignored (defensive); tag mismatch emits `[ZFOS][MM]` on serial (x86_64).
  2. `POOL_BYTES` tracks PFN bytes pulled into the pool for coarse telemetry only; it is not yet a leak detector.
  3. Large contiguous allocations should prefer PFN-page slabs or section paths, not the small-chunk classes.
- **Telemetry**: [`pool_alloc_fail_count`](../../crates/nt10-kernel/src/mm/pool.rs) counts failed `ex_allocate_pool_with_tag` attempts; failures log `pool_alloc_req_bytes=` / `pool_alloc_class_idx=` on COM1.

## Future work (P0 backlog)

- Per-tag **histograms** and optional **leak audit** (debug builds) recording outstanding `(tag, size)` pairs.
- Full **NUMA** awareness is explicitly out of scope for this bring-up document.
- **Kernel relocate** (full image) remains a separate milestone; see [Roadmap-and-TODO.md](Roadmap-and-TODO.md) Phase 3.

See also: [Roadmap-and-TODO.md](Roadmap-and-TODO.md) Phase 1.
