#!/usr/bin/env bash
# Download SIL-OFL UI fonts required by nt10-kernel build.rs (no Microsoft fonts).
# Run from repo root: ./scripts/fetch-ofl-fonts.sh
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LATIN="$ROOT/third_party/fonts/latin"
mkdir -p "$LATIN"

# Noto Sans Regular — googlefonts/noto-fonts (OFL). Single static TTF for fontdue.
NOTO_URL="https://raw.githubusercontent.com/googlefonts/noto-fonts/main/hinted/ttf/NotoSans/NotoSans-Regular.ttf"
NOTO_DST="$LATIN/NotoSans-Regular.ttf"

if [[ -f "$NOTO_DST" ]]; then
  echo "Already present: $NOTO_DST"
  exit 0
fi

if command -v curl >/dev/null 2>&1; then
  curl -fsSL -o "$NOTO_DST" "$NOTO_URL"
elif command -v wget >/dev/null 2>&1; then
  wget -q -O "$NOTO_DST" "$NOTO_URL"
else
  echo "Need curl or wget to download OFL fonts." >&2
  exit 1
fi

echo "Wrote $NOTO_DST (OFL — see third_party/fonts/licenses/OFL-NotoSans.txt)"
