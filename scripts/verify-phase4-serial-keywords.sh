#!/usr/bin/env bash
# Verify Phase 4 compositor / syscall-demo strings are present in source (offline check).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
need() {
  local f="$1"
  shift
  if ! rg -q "$@" "$f"; then
    echo "verify-phase4: missing pattern in $f: $*" >&2
    exit 1
  fi
}
need crates/nt10-kernel/src/subsystems/win32/csrss_host.rs "Phase4 compositor smoke begin"
need crates/nt10-kernel/src/subsystems/win32/csrss_host.rs "Phase4 compositor smoke OK"
need crates/nt10-kernel/src/mm/bringup_user.rs "USER_RING3_GETMESSAGE_SYSCALL_DEMO"
need crates/nt10-kernel/src/subsystems/win32/msg_dispatch.rs "ZR_SYSCALL_SEND_MESSAGE"
need crates/nt10-kernel/src/subsystems/win32/compositor.rs "composite_desktop_to_framebuffer"
need crates/nt10-kernel/src/desktop/fluent/session_win32.rs "nt10-phase4: GOP_COMPOSITE"
need crates/nt10-kernel/src/desktop/fluent/session_win32.rs "nt10-phase4: WM_LBUTTONDOWN"
need crates/nt10-kernel/src/subsystems/win32/win32_paint.rs "BringupPaintStruct"
need crates/nt10-kernel/src/subsystems/win32/window_surface.rs "blend_src_over_bgra"
need crates/nt10-kernel/src/ob/winsta.rs "hit_test_screen_topmost"
echo "verify-phase4-serial-keywords: OK (source strings present)"
