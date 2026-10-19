#!/usr/bin/env bash
# Links the global factory into ~/.claude so every project gets it. Called by bootstrap.sh.
set -euo pipefail
src="$(cd "$(dirname "$0")/../global" && pwd)"
for kind in agents skills; do
  mkdir -p "$HOME/.claude/$kind"
  for item in "$src/$kind"/*; do
    dest="$HOME/.claude/$kind/$(basename "$item")"
    if [ -e "$dest" ] && [ ! -L "$dest" ]; then
      echo "skip $dest (exists and is not our symlink)"; continue
    fi
    ln -sfn "$item" "$dest" && echo "linked $dest"
  done
done
