# Kernel image relocate — phased milestone (clean-room)

**中文**：[Kernel-Relocate-Phases.md](../cn/Kernel-Relocate-Phases.md)

Full **kernel virtual relocate** (moving the running image to a new VA range) is **not** implemented in bring-up. This page splits future work into **independent verify-able phases**. Rationale and public references only — no Windows-internal layout as authority.

## Phase R1 — Parse and bound the loaded image

- Determine the PE/ELF (or loader-specific) **loaded range** and section permissions from **this** loader’s data structures.
- **Verify:** `cargo check -p nt10-kernel --target x86_64-unknown-none` with parsing code behind a feature if needed; no runtime CR3/`rip` change.

## Phase R2 — Relocation table validation (read-only)

- Read relocation records; **validate** consistency against R1 (ranges, alignment, no overlap with firmware reservations from the validated memory map).
- **Verify:** unit tests on host with **synthetic** tables; still no change to the running PC.

## Phase R3 — Bootstrap page-table adjustments

- Build or edit early mappings so the **target** VA range is mapped identically to the current one (temporary mirror or staged switch), without jumping yet.
- **Verify:** QEMU boot to a known checkpoint with logging; optional serial keyword script.

## Phase R4 — Integrate with MM / PFN

- Ensure PFN/buddy/pool **exclude** both old and new image footprints during transition; cooperate with [`boot_mem`](../../crates/nt10-kernel/src/mm/boot_mem.rs) invariants.
- **Verify:** no double-mapped writable aliases unless intentionally transient and documented.

## Phase R5 — Actual `rip` / stack migration (last)

- Perform the controlled jump to the new VA only after R1–R4 pass.
- **Verify:** single-path QEMU run; rollback plan documented.

See also: [Roadmap-and-TODO.md](Roadmap-and-TODO.md) Phase 3, [MM-Pool-and-PFN-Bringup.md](MM-Pool-and-PFN-Bringup.md).
