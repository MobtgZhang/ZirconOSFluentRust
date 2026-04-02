# Boot paths and ring-3 smoke (ZirconOSFluent / NT10)

**中文**：[../cn/Boot-paths.md](../cn/Boot-paths.md)

## Two primary paths

| Path | Page tables / CR3 | HW timer + `sti` | Ring-3 smoke |
|------|-------------------|------------------|--------------|
| **QEMU `-kernel`** (`nt10-kernel-bin`) | Built-in PML4 ([`paging::init_low_identity`](../../crates/nt10-kernel/src/arch/x86_64/paging.rs)) | PIC/LAPIC bring-up enabled | **Runs** [`user_enter`](../../crates/nt10-kernel/src/arch/x86_64/user_enter.rs) |
| **UEFI → ZBM10 → NT10KRNL.BIN** | Firmware paging; **no** built-in CR3 | **Skipped** reprogramming PIT/LAPIC + `sti` (OVMF IRQ clash) | **Skipped** (see [`kmain`](../../crates/nt10-kernel/src/kmain.rs)) |

## Rationale

Under OVMF, the firmware already owns IOAPIC/LAPIC virtual-wire delivery. Re-arming PIC or enabling interrupts from the kernel often causes spurious vectors or triple faults (“reboot loop”). When a valid handoff exists and built-in page tables are **not** used, only a sample KAPC is queued.

## Convergence options

- Implement a firmware-coordinated LAPIC timer (or TSC deadline) with a documented EOI contract, then re-enable user smoke on UEFI; or
- Keep “syscall smoke only on `-kernel`” as the supported matrix—this document is the source of truth.

Kernel binary on the ESP: `EFI/ZirconOSFluent/NT10KRNL.BIN` ([`pack-esp.sh`](../../scripts/pack-esp.sh)).
