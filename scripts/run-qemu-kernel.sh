#!/usr/bin/env bash
# Smoke-test the low-address kernel ELF with QEMU -kernel (serial on stdio).
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: run-qemu-kernel.sh [qemu-system-x86_64 options...]

  Run the low-address kernel ELF via QEMU -kernel (serial on stdio, no display).

Environment:
  KERNEL  Path to nt10-kernel-bin (default: <repo>/target/x86_64-unknown-none/debug/nt10-kernel-bin)

Prerequisites:
  qemu-system-x86_64
  cargo build -p nt10-kernel-bin --target x86_64-unknown-none

Options:
  -h, --help  Show this help and exit.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

KERNEL="${KERNEL:-$(dirname "$0")/../target/x86_64-unknown-none/debug/nt10-kernel-bin}"

if [[ ! -f "$KERNEL" ]]; then
  echo "Build first: cargo build -p nt10-kernel-bin --target x86_64-unknown-none" >&2
  exit 1
fi

exec qemu-system-x86_64 \
  -machine q35 \
  -cpu qemu64 \
  -m 128M \
  -serial stdio \
  -display none \
  -kernel "$KERNEL" \
  "$@"
