#!/usr/bin/env bash
# Installs the rtk-oss binary (Apache-2.0 edition: hook + filters only) into ~/.local/bin.
# Pinned version, checksum-verified. Linux (x86_64, aarch64), macOS (x86_64, arm64), WSL.
set -euo pipefail
RTK_VERSION="${RTK_VERSION:-1.0.1}"
BIN="${RTK_BIN_DIR:-$HOME/.local/bin}"

if command -v rtk >/dev/null 2>&1 && rtk --version 2>/dev/null | grep -q "$RTK_VERSION"; then
  echo "rtk $RTK_VERSION already installed: $(command -v rtk)"
else
  case "$(uname -s)-$(uname -m)" in
    Linux-x86_64)               target=x86_64-unknown-linux-musl ;;
    Linux-aarch64|Linux-arm64)  target=aarch64-unknown-linux-gnu ;;
    Darwin-arm64)               target=aarch64-apple-darwin ;;
    Darwin-x86_64)              target=x86_64-apple-darwin ;;
    *) echo "Unsupported platform $(uname -s)-$(uname -m). On Windows, use WSL." >&2; exit 1 ;;
  esac
  f="rtk-oss-$target.tar.gz"
  base="https://github.com/rtk-ai/rtk/releases/download/v$RTK_VERSION"
  tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
  curl -fsSL -o "$tmp/$f" "$base/$f"
  curl -fsSL -o "$tmp/checksums.txt" "$base/checksums.txt"
  want=$(awk -v f="$f" '$2==f || $2=="*"f {print $1}' "$tmp/checksums.txt")
  if command -v sha256sum >/dev/null; then got=$(sha256sum "$tmp/$f" | awk '{print $1}')
  else got=$(shasum -a 256 "$tmp/$f" | awk '{print $1}'); fi
  [ -n "$want" ] && [ "$want" = "$got" ] || { echo "Checksum mismatch for $f" >&2; exit 1; }
  tar xzf "$tmp/$f" -C "$tmp"
  mkdir -p "$BIN"
  install -m755 "$(find "$tmp" -type f -name rtk | head -1)" "$BIN/rtk"
  echo "installed $("$BIN/rtk" --version) to $BIN/rtk"
fi

case ":$PATH:" in *":$BIN:"*) ;; *) echo "NOTE: add $BIN to your PATH (e.g. in ~/.profile or ~/.zshrc)." ;; esac
echo "Next: rtk init --global --auto-patch"
