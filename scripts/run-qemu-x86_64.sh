#!/usr/bin/env bash
# Run ZBM10 UEFI app under QEMU with OVMF (x86_64).
# ZBM10 shows a blue GOP tile menu when a framebuffer is available; use usb-mouse/usb-kbd (below)
# to exercise pointer input. With `-display none` or missing GOP, it falls back to a text ConOut menu.
# Prerequisites: qemu-system-x86_64, OVMF.fd (monolithic) or OVMF_CODE.fd + OVMF_VARS.fd (split).
#
# Override with env: OVMF_CODE, OVMF_VARS, ZBM10_EFI
# Auto-detect searches (in order): /usr/share/OVMF, /usr/share/ovmf, /usr/share/edk2/ovmf
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: run-qemu-x86_64.sh [qemu-system-x86_64 options...]

  Build a temporary ESP (BOOTX64.EFI + EFI/ZirconOSFluent/NT10KRNL.BIN) unless ZBM10_ESP is set, then run QEMU + OVMF (q35).

Environment:
  OVMF_CODE   Firmware image. Auto-detected if unset.
  OVMF_VARS   Split build only: writable vars flash (OVMF_VARS.fd next to CODE).
  ZBM10_EFI   Path to ZBM10 PE (default: zbm10.efi under debug/ or release/; falls back to `zbm10` if no .efi)
  ZBM10_ESP   If set, use this directory as the FAT ESP root (must contain EFI/BOOT/BOOTX64.EFI).
              When unset, a temp ESP is built via scripts/pack-esp.sh (kernel at EFI/ZirconOSFluent/NT10KRNL.BIN).
  PROFILE     Passed to pack-esp.sh when building temp ESP: `release` or `debug` (default).
  ZBM10_NO_REBOOT  If non-empty, pass QEMU `-no-reboot` so a guest triple fault exits instead of resetting.

  Monolithic (e.g. /usr/share/ovmf/OVMF.fd): use -bios only; do not pair OVMF_VARS.fd.
  Split (e.g. OVMF_CODE.fd + OVMF_VARS.fd): two pflash drives; VARS is auto-picked only when
  OVMF_CODE filename contains "code" (case-insensitive), e.g. *CODE*.fd or *-code.fd.

  When OVMF_CODE is unset, searches each directory for OVMF_CODE.fd, then OVMF.fd:
    /usr/share/OVMF, /usr/share/ovmf, /usr/share/edk2/ovmf

Prerequisites:
  qemu-system-x86_64, OVMF (e.g. apt install ovmf)
  cargo build -p nt10-boot-uefi --target x86_64-unknown-uefi
  cargo build -p nt10-kernel-bin --target x86_64-unknown-none
  rust-objcopy/llvm-objcopy/objcopy (for pack-esp.sh when ZBM10_ESP is unset)

Options:
  -h, --help  Show this help and exit.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

ZBM10_ROOT=$(dirname "$0")/..
if [[ -z "${ZBM10_EFI:-}" ]]; then
  _zbm_dbg="$ZBM10_ROOT/target/x86_64-unknown-uefi/debug"
  if [[ -f "$_zbm_dbg/zbm10.efi" ]]; then
    ZBM10_EFI="$_zbm_dbg/zbm10.efi"
  else
    ZBM10_EFI="$_zbm_dbg/zbm10"
  fi
fi

_ovmf_search_dirs=(/usr/share/OVMF /usr/share/ovmf /usr/share/edk2/ovmf)

# Bash 4.2+: -v is true if variable is set (including empty).
if [[ ! -v OVMF_CODE ]]; then
  OVMF_CODE=""
  for _dir in "${_ovmf_search_dirs[@]}"; do
    if [[ -f "$_dir/OVMF_CODE.fd" ]]; then
      OVMF_CODE="$_dir/OVMF_CODE.fd"
      break
    fi
  done
  if [[ -z "$OVMF_CODE" ]]; then
    for _dir in "${_ovmf_search_dirs[@]}"; do
      if [[ -f "$_dir/OVMF.fd" ]]; then
        OVMF_CODE="$_dir/OVMF.fd"
        break
      fi
    done
  fi
fi

# Split CODE/VARS: only pair OVMF_VARS when CODE is a "code" region image, not monolithic OVMF.fd.
_ovmf_code_lc=""
if [[ -n "$OVMF_CODE" ]]; then
  _bn=$(basename "$OVMF_CODE")
  _ovmf_code_lc=${_bn,,}
fi

is_split_ovmf_code() {
  [[ -n "$_ovmf_code_lc" && "$_ovmf_code_lc" == *code*.fd ]]
}

if [[ ! -v OVMF_VARS ]]; then
  OVMF_VARS=""
  if [[ -n "$OVMF_CODE" ]] && is_split_ovmf_code; then
    _ovmf_dir=$(dirname "$OVMF_CODE")
    if [[ -f "$_ovmf_dir/OVMF_VARS.fd" ]]; then
      OVMF_VARS="$_ovmf_dir/OVMF_VARS.fd"
    fi
  fi
fi

if [[ ! -f "$OVMF_CODE" ]]; then
  echo "Missing OVMF_CODE (tried split OVMF_CODE.fd then monolithic OVMF.fd under:" >&2
  echo "  ${_ovmf_search_dirs[*]}" >&2
  echo "Set OVMF_CODE, e.g. export OVMF_CODE=/usr/share/ovmf/OVMF.fd" >&2
  echo "Debian/Ubuntu: apt install ovmf" >&2
  exit 1
fi

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

if [[ -n "${ZBM10_ESP:-}" ]]; then
  ESP="$ZBM10_ESP"
else
  ESP="$WORK/esp"
  mkdir -p "$ESP"
  "$ZBM10_ROOT/scripts/pack-esp.sh" "$ESP"
fi

QEMU_ARGS=(
  -machine q35
  -m 256M
  -serial stdio
  -net none
  -device nec-usb-xhci
  -device usb-kbd
  -device usb-mouse
)
if [[ -n "${ZBM10_NO_REBOOT:-}" ]]; then
  QEMU_ARGS+=(-no-reboot)
fi

# Dual pflash only for split EDK2 images; monolithic OVMF.fd must use -bios.
if [[ -f "$OVMF_VARS" ]] && is_split_ovmf_code; then
  QEMU_ARGS+=(-drive "if=pflash,format=raw,readonly=on,file=$OVMF_CODE")
  QEMU_ARGS+=(-drive "if=pflash,format=raw,file=$OVMF_VARS")
else
  QEMU_ARGS+=(-bios "$OVMF_CODE")
fi

# FAT directory as virtual disk (ESP layout for EFI boot).
QEMU_ARGS+=(-hda "fat:rw:$ESP")

exec qemu-system-x86_64 "${QEMU_ARGS[@]}" "$@"
