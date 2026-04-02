#!/usr/bin/env bash
# Verify Phase 6 scaffold strings are present in source (offline check).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
need() {
  local f="$1"
  shift
  if ! rg -q "$@" "$f"; then
    echo "verify-phase6: missing pattern in $f: $*" >&2
    exit 1
  fi
}
need crates/nt10-kernel/src/alpc/cross_proc.rs "post_cross_address_space"
need crates/nt10-kernel/src/alpc/cross_proc.rs "CROSS_AS_BOUNCE"
need crates/nt10-kernel/src/alpc/phase6_csrss.rs "ZR_ALPC_CSRSS_API_PORT_UTF8"
need crates/nt10-kernel/src/loader/import_.rs "resolve_imports_for_image_stub"
need crates/nt10-kernel/src/mm/vm.rs "ProcessAddressSpaceBringup"
need crates/nt10-kernel/src/servers/smss.rs "NT10_PHASE6_RING3_CSRSS_FALLBACK_TO_KERNEL_HOST"
need crates/nt10-kernel/src/servers/smss.rs "try_launch_ring3_smss_from_vfs"
need crates/nt10-kernel/src/subsystems/win32/csrss_host.rs "PHASE6_CSRSS_OWNS_WINSTA_IN_RING3"
echo "verify-phase6-serial-keywords: OK (source strings present)"
