#!/usr/bin/env bash
# Offline markers for MM plan (invariants doc, VAD clear, pressure hooks).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
need() {
  local f="$1"
  shift
  if ! rg -q "$@" "$f"; then
    echo "verify-mm-keywords: missing pattern in $f: $*" >&2
    exit 1
  fi
}
need docs/en/MM-Goals-and-Invariants.md "SECTION_ANONYMOUS_PAGE_CAP"
need crates/nt10-kernel/src/mm/vad.rs "pub fn clear"
need crates/nt10-kernel/src/mm/user_va.rs "user_pointer_canonical"
need crates/nt10-kernel/src/mm/phys.rs "pfn_pool_starved_flag"
need crates/nt10-kernel/src/mm/working_set.rs "WorkingSetBringup"
need crates/nt10-kernel/src/mm/pagefile.rs "PageFileBackend"
need crates/nt10-kernel/src/mm/pagefile.rs "PageFileIoError"
need crates/nt10-kernel/src/mm/pagefile.rs "stub_pagefile_issue_read_irp"
need crates/nt10-kernel/src/mm/large_page.rs "LargePagePolicy"
echo "verify-mm-keywords: OK (source strings present)"
