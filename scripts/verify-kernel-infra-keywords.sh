#!/usr/bin/env bash
# Offline check: kernel infrastructure serial / source markers from the infra bring-up plan.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
need() {
  local f="$1"
  shift
  if ! rg -q "$@" "$f"; then
    echo "verify-kernel-infra: missing pattern in $f: $*" >&2
    exit 1
  fi
}
need crates/nt10-kernel/src/kmain.rs "kmain_phase_begin"
need crates/nt10-kernel/src/infra_bringup.rs "handoff_invalid_abort_magic_version"
need crates/nt10-kernel/src/mm/page_fault.rs "try_dispatch_page_fault_for_vad"
need crates/nt10-kernel/src/io/iomgr.rs "io_read_ramdisk_complete_irp"
need crates/nt10-kernel/src/servers/smss.rs "smss_run_documented_phase_chain"
need crates/nt10-kernel/src/milestones.rs "PHASE_KERNEL_INFRA"
echo "verify-kernel-infra-keywords: OK (source strings present)"
