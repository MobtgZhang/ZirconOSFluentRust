#!/usr/bin/env bash
# Verify Phase 5 Win32 shell / UEFI overlay strings are present in source (offline check).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
need() {
  local f="$1"
  shift
  if ! rg -q "$@" "$f"; then
    echo "verify-phase5: missing pattern in $f: $*" >&2
    exit 1
  fi
}
need crates/nt10-kernel/src/desktop/fluent/session_win32.rs "nt10-phase5: CLOCK_POP"
need crates/nt10-kernel/src/desktop/fluent/session_win32.rs "nt10-phase5: SW_MINIMIZE"
need crates/nt10-kernel/src/desktop/fluent/session_win32.rs "nt10-phase5: SW_RESTORE"
need crates/nt10-kernel/src/desktop/fluent/session_win32.rs "nt10-phase5: MENU_CMD"
need crates/nt10-kernel/src/desktop/fluent/session_win32.rs "nt10-phase5: CLOCK_FLYOUT"
need crates/nt10-kernel/src/desktop/fluent/session_win32.rs "ZR_WM_MENU_COMMAND"
need crates/nt10-kernel/src/subsystems/win32/compositor.rs "CompositeDesktopFilter"
need crates/nt10-kernel/src/ob/winsta.rs "WS_EX_TOOLWINDOW"
need crates/nt10-kernel/src/subsystems/win32/windowing.rs "WM_NCHITTEST"
need crates/nt10-kernel/src/subsystems/win32/gdi32.rs "bringup_select_solid_brush"
echo "verify-phase5-serial-keywords: OK (source strings present)"
