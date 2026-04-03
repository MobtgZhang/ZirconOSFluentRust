#!/usr/bin/env bash
# Optional MM serial regression: grep captured guest serial for [ZFOS][MM] markers.
# Clean-room: uses only this repo's log format (see crates/nt10-kernel/src/rtl/log.rs).
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: verify-mm-serial-keywords.sh [--require] <serial-log-file>

  Without --require: print matching lines (if any) and exit 0.
  With --require: exit 1 if no line matches \[ZFOS\]\[MM\] (strict gate for CI or local runs).

Example:
  ZBM10_CAPTURE_SERIAL=/tmp/zbm10-serial.log bash scripts/run-qemu-x86_64.sh
  bash scripts/verify-mm-serial-keywords.sh /tmp/zbm10-serial.log
EOF
}

require=0
if [[ "${1:-}" == "--require" ]]; then
  require=1
  shift
fi

if [[ $# -lt 1 ]]; then
  usage >&2
  exit 2
fi

logfile=$1
if [[ ! -f "$logfile" ]]; then
  echo "verify-mm-serial-keywords: no such file: $logfile" >&2
  exit 2
fi

if grep -E '\[ZFOS\]\[MM\]' "$logfile"; then
  echo "verify-mm-serial-keywords: found MM marker(s) in $logfile"
  exit 0
fi

echo "verify-mm-serial-keywords: no [ZFOS][MM] lines in $logfile" >&2
if [[ "$require" -eq 1 ]]; then
  exit 1
fi
exit 0
