#!/usr/bin/env bash
# Verify Phase 1 UEFI Ring-3 serial strings are present in source (offline check).
# Full validation: run scripts/run-qemu-x86_64.sh and watch COM1 for the same phrases.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
need() {
  local f="$1"
  shift
  if ! rg -q "$@" "$f"; then
    echo "verify-phase1: missing pattern in $f: $*" >&2
    exit 1
  fi
}
need crates/nt10-kernel/src/kmain.rs "PML4\[256\] high-half 512MiB mirror"
need crates/nt10-kernel/src/kmain.rs "UEFI user thread starting"
need crates/nt10-kernel/src/mm/page_fault.rs "demand-zero #PF handled"
need crates/nt10-kernel/src/arch/x86_64/syscall.rs "user syscall smoke"
need crates/nt10-kernel/src/arch/x86_64/syscall.rs "syscall num"
echo "verify-phase1-serial-keywords: OK (source strings present)"
