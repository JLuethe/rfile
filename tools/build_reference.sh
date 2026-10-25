#!/usr/bin/env bash
set -euo pipefail
repo_root=$(cd -- "$(dirname -- "$0")/.." && pwd)
reference_root=${RFILE_REFERENCE:-"$repo_root/upstream"}
reference_pin=41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59
if [[ ! -e "$reference_root/.git" ]]; then
  git clone https://github.com/file/file.git "$reference_root"
  git -C "$reference_root" checkout --detach "$reference_pin"
fi
if [[ $(git -C "$reference_root" rev-parse HEAD) != "$reference_pin" ]]; then
  echo "Expected upstream $reference_pin; existing checkout differs. No checkout was changed." >&2
  exit 1
fi
if [[ -n $(git -C "$reference_root" status --porcelain --untracked-files=no) ]]; then
  echo "Reference has tracked modifications. Preserve it and set RFILE_REFERENCE to a clean pinned checkout." >&2
  exit 1
fi
cd "$reference_root"
autoreconf -fiv
./configure --prefix="$repo_root/.reference"
make -j4
make check
