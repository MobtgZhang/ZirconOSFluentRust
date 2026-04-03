# SMP, TLB shootdown, and QEMU (MM linkage)

**中文**：[SMP-MM-TLB-QEMU.md](../cn/SMP-MM-TLB-QEMU.md)

Clean-room notes for how **memory management** ties to **TLB invalidation** when more than one logical CPU is online.

## Same IDT on BSP and APs

Before calling [`smp_set_online_cpu_count`](../../crates/nt10-kernel/src/arch/x86_64/tlb.rs) with `n > 1`, every application processor must install the **same IDT** as the BSP, including the gate for [`TLB_FLUSH_IPI_VECTOR`](../../crates/nt10-kernel/src/arch/x86_64/tlb.rs) (`0xFD`). Otherwise remote `invlpg` via IPI is undefined on APs and PTE edits can leave stale TLB entries.

## MM contract

After any successful [`map_4k`](../../crates/nt10-kernel/src/mm/pt.rs) / [`unmap_4k`](../../crates/nt10-kernel/src/mm/pt.rs) that changes a user-visible mapping, call [`flush_after_pte_change`](../../crates/nt10-kernel/src/arch/x86_64/tlb.rs) (or equivalent range shootdown). Under SMP, this may escalate to [`shootdown_range_all_cpus`](../../crates/nt10-kernel/src/arch/x86_64/tlb.rs).

## QEMU `-smp`

- Example: `qemu-system-x86_64 -smp 2 ...` (plus your usual OVMF/ESP flags from [`scripts/run-qemu-x86_64.sh`](../../scripts/run-qemu-x86_64.sh)).
- **Bring-up caveat:** this repo does not assume AP bring-up is complete in the default boot path; treat multi-core as an **optional experiment** until AP startup and IDT handoff are wired.
- Expect serial ordering to differ under SMP; use [`ZBM10_CAPTURE_SERIAL`](../../scripts/run-qemu-x86_64.sh) and [`scripts/verify-mm-serial-keywords.sh`](../../scripts/verify-mm-serial-keywords.sh) only as an optional MM marker check, not as proof of IPI delivery unless AP code is enabled.

## Host `cargo test` and `invlpg`

[`shootdown_bringup_tests`](../../crates/nt10-kernel/src/arch/x86_64/tlb.rs) are marked `#[ignore]` because `invlpg` and LAPIC IPI paths are ring-0–only. Run them under a **kernel harness** or QEMU-backed test when such a runner exists; do not un-ignore for default host `cargo test`.

See also: [MM-Goals-and-Invariants.md](MM-Goals-and-Invariants.md), [Roadmap-and-TODO.md](Roadmap-and-TODO.md) Phase 3 / SMP notes.
