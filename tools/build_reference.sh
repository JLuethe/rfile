#!/usr/bin/env bash
set -euo pipefail
repo_root=$(cd -- "$(dirname -- "$0")/.." && pwd)
reference_pin=41b1123083ab0bbdfe32f6d4ec14f6aec9a91e59
if [[ ! -d "$repo_root/upstream/.git" ]]; then
  git clone https://github.com/file/file.git "$repo_root/upstream"
  git -C "$repo_root/upstream" checkout --detach "$reference_pin"
fi
if [[ $(git -C "$repo_root/upstream" rev-parse HEAD) != "$reference_pin" ]]; then
  echo "Expected upstream $reference_pin; existing checkout differs. No checkout was changed." >&2
  exit 1
fi
cd "$repo_root/upstream"
autoreconf -fiv
./configure --prefix="$repo_root/.reference"
make -j4
make check
