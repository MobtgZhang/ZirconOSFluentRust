#!/usr/bin/env bash
# Verify Phase 3 message-pump serial strings are present in source (offline check).
# Full validation: run scripts/run-qemu-x86_64.sh and watch COM1 for the same phrases.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
need() {
  local f="$1"
  shift
  if ! rg -q "$@" "$f"; then
    echo "verify-phase3: missing pattern in $f: $*" >&2
    exit 1
  fi
}
need crates/nt10-kernel/src/subsystems/win32/csrss_host.rs "Phase3 msg pump smoke begin"
need crates/nt10-kernel/src/subsystems/win32/csrss_host.rs "Phase3 WndProc dispatched"
need crates/nt10-kernel/src/subsystems/win32/csrss_host.rs "Phase3 msg pump smoke OK"
need crates/nt10-kernel/src/subsystems/win32/msg_dispatch.rs "ZR_SYSCALL_POST_MESSAGE"
need crates/nt10-kernel/src/arch/x86_64/syscall.rs "zircon_syscall_from_user"
echo "verify-phase3-serial-keywords: OK (source strings present)"
