# MM: PFN, buddy, and pool bring-up (ZirconOSFluent)

**中文**：[MM-Pool-and-PFN-Bringup.md](../cn/MM-Pool-and-PFN-Bringup.md)（简体镜像）。

This page records **design intent and invariants** for early kernel memory management. It is a clean-room description of **this repository’s** behavior, not a copy of any vendor internals.

## Physical frames (PFN)

- **Source of truth**: UEFI memory map via [`ZirconBootInfo`](../../crates/nt10-boot-protocol/src/lib.rs), validated in [`boot_mem`](../../crates/nt10-kernel/src/mm/boot_mem.rs) and summarized in [`early_map`](../../crates/nt10-kernel/src/mm/early_map.rs).
- **Bring-up allocator**: [`phys`](../../crates/nt10-kernel/src/mm/phys.rs) / [`pfn`](../../crates/nt10-kernel/src/mm/pfn.rs) bump-style allocation for early kernel use.
- **Invariant**: Frames handed to the pool or to user mappings must not overlap the loaded kernel image or firmware-reserved regions described in the validated map.

## Responsibility boundary (PFN ↔ buddy ↔ pool)

| Layer | Role | Caller expectation |
|-------|------|-------------------|
| **PFN / phys** | Owns the sorted manageable frame list after [`pfn_bringup_init`](../../crates/nt10-kernel/src/mm/phys.rs); [`pfn_bringup_alloc`](../../crates/nt10-kernel/src/mm/phys.rs) / [`pfn_bringup_free`](../../crates/nt10-kernel/src/mm/phys.rs) are the low-level 4 KiB page entry points used by pool refill and demand-zero #PF paths. | Higher layers must not free a physical page they did not obtain through the same stack, and must not map frames excluded by `boot_mem`. |
| **Buddy** | Optional coalescing for multi-page blocks when initialized from the same PFN sorted slice; [`alloc_order`](../../crates/nt10-kernel/src/mm/buddy.rs) / [`free_order`](../../crates/nt10-kernel/src/mm/buddy.rs) update PFN metadata. | Not used for single small pool chunks; pool uses direct PFN pages for [`refill_class`](../../crates/nt10-kernel/src/mm/pool.rs). |
| **Pool** | Slab freelists per size class; [`refill_class`](../../crates/nt10-kernel/src/mm/pool.rs) pulls **one** 4 KiB PFN and carves multiple headers+payloads. | If refill fails (PFN starved), allocation returns `null`; the **caller** must surface failure (no automatic buddy fallback in bring-up). |

This is **ZirconOSFluent** layering documentation, not a description of any vendor’s internal structures.

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
- **Feature `mm-pool-stats`**: [`pool_stats_snapshot`](../../crates/nt10-kernel/src/mm/pool.rs) exposes successful alloc/free counts and approximate slab bytes.
- **Feature `mm-pool-tag-hist`**: eight fixed buckets keyed by `tag % 8` via [`pool_tag_buckets_snapshot`](../../crates/nt10-kernel/src/mm/pool.rs) (lightweight tag shape hint, not a full tag map).
- **Debug builds** (`debug_assertions`): [`pool_debug_live_count`](../../crates/nt10-kernel/src/mm/pool.rs) tracks outstanding successful allocs; a free that would drive the count below zero trips `debug_assert!` (suspected double-free or balance bug). Host `cargo test` must not write COM1 — the pool already avoids serial logging in `cfg(test)` paths.

## Optional QEMU / serial check

After capturing guest serial (e.g. `ZBM10_CAPTURE_SERIAL` in [`scripts/run-qemu-x86_64.sh`](../../scripts/run-qemu-x86_64.sh)), run [`scripts/verify-mm-serial-keywords.sh`](../../scripts/verify-mm-serial-keywords.sh) to grep `[ZFOS][MM]` lines.

## Future work (P0 backlog)

- Richer per-tag tracking (beyond `tag % 8`) once release binary size impact is bounded.
- Full **NUMA** awareness is explicitly out of scope for this bring-up document.
- **Kernel relocate** (full image) remains a separate milestone; see [Kernel-Relocate-Phases.md](Kernel-Relocate-Phases.md) and [Roadmap-and-TODO.md](Roadmap-and-TODO.md) Phase 3.

See also: [Roadmap-and-TODO.md](Roadmap-and-TODO.md) Phase 1.
