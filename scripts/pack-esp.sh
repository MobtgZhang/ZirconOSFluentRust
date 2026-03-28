#!/usr/bin/env bash
# Build ZBM10 + flat kernel binary and populate a FAT ESP tree:
#   EFI/BOOT/BOOTX64.EFI
#   EFI/ZirconOS/NT10KRNL.BIN
#
# Usage: pack-esp.sh <esp-root-dir>
# Requires: cargo, and one of rust-objcopy | llvm-objcopy | objcopy (for .BIN).
# Optional: PROFILE=release for release artifacts (same as cargo --release).
set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
DEST=${1:?destination ESP directory (e.g. mktemp -d/esp)}
PROFILE="${PROFILE:-debug}"

if [[ ! -d "$DEST" ]]; then
  echo "Destination must be an existing directory: $DEST" >&2
  exit 1
fi

pick_objcopy() {
  for c in rust-objcopy llvm-objcopy objcopy; do
    if command -v "$c" &>/dev/null; then
      echo "$c"
      return
    fi
  done
  echo "" 
}

OBJCOPY=$(pick_objcopy)
if [[ -z "$OBJCOPY" ]]; then
  echo "No rust-objcopy/llvm-objcopy/objcopy found. Install llvm-tools (rustup component) or binutils." >&2
  exit 1
fi

if [[ "$PROFILE" == "release" ]]; then
  ( cd "$ROOT" && cargo build -p nt10-boot-uefi --target x86_64-unknown-uefi --release )
  ( cd "$ROOT" && cargo build -p nt10-kernel-bin --target x86_64-unknown-none --release )
  TGT_SUB=release
else
  ( cd "$ROOT" && cargo build -p nt10-boot-uefi --target x86_64-unknown-uefi )
  ( cd "$ROOT" && cargo build -p nt10-kernel-bin --target x86_64-unknown-none )
  TGT_SUB=debug
fi

EFI_SRC="$ROOT/target/x86_64-unknown-uefi/$TGT_SUB/zbm10.efi"
KRNL_ELF="$ROOT/target/x86_64-unknown-none/$TGT_SUB/nt10-kernel-bin"
if [[ ! -f "$EFI_SRC" ]]; then
  EFI_SRC="$ROOT/target/x86_64-unknown-uefi/$TGT_SUB/zbm10"
fi

mkdir -p "$DEST/EFI/BOOT" "$DEST/EFI/ZirconOS"
cp -f "$EFI_SRC" "$DEST/EFI/BOOT/BOOTX64.EFI"

"$OBJCOPY" -O binary "$KRNL_ELF" "$DEST/EFI/ZirconOS/NT10KRNL.BIN"

# OVMF often fails Bds default boot on QEMU's vvfat HDD ("Unsupported"); Internal Shell runs startup.nsh from the volume root after the countdown.
cat >"$DEST/startup.nsh" <<'NSH'
@echo -off
fs0:
\EFI\BOOT\BOOTX64.EFI
NSH

echo "ESP ready: $DEST"
echo "  BOOTX64.EFI <- nt10-boot-uefi"
echo "  NT10KRNL.BIN <- nt10-kernel-bin (flat, load 0x8000000)"
