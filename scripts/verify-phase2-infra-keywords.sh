#!/usr/bin/env bash
# Offline markers for phase-2 infra (VirtIO MMIO, TLB flush helper, syscall probe).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
need() {
  local f="$1"
  shift
  if ! rg -q "$@" "$f"; then
    echo "verify-phase2-infra: missing pattern in $f: $*" >&2
    exit 1
  fi
}
need crates/nt10-kernel/src/drivers/storage/virtio_blk.rs "VirtioBlkMmioBringup"
need crates/nt10-kernel/src/io/iomgr.rs "io_read_block_volume_complete_irp"
need crates/nt10-kernel/src/arch/x86_64/tlb.rs "flush_after_pte_change"
need crates/nt10-kernel/src/arch/x86_64/syscall.rs "ZR_USER_VA_PROBE_OK"
need crates/nt10-kernel/src/milestones.rs "PHASE_VIRTIO_MMIO_BLK"
echo "verify-phase2-infra-keywords: OK (source strings present)"
