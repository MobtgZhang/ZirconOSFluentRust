#!/usr/bin/env bash
# Optional: shallow-clone the full kiwi0fruit/open-fonts catalog (~825MB on disk).
# The kernel build uses a small curated subset under third_party/fonts/{latin,cjk,kai}/.
# Upstream: https://github.com/kiwi0fruit/open-fonts
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEST="${REPO_ROOT}/third_party/fonts/open-fonts-full"
if [[ -d "${DEST}/.git" ]]; then
  git -C "${DEST}" fetch --depth 1 origin master
  git -C "${DEST}" reset --hard "origin/master"
else
  mkdir -p "$(dirname "${DEST}")"
  git clone --depth 1 https://github.com/kiwi0fruit/open-fonts.git "${DEST}"
fi
echo "Catalog ready: ${DEST}"
