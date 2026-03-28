#!/usr/bin/env bash
# Generate a path-only appendix from references/win32/desktop-src/toc.yml (Phase 00 optional).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TOC="$ROOT/references/win32/desktop-src/toc.yml"
OUT="${EXTENSIONS_INDEX_OUT:-$ROOT/extensions/REFERENCE-INDEX.auto.md}"

if [[ ! -f "$TOC" ]]; then
  echo "gen-extensions-index: missing $TOC" >&2
  exit 1
fi

{
  echo "# Reference index (auto-generated)"
  echo ""
  echo "Paths extracted from \`toc.yml\` \`href:\` entries ending in \`.md\` (local paths only)."
  echo "Regenerate: \`./scripts/gen-extensions-index.sh\`"
  echo ""
  grep -E '^\s*href:\s*\./.*\.md\s*$' "$TOC" | sed -E 's/^\s*href:\s*\.\//references\/win32\/desktop-src\//' | sort -u
} >"$OUT"

echo "Wrote $OUT"
